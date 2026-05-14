use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use nux_audio::{SYSEX_END, SYSEX_START};
use std::error::Error;
use std::time::Duration;
use tokio::sync::broadcast;

const DISPATCHER_CAPACITY: usize = 64;

/// Async MIDI response dispatcher backed by a `tokio::sync::broadcast` channel.
///
/// Bridges the synchronous midir callback into async tokio consumers.
/// Multiple tasks can subscribe independently; each receives every message.
pub struct MidiDispatcher {
    tx: broadcast::Sender<Vec<u8>>,
}

impl MidiDispatcher {
    fn new(tx: broadcast::Sender<Vec<u8>>) -> Self {
        Self { tx }
    }

    /// Subscribe to receive all incoming MIDI messages.
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.tx.subscribe()
    }

    /// Wait asynchronously for the first message whose bytes start with `pattern`.
    ///
    /// Returns [`NuxError::ResponseTimeout`] if no matching message arrives within
    /// `timeout`, and [`NuxError::ChannelClosed`] if the sender is dropped.
    pub async fn await_response(
        &self,
        pattern: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, crate::error::NuxError> {
        let mut rx = self.tx.subscribe();
        let pattern = pattern.to_vec();
        let result = tokio::time::timeout(timeout, async move {
            loop {
                match rx.recv().await {
                    Ok(msg) if msg.starts_with(&pattern) => return Ok(msg),
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => {
                        return Err(crate::error::NuxError::ChannelClosed);
                    }
                }
            }
        })
        .await;
        match result {
            Ok(inner) => inner,
            Err(_elapsed) => Err(crate::error::NuxError::ResponseTimeout),
        }
    }
}

/// MIDI connection manager for NUX devices
pub struct MidiManager {
    input: MidiInput,
    output: MidiOutput,
    connection_in: Option<MidiInputConnection<()>>,
    connection_out: Option<MidiOutputConnection>,
}

impl MidiManager {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let output = MidiOutput::new("NUX Audio MIDI Output")?;
        let input = MidiInput::new("NUX Audio MIDI Input")?;

