//! MIDI transport layer.
//!
//! - [`MidiTransport`]: the abstraction the session service (`crate::session`) talks to.
//! - [`MidiManager`]: midir-backed port enumeration, connection and sending.
//! - [`MidiDispatcher`]: fans incoming messages out to async tokio consumers.
//! - [`MidiConnection`]: a connected manager + dispatcher pair implementing [`MidiTransport`].
//! - `MockMidi` (tests only): scripted in-memory transport.

mod connection;
mod dispatcher;
mod manager;
#[cfg(test)]
mod mock;

pub use connection::MidiConnection;
pub use dispatcher::MidiDispatcher;
pub use manager::MidiManager;
#[cfg(test)]
pub use mock::MockMidi;

use crate::error::MidiError;
use nux_audio::{SYSEX_END, SYSEX_START};
use std::future::Future;
use std::time::Duration;

/// Abstraction over a connected MIDI device.
///
/// Implemented by [`MidiConnection`] for real hardware and by `MockMidi`
/// (test-only) so that device logic can be unit-tested without a physical device.
///
/// `send_cc` is async because it enforces a minimum inter-command delay
/// via [`tokio::time::sleep`], which must not block the runtime worker.
pub trait MidiTransport {
    /// Send `data` framed as `F0 43 <data…> F7`.
    fn send_sysex(&mut self, data: &[u8]) -> Result<(), MidiError>;

    /// Send a Control Change message on MIDI channel 1. Rejects `value > 127`.
    fn send_cc(
        &mut self,
        control: u8,
        value: u8,
    ) -> impl Future<Output = Result<(), MidiError>> + Send;

    /// Send `request` as SysEx (same framing as [`send_sysex`](Self::send_sysex)) and
    /// wait up to `timeout` for the first incoming message starting with `response_header`.
    ///
    /// Implementations must start listening *before* sending so a fast reply is not lost.
    fn request_sysex(
        &mut self,
        request: &[u8],
        response_header: &[u8],
        timeout: Duration,
    ) -> impl Future<Output = Result<Vec<u8>, MidiError>> + Send;
}

/// Manufacturer byte prepended by [`MidiTransport::send_sysex`].
const SYSEX_MANUFACTURER: u8 = 0x43;

/// Frame `data` as a SysEx message: `F0 <prefix…> <data…> F7`.
fn frame_sysex(prefix: &[u8], data: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(prefix.len() + data.len() + 2);
    buffer.push(SYSEX_START);
    buffer.extend_from_slice(prefix);
    buffer.extend_from_slice(data);
    buffer.push(SYSEX_END);
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
