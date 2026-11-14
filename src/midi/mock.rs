//! `MockMidi`: scripted in-memory [`MidiTransport`] for unit tests (test builds only).

use super::{MidiTransport, SYSEX_MANUFACTURER, frame_sysex};
use crate::error::MidiError;
use std::collections::VecDeque;
use std::time::Duration;

/// In-memory MIDI transport for unit tests.
///
/// Records every outgoing message so tests can assert which bytes were sent,
/// and answers [`MidiTransport::request_sysex`] from a FIFO of canned responses.
/// Set `should_fail = true` to simulate a connection error.
#[derive(Default)]
pub struct MockMidi {
    /// Raw bytes of every message sent (CC: `[0xB0, ctrl, val]`;
    /// SysEx: `[0xF0, 0x43, …data…, 0xF7]`).
    pub sent: Vec<Vec<u8>>,
    pub should_fail: bool,
    /// Incoming messages, consumed in order by `request_sysex`. Messages not
    /// matching the requested header are skipped, like the real dispatcher does.
    pub responses: VecDeque<Vec<u8>>,
}

impl MockMidi {
    pub fn new() -> Self {
        Self::default()
    }

    /// A mock that will answer requests with `responses`, in order.
    pub fn with_responses<I: IntoIterator<Item = Vec<u8>>>(responses: I) -> Self {
        Self {
            responses: responses.into_iter().collect(),
            ..Self::default()
        }
    }
}

impl MidiTransport for MockMidi {
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

    async fn request_sysex(
        &mut self,
        request: &[u8],
        response_header: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, MidiError> {
        self.send_sysex(request)?;
        while let Some(message) = self.responses.pop_front() {
            if message.starts_with(response_header) {
                return Ok(message);
            }
        }
        Err(MidiError::Timeout(timeout))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(matches!(
            mock.request_sysex(&[0x01], &[0xF0], Duration::from_secs(1))
                .await,
            Err(MidiError::Send(_))
        ));
        assert!(mock.sent.is_empty());
    }

    #[tokio::test]
    async fn mock_midi_request_skips_unrelated_then_times_out() {
        let mut mock = MockMidi::with_responses([vec![0xB0, 1, 2], vec![0xF0, 0x43, 0x58, 0x10]]);
        let reply = mock
            .request_sysex(
                &[0x58, 0x00],
                &[0xF0, 0x43, 0x58, 0x10],
                Duration::from_secs(1),
            )
            .await
            .unwrap();
        assert_eq!(reply, vec![0xF0, 0x43, 0x58, 0x10]);
        assert_eq!(mock.sent, vec![vec![0xF0, 0x43, 0x58, 0x00, 0xF7]]);

        assert!(matches!(
            mock.request_sysex(&[0x58, 0x00], &[0xF0], Duration::from_secs(1))
                .await,
            Err(MidiError::Timeout(_))
        ));
    }
}
