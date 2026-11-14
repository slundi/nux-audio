//! MIDI transport: the [`MidiSend`] abstraction, the midir-backed [`MidiManager`],
//! and the async [`MidiDispatcher`] that routes device responses to tokio tasks.

use crate::error::MidiError;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection, MidiOutputPort};
use nux_audio::{SYSEX_END, SYSEX_START};
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
    fn send_sysex(&mut self, data: &[u8]) -> Result<(), MidiError>;
    fn send_cc(
        &mut self,
        control: u8,
        value: u8,
    ) -> impl std::future::Future<Output = Result<(), MidiError>> + Send;
}

/// Reject a port `index` that is not below `available`.
fn check_port_index(
    direction: &'static str,
    index: usize,
    available: usize,
) -> Result<(), MidiError> {
    if index < available {
        Ok(())
    } else {
        Err(MidiError::PortOutOfRange {
            direction,
            index,
            available,
        })
    }
}

/// Map a midir connection failure to [`MidiError::Connect`], dropping the
/// consumed `MidiInput`/`MidiOutput` that midir hands back.
fn connect_error<T>(port: &str) -> impl FnOnce(midir::ConnectError<T>) -> MidiError + '_ {
    move |err| MidiError::Connect {
        port: port.to_owned(),
        kind: err.kind(),
    }
}

/// Frame `data` as a SysEx message: `F0 <prefix…> <data…> F7`.
fn frame_sysex(prefix: &[u8], data: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(prefix.len() + data.len() + 2);
    buffer.push(SYSEX_START);
    buffer.extend_from_slice(prefix);
    buffer.extend_from_slice(data);
    buffer.push(SYSEX_END);
    buffer
}

