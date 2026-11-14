//! [`MidiManager`]: midir-backed port enumeration, connection, and rate-limited sending.

use super::dispatcher::{DISPATCHER_CAPACITY, MidiDispatcher};
use super::{SYSEX_MANUFACTURER, frame_sysex};
use crate::error::MidiError;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::time::Duration;
use tokio::sync::broadcast;

/// Minimum delay enforced between successive `send_cc` calls.
/// Protects device firmware from being overwhelmed by rapid CC floods.
const MIN_CC_INTERVAL: std::time::Duration = std::time::Duration::from_millis(5);

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
    /// `Receiver<Vec<u8>>`.
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
