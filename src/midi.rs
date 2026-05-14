use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use nux_audio::{SYSEX_END, SYSEX_START};
use std::error::Error;

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

    /// Connect to a MIDI port by index
    pub fn connect(&mut self, port_in: usize, port_out: usize) -> Result<(), Box<dyn Error>> {
        // check port range
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

        // Create a new output instance for the connection
        let output = MidiOutput::new("NUX Audio MIDI Output")?;
        let connection_out = output.connect(port, &format!("nux-audio-{}", port_name))?;
        self.connection_out = Some(connection_out);

        let port = &ports_in[port_in];
        let port_name = self.input.port_name(port)?;

        let input = MidiInput::new("NUX Audio MIDI Intput")?;
        // let connection_in = input.connect(port, &format!("nux-audio-{}", port_name), move |stamp, message, data| {
        //     println!("TODO: callback: {stamp}ms\n\tSent: {:?}\n\tResp: {:?}", message, data);
        // }, ())?;
        let connection_in = input.connect(
            port,
            &format!("nux-audio-{}", port_name),
            midi_input_callback,
            (),
        )?;

        self.connection_in = Some(connection_in);
        Ok(())
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

    // /// Send effect parameter change
    // pub fn send_effect_parameter(
    //     &mut self,
    //     device_type: DeviceType,
    //     effect_id: u8,
    //     param_id: u8,
    //     value: u8,
    // ) -> Result<(), Box<dyn Error>> {
    //     use crate::DeviceFactory;
    //     let device = DeviceFactory::create_device(device_type);

    //     let mut sysex = device.sysex_header();
    //     sysex.extend(&[
    //         0x07, // Parameter change command
    //         effect_id, param_id, value, 0xF7, // SysEx end
    //     ]);

    //     self.send_sysex(&sysex)
    // }

    // /// Send preset change command
    // pub fn send_preset_change(
    //     &mut self,
    //     device_type: DeviceType,
    //     preset_number: u8,
    // ) -> Result<(), Box<dyn Error>> {
    //     use crate::DeviceFactory;
    //     let device = DeviceFactory::create_device(device_type);

    //     let mut sysex = device.sysex_header();
    //     sysex.extend(&[
    //         0x01, // Preset change command
    //         preset_number,
    //         0xF7, // SysEx end
    //     ]);

    //     self.send_sysex(&sysex)
    // }

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

// --- Corrected Callback function for incoming MIDI messages ---
// The third parameter is `data: &mut D`, where D is the type of the user data passed to `connect`.
// Since we passed `()`, it will be `data: &mut ()`.
fn midi_input_callback(timestamp: u64, message: &[u8], _data: &mut ()) {
    println!(
        "[MIDI IN] Timestamp: {} - Message: {:?}",
        timestamp, message
    );
    if !message.is_empty() {
        match message[0] {
            0xF0 => {
                // System Exclusive (SysEx) message
                println!("  SysEx message received!");
            }
            0x90..=0x9F => println!("  Note On message received! Channel: {}", message[0] & 0x0F),
            0x80..=0x8F => println!(
                "  Note Off message received! Channel: {}",
                message[0] & 0x0F
            ),
            _ => {}
        }
    }
}
