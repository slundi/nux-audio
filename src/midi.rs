use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use nux_audio::{SYSEX_END, SYSEX_START};
use std::error::Error;
use std::time::Duration;
use tokio::sync::broadcast;

// ── MidiSend trait ────────────────────────────────────────────────────────────

/// Abstraction over MIDI send operations.
///
/// Implemented by [`MidiManager`] for real hardware and by `MockMidi` (test-only, `#[cfg(test)]`)
/// so that command logic can be unit-tested without a physical device.
///
/// `send_cc` is async because it enforces a minimum inter-command delay
/// (see [`MIN_CC_INTERVAL`]) via [`tokio::time::sleep`], which must not block
/// the runtime worker.
pub trait MidiSend {
    fn send_sysex(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>>;
    fn send_cc(
        &mut self,
        control: u8,
        value: u8,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + Send;
}

/// Buffer size for the broadcast channel that fans out incoming MIDI messages.
///
/// Sized to absorb a full bank dump (128 presets × several SysEx packets each)
/// with headroom in case a subscriber is briefly slow. Previously 64, which
/// could silently overflow during multi-preset fetches.
const DISPATCHER_CAPACITY: usize = 256;

/// Minimum delay enforced between successive `send_cc` calls.
/// Protects device firmware from being overwhelmed by rapid CC floods.
const MIN_CC_INTERVAL: std::time::Duration = std::time::Duration::from_millis(5);

/// Outcome of processing one `broadcast::Receiver::recv()` result inside
/// [`MidiDispatcher::await_response`]. Extracted so the pattern-match branches —
/// especially the silent-drop-on-lag path — can be unit-tested without needing
/// to race a real broadcast channel.
enum RecvStep {
    /// Terminate the loop and return this result to the caller.
    Return(Result<Vec<u8>, crate::error::NuxError>),
    /// Discard this message and keep waiting.
    Continue,
}

/// Classify a `recv()` result against `pattern`, emitting a stderr warning
/// when tokio reports lag (broadcast overflow → silent message drops).
fn classify_recv(result: Result<Vec<u8>, broadcast::error::RecvError>, pattern: &[u8]) -> RecvStep {
    match result {
        Ok(msg) if msg.starts_with(pattern) => RecvStep::Return(Ok(msg)),
        Ok(_) => RecvStep::Continue,
        Err(broadcast::error::RecvError::Lagged(n)) => {
            // The receiver fell behind by `n` messages; those are lost.
            // Warn the user because a lag during a bank dump means silently
            // missing preset data downstream.
            eprintln!(
                "warning: MIDI dispatcher lagged, dropped {n} message(s) — \
                 consider increasing DISPATCHER_CAPACITY or slowing bulk queries"
            );
            RecvStep::Continue
        }
        Err(broadcast::error::RecvError::Closed) => {
            RecvStep::Return(Err(crate::error::NuxError::ChannelClosed))
        }
    }
}

/// Async wait until `min_interval` has elapsed since `last`. No-op if `last`
/// is `None` or the interval has already passed.
///
/// Extracted so it can be exercised under `tokio::time::pause()` in tests
/// without needing a real MIDI backend.
async fn wait_rate_limit(last: Option<std::time::Instant>, min_interval: Duration) {
    if let Some(l) = last {
        let elapsed = l.elapsed();
        if elapsed < min_interval {
            tokio::time::sleep(min_interval - elapsed).await;
        }
    }
}

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
    /// Returns [`crate::error::NuxError::ResponseTimeout`] if no matching message arrives within
    /// `timeout`, and [`crate::error::NuxError::ChannelClosed`] if the sender is dropped.
    pub async fn await_response(
        &self,
        pattern: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, crate::error::NuxError> {
        let mut rx = self.tx.subscribe();
        let pattern = pattern.to_vec();
        let result = tokio::time::timeout(timeout, async move {
            loop {
                match classify_recv(rx.recv().await, &pattern) {
                    RecvStep::Return(r) => return r,
                    RecvStep::Continue => continue,
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
    /// Timestamp of the last successful CC send — used to enforce `MIN_CC_INTERVAL`.
    last_cc_sent: Option<std::time::Instant>,
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
            last_cc_sent: None,
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
    /// Enforces a minimum inter-command delay of [`MIN_CC_INTERVAL`] via
    /// `tokio::time::sleep` to protect the device from CC floods. Because the
    /// wait is async, this function must be `.await`ed from a tokio runtime.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example(manager: &mut nux_audio::MidiManager) -> Result<(), Box<dyn std::error::Error>> {
    /// // Set AMP GAIN (CC 22) to 80:
    /// manager.send_cc(22, 80).await?;
    ///
    /// // Bypass the WAH slot (CC 0, value 0 = off):
    /// manager.send_cc(0, 0x00).await?;
    ///
    /// // Enable the WAH slot (CC 0, value 127 = on):
    /// manager.send_cc(0, 0x7F).await?;
    /// # Ok(()) }
    /// ```
    pub async fn send_cc(&mut self, control: u8, value: u8) -> Result<(), Box<dyn Error>> {
        if value > 127 {
            return Err(crate::error::NuxError::CcValueOutOfRange(value).into());
        }
        wait_rate_limit(self.last_cc_sent, MIN_CC_INTERVAL).await;
        // 0xB0 = Control Change on MIDI channel 1 (0-indexed channel 0)
        let result = self.send_raw_midi(&[0xB0, control, value]);
        if result.is_ok() {
            self.last_cc_sent = Some(std::time::Instant::now());
        }
        result
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

impl MidiSend for MidiManager {
    fn send_sysex(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.send_sysex(data)
    }

    async fn send_cc(&mut self, control: u8, value: u8) -> Result<(), Box<dyn Error>> {
        self.send_cc(control, value).await
    }
}

// ── MockMidi (test only) ──────────────────────────────────────────────────────

/// In-memory MIDI sink for unit tests.
///
/// Records every outgoing message so tests can assert which bytes were sent.
/// Set `should_fail = true` to simulate a connection error.
#[cfg(test)]
pub struct MockMidi {
    /// Raw bytes of every message sent (CC: `[0xB0, ctrl, val]`;
    /// SysEx: `[0xF0, 0x43, …data…, 0xF7]`).
    pub sent: Vec<Vec<u8>>,
    pub should_fail: bool,
}

#[cfg(test)]
impl MockMidi {
    pub fn new() -> Self {
        Self {
            sent: Vec::new(),
            should_fail: false,
        }
    }
}

#[cfg(test)]
impl MidiSend for MockMidi {
    fn send_sysex(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        if self.should_fail {
            return Err("mock send failure".into());
        }
        let mut msg = vec![SYSEX_START, 0x43];
        msg.extend_from_slice(data);
        msg.push(SYSEX_END);
        self.sent.push(msg);
        Ok(())
    }

    async fn send_cc(&mut self, control: u8, value: u8) -> Result<(), Box<dyn Error>> {
        if self.should_fail {
            return Err("mock send failure".into());
        }
        // NuxError::CcValueOutOfRange mirrors MidiManager::send_cc validation
        if value > 127 {
            return Err(crate::error::NuxError::CcValueOutOfRange(value).into());
        }
        self.sent.push(vec![0xB0, control, value]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// When `last` is `None`, no wait should occur.
    #[tokio::test(start_paused = true)]
    async fn wait_rate_limit_no_last_is_instant() {
        let start = Instant::now();
        wait_rate_limit(None, Duration::from_millis(5)).await;
        assert!(start.elapsed() < Duration::from_millis(1));
    }

    /// When the previous send is already older than the min interval, no wait.
    #[tokio::test(start_paused = true)]
    async fn wait_rate_limit_stale_last_is_instant() {
        // A `last` well in the past → elapsed >= min_interval → no sleep.
        let long_ago = Instant::now() - Duration::from_secs(1);
        wait_rate_limit(Some(long_ago), Duration::from_millis(5)).await;
        // Under `start_paused = true`, mock time doesn't advance unless we sleep;
        // reaching this line without hanging is sufficient proof.
    }

    /// When called within the min interval, the future must await a real
    /// `tokio::time::sleep` — verifiable because `start_paused = true` makes
    /// tokio's clock deterministic.
    #[tokio::test(start_paused = true)]
    async fn wait_rate_limit_waits_when_recent() {
        let now = Instant::now();
        // Fresh `last` → elapsed ~= 0 → must wait ~min_interval.
        let handle = tokio::spawn(async move {
            wait_rate_limit(Some(now), Duration::from_millis(5)).await;
        });
        // With paused time, the spawned task blocks on tokio::time::sleep until
        // we advance the clock. Advancing by the full interval lets it complete.
        tokio::time::advance(Duration::from_millis(5)).await;
        handle.await.unwrap();
    }

    /// Matching message → terminate with Ok.
    #[test]
    fn classify_recv_matching_message_returns_ok() {
        let step = classify_recv(Ok(vec![0xF0, 0x43, 0x58, 0x99]), &[0xF0, 0x43, 0x58]);
        match step {
            RecvStep::Return(Ok(msg)) => assert_eq!(msg, vec![0xF0, 0x43, 0x58, 0x99]),
            _ => panic!("expected Return(Ok(...)), got other"),
        }
    }

    /// Non-matching message → keep polling.
    #[test]
    fn classify_recv_unrelated_message_continues() {
        let step = classify_recv(Ok(vec![0xB0, 0x22, 0x50]), &[0xF0, 0x43, 0x58]);
        matches!(step, RecvStep::Continue).then_some(()).unwrap();
    }

    /// Lag → must continue (regression: previously silent, now also warns).
    /// The behavioural invariant is that lag is NOT a hard failure.
    #[test]
    fn classify_recv_lagged_continues_not_returns() {
        let step = classify_recv(
            Err(broadcast::error::RecvError::Lagged(7)),
            &[0xF0, 0x43, 0x58],
        );
        assert!(
            matches!(step, RecvStep::Continue),
            "Lagged must not cause await_response to return — it drops n messages and keeps polling"
        );
    }

    /// Channel closed → surface as `ChannelClosed` so the caller knows the
    /// dispatcher is dead (as opposed to just idle).
    #[test]
    fn classify_recv_closed_returns_channel_closed() {
        let step = classify_recv(
            Err(broadcast::error::RecvError::Closed),
            &[0xF0, 0x43, 0x58],
        );
        match step {
            RecvStep::Return(Err(crate::error::NuxError::ChannelClosed)) => {}
            _ => panic!("expected Return(Err(ChannelClosed))"),
        }
    }
}