/// Manufacturer byte prepended by [`MidiManager::send_sysex`].
const SYSEX_MANUFACTURER: u8 = 0x43;

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
    pub fn new() -> Result<Self, MidiError> {
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
    ) -> Result<std::sync::mpsc::Receiver<Vec<u8>>, MidiError> {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        self.connect_both(port_in, port_out, move |message| {
            let _ = tx.send(message.to_vec());
        })?;
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
    ) -> Result<MidiDispatcher, MidiError> {
        let (tx, _) = broadcast::channel::<Vec<u8>>(DISPATCHER_CAPACITY);
        let tx_clone = tx.clone();
        self.connect_both(port_in, port_out, move |message| {
            let _ = tx_clone.send(message.to_vec());
        })?;
        Ok(MidiDispatcher::new(tx))
    }

    /// Connect to a MIDI output port only (for send-only operations like CC messages).
    pub fn connect_output(&mut self, port_out: usize) -> Result<(), MidiError> {
        let ports_out = self.output.ports();
        check_port_index("output", port_out, ports_out.len())?;
        self.connection_out = Some(self.open_output(&ports_out[port_out])?);
        Ok(())
    }

    /// Validate both port indices, then open output and input connections.
    ///
    /// `on_message` is invoked from the midir callback thread for every incoming
    /// message. Connections are only stored once both have opened successfully.
    fn connect_both<F>(
        &mut self,
        port_in: usize,
        port_out: usize,
        mut on_message: F,
    ) -> Result<(), MidiError>
    where
        F: FnMut(&[u8]) + Send + 'static,
    {
        let ports_out = self.output.ports();
        check_port_index("output", port_out, ports_out.len())?;
        let ports_in = self.input.ports();
        check_port_index("input", port_in, ports_in.len())?;

        let connection_out = self.open_output(&ports_out[port_out])?;

        let port = &ports_in[port_in];
        let port_name = self.input.port_name(port)?;
        let input = MidiInput::new("NUX Audio MIDI Input")?;
        let connection_in = input
            .connect(
                port,
                &format!("nux-audio-{port_name}"),
                move |_stamp, message, _| on_message(message),
                (),
            )
            .map_err(connect_error(&port_name))?;

        self.connection_out = Some(connection_out);
        self.connection_in = Some(connection_in);
        Ok(())
    }

    /// Open a connection to `port` on a fresh `MidiOutput` (midir consumes it on connect).
    fn open_output(&self, port: &MidiOutputPort) -> Result<MidiOutputConnection, MidiError> {
        let port_name = self.output.port_name(port)?;
        let output = MidiOutput::new("NUX Audio MIDI Output")?;
        output
            .connect(port, &format!("nux-audio-{port_name}"))
            .map_err(connect_error(&port_name))
    }

    /// Send SysEx data to the connected device
    pub fn send_sysex(&mut self, data: &[u8]) -> Result<(), MidiError> {
        self.send_raw_midi(&frame_sysex(&[SYSEX_MANUFACTURER], data))
    }

    /// Send `data` wrapped only in `F0 … F7` (no manufacturer byte added).
    pub fn send_sysex_raw(&mut self, data: &[u8]) -> Result<(), MidiError> {
        self.send_raw_midi(&frame_sysex(&[], data))
    }

    /// Send a raw MIDI message directly to the connected output port.
    ///
    /// The caller is responsible for providing a valid MIDI message.
    /// Use [`send_sysex`](MidiManager::send_sysex) for SysEx and
    /// [`send_cc`](MidiManager::send_cc) for Control Change messages.
    pub fn send_raw_midi(&mut self, msg: &[u8]) -> Result<(), MidiError> {
        match &mut self.connection_out {
            Some(conn) => conn.send(msg).map_err(Into::into),
            None => Err(MidiError::NotConnected),
        }
    }

    /// Send a MIDI Control Change (CC) message on channel 1 (MIDI channel 0).
    ///
    /// Both `control` and `value` must be in the range 0–127 (MIDI 7-bit limit).
    /// Returns [`MidiError::CcValueOutOfRange`] if `value > 127`.
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
    pub async fn send_cc(&mut self, control: u8, value: u8) -> Result<(), MidiError> {
        if value > 127 {
            return Err(MidiError::CcValueOutOfRange(value));
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
    fn send_sysex(&mut self, data: &[u8]) -> Result<(), MidiError> {
        self.send_sysex(data)
    }

    async fn send_cc(&mut self, control: u8, value: u8) -> Result<(), MidiError> {
        self.send_cc(control, value).await
    }
}

// ── MockMidi (test only) ──────────────────────────────────────────────────────

/// In-memory MIDI sink for unit tests.
///
/// Records every outgoing message so tests can assert which bytes were sent.
/// Set `should_fail = true` to simulate a connection error.
#[cfg(test)]
#[derive(Default)]
pub struct MockMidi {
    /// Raw bytes of every message sent (CC: `[0xB0, ctrl, val]`;
    /// SysEx: `[0xF0, 0x43, …data…, 0xF7]`).
    pub sent: Vec<Vec<u8>>,
    pub should_fail: bool,
}

#[cfg(test)]
impl MockMidi {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
impl MidiSend for MockMidi {
    fn send_sysex(&mut self, data: &[u8]) -> Result<(), MidiError> {
        if self.should_fail {
            return Err(midir::SendError::Other("mock send failure").into());
        }
        self.sent.push(frame_sysex(&[SYSEX_MANUFACTURER], data));
        Ok(())
    }

    async fn send_cc(&mut self, control: u8, value: u8) -> Result<(), MidiError> {
        if self.should_fail {
            return Err(midir::SendError::Other("mock send failure").into());
        }
        // Mirrors MidiManager::send_cc validation
        if value > 127 {
            return Err(MidiError::CcValueOutOfRange(value));
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

    #[test]
    fn check_port_index_bounds() {
        assert!(check_port_index("input", 0, 1).is_ok());
        assert!(matches!(
            check_port_index("output", 2, 2),
            Err(MidiError::PortOutOfRange {
                direction: "output",
                index: 2,
                available: 2
            })
        ));
        assert!(check_port_index("input", 0, 0).is_err());
    }

    #[test]
    fn frame_sysex_with_and_without_prefix() {
        assert_eq!(
            frame_sysex(&[SYSEX_MANUFACTURER], &[0x58, 0x01]),
            vec![0xF0, 0x43, 0x58, 0x01, 0xF7]
        );
        assert_eq!(
            frame_sysex(&[], &[0x43, 0x58]),
            vec![0xF0, 0x43, 0x58, 0xF7]
        );
        assert_eq!(frame_sysex(&[], &[]), vec![0xF0, 0xF7]);
    }

    #[test]
    fn connect_error_keeps_port_and_kind() {
        let err = connect_error("NUX MG-30")(midir::ConnectError::new(
            midir::ConnectErrorKind::InvalidPort,
            (),
        ));
        assert!(matches!(
            err,
            MidiError::Connect { ref port, kind: midir::ConnectErrorKind::InvalidPort }
                if port == "NUX MG-30"
        ));
    }

    #[tokio::test]
    async fn mock_midi_returns_typed_errors() {
        let mut mock = MockMidi::new();
        assert!(matches!(
            mock.send_cc(1, 200).await,
            Err(MidiError::CcValueOutOfRange(200))
        ));

        mock.should_fail = true;
        assert!(matches!(mock.send_sysex(&[0x01]), Err(MidiError::Send(_))));
        assert!(matches!(mock.send_cc(1, 1).await, Err(MidiError::Send(_))));
        assert!(mock.sent.is_empty());
    }

    /// Hardware-free checks on a real backend; skipped when MIDI can't initialize
    /// (e.g. CI containers without ALSA sequencer access).
    #[tokio::test]
    async fn manager_errors_without_connection() {
        let Ok(mut manager) = MidiManager::new() else {
            return;
        };
        assert!(matches!(
            manager.send_sysex(&[0x01]),
            Err(MidiError::NotConnected)
        ));
        assert!(matches!(
            manager.send_cc(1, 200).await,
            Err(MidiError::CcValueOutOfRange(200))
        ));
        assert!(matches!(
            manager.connect_output(usize::MAX),
            Err(MidiError::PortOutOfRange {
                direction: "output",
                ..
            })
        ));
        assert!(matches!(
            manager.connect_with_dispatcher(usize::MAX, usize::MAX),
            Err(MidiError::PortOutOfRange { .. })
        ));
    }
}