        Ok(Self {
            input,
            output,
            connection_in: None,
            connection_out: None,
        })
    }

    /// List available MIDI output ports
    pub fn list_output_ports(&self) -> Vec<(usize, String)> {
        let ports = self.output.ports();
        ports
            .iter()
            .enumerate()
            .filter_map(|(i, port)| self.output.port_name(port).ok().map(|name| (i, name)))
            .collect()
    }

    /// List available MIDI input ports
    pub fn list_input_ports(&self) -> Vec<(usize, String)> {
        let ports = self.input.ports();
        ports
            .iter()
            .enumerate()
            .filter_map(|(i, port)| self.input.port_name(port).ok().map(|name| (i, name)))
            .collect()
    }

    /// Connect to MIDI ports and return a channel receiver for incoming messages.
    ///
    /// Every raw MIDI message received on `port_in` is forwarded to the returned
    /// `Receiver<Vec<u8>>`.  This is the preferred way to connect when the caller
    /// needs to read device responses (e.g. the version handshake).
    pub fn connect_with_channel(
        &mut self,
        port_in: usize,
        port_out: usize,
    ) -> Result<std::sync::mpsc::Receiver<Vec<u8>>, Box<dyn Error>> {
        let ports_out = self.output.ports();
        if port_out >= ports_out.len() {
            return Err("Output port index out of range".into());
        }
        let ports_in = self.input.ports();
        if port_in >= ports_in.len() {
            return Err("Input port index out of range".into());
        }

        let port = &ports_out[port_out];
        let port_name = self.output.port_name(port)?;
        let output = MidiOutput::new("NUX Audio MIDI Output")?;
        self.connection_out = Some(output.connect(port, &format!("nux-audio-{}", port_name))?);

        let port = &ports_in[port_in];
        let port_name = self.input.port_name(port)?;
        let input = MidiInput::new("NUX Audio MIDI Input")?;
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let connection_in = input.connect(
            port,
            &format!("nux-audio-{}", port_name),
            move |_stamp, message, _| {
                let _ = tx.send(message.to_vec());
            },
            (),
        )?;
        self.connection_in = Some(connection_in);

        Ok(rx)
    }

    /// Connect to MIDI ports and return a [`MidiDispatcher`] for async response routing.
    ///
    /// All incoming MIDI bytes are forwarded to a `tokio::sync::broadcast` channel.
    /// Callers use [`MidiDispatcher::await_response`] to wait for a specific message
    /// without blocking the async runtime.
    pub fn connect_with_dispatcher(
        &mut self,
        port_in: usize,
        port_out: usize,
    ) -> Result<MidiDispatcher, Box<dyn Error>> {
        let ports_out = self.output.ports();
        if port_out >= ports_out.len() {
            return Err("Output port index out of range".into());
        }
        let ports_in = self.input.ports();
        if port_in >= ports_in.len() {
            return Err("Input port index out of range".into());
        }

        let port = &ports_out[port_out];
        let port_name = self.output.port_name(port)?;
        let output = MidiOutput::new("NUX Audio MIDI Output")?;
        self.connection_out = Some(output.connect(port, &format!("nux-audio-{port_name}"))?);

        let port = &ports_in[port_in];
        let port_name = self.input.port_name(port)?;
        let input = MidiInput::new("NUX Audio MIDI Input")?;
        let (tx, _) = broadcast::channel::<Vec<u8>>(DISPATCHER_CAPACITY);
        let tx_clone = tx.clone();
        let connection_in = input.connect(
            port,
            &format!("nux-audio-{port_name}"),
            move |_stamp, message, _| {
                let _ = tx_clone.send(message.to_vec());
            },
            (),
        )?;
        self.connection_in = Some(connection_in);

        Ok(MidiDispatcher::new(tx))
    }

    /// Connect to a MIDI output port only (for send-only operations like CC messages).
    pub fn connect_output(&mut self, port_out: usize) -> Result<(), Box<dyn Error>> {
        let ports_out = self.output.ports();
        if port_out >= ports_out.len() {
            return Err("Output port index out of range".into());
        }
        let port = &ports_out[port_out];
        let port_name = self.output.port_name(port)?;
        let output = MidiOutput::new("NUX Audio MIDI Output")?;
        self.connection_out = Some(output.connect(port, &format!("nux-audio-{port_name}"))?);
        Ok(())
    }

    /// Send SysEx data to the connected device
    pub fn send_sysex(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut buffer = Vec::with_capacity(data.len() + 3);
        buffer.push(SYSEX_START);
        buffer.push(0x43);
        buffer.extend_from_slice(data);
        buffer.push(SYSEX_END);
        match &mut self.connection_out {
            Some(conn) => {
                conn.send(&buffer)?;
                Ok(())
            }
            None => Err("No MIDI connection established".into()),
        }
    }

    pub fn send_sysex_raw(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut buffer = Vec::with_capacity(data.len() + 2);
        buffer.push(SYSEX_START);
        buffer.extend_from_slice(data);
        buffer.push(SYSEX_END);
        match &mut self.connection_out {
            Some(conn) => {
                conn.send(&buffer)?;
                Ok(())
            }
            None => Err("No MIDI connection established".into()),
        }
    }

    /// Send a raw MIDI message directly to the connected output port.
    ///
    /// The caller is responsible for providing a valid MIDI message.
    /// Use [`send_sysex`](MidiManager::send_sysex) for SysEx and
    /// [`send_cc`](MidiManager::send_cc) for Control Change messages.
    pub fn send_raw_midi(&mut self, msg: &[u8]) -> Result<(), Box<dyn Error>> {
        match &mut self.connection_out {
            Some(conn) => conn.send(msg).map_err(Into::into),
            None => Err("No MIDI connection established".into()),
        }
    }

    /// Send a MIDI Control Change (CC) message on channel 1 (MIDI channel 0).
    ///
    /// Both `control` and `value` must be in the range 0–127 (MIDI 7-bit limit).
    /// Returns [`crate::error::NuxError::CcValueOutOfRange`] if `value > 127`.
    ///
    /// # Example
    /// ```no_run
    /// // Set AMP GAIN (CC 22) to 80:
    /// manager.send_cc(22, 80)?;
    ///
    /// // Bypass the WAH slot (CC 0, value 0 = off):
    /// manager.send_cc(0, 0x00)?;
    ///
    /// // Enable the WAH slot (CC 0, value 127 = on):
    /// manager.send_cc(0, 0x7F)?;
    /// ```
    pub fn send_cc(&mut self, control: u8, value: u8) -> Result<(), Box<dyn Error>> {
        if value > 127 {
            return Err(crate::error::NuxError::CcValueOutOfRange(value).into());
        }
        // 0xB0 = Control Change on MIDI channel 1 (0-indexed channel 0)
        self.send_raw_midi(&[0xB0, control, value])
    }

    /// Disconnect from the current MIDI device
    pub fn disconnect(&mut self) {
        self.connection_out = None;
        self.connection_in = None;
    }
}

impl Drop for MidiManager {
    fn drop(&mut self) {
        self.disconnect();
    }
}
