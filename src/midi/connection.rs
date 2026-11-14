//! [`MidiConnection`]: a connected [`MidiManager`] + [`MidiDispatcher`] pair — the
//! real-hardware implementation of [`MidiTransport`].

use super::dispatcher::{MidiDispatcher, wait_for};
use super::{MidiManager, MidiTransport};
use crate::error::MidiError;
use std::time::Duration;
use tokio::sync::broadcast;

/// An open bidirectional connection to one device.
pub struct MidiConnection {
    manager: MidiManager,
    dispatcher: MidiDispatcher,
}

impl MidiConnection {
    /// Initialize the MIDI backend and connect to `port_in` / `port_out`.
    pub fn open(port_in: usize, port_out: usize) -> Result<Self, MidiError> {
        let mut manager = MidiManager::new()?;
        let dispatcher = manager.connect_with_dispatcher(port_in, port_out)?;
        Ok(Self {
            manager,
            dispatcher,
        })
    }

    /// Subscribe to every incoming MIDI message (e.g. for live device updates).
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.dispatcher.subscribe()
    }
}

impl MidiTransport for MidiConnection {
    fn send_sysex(&mut self, data: &[u8]) -> Result<(), MidiError> {
        self.manager.send_sysex(data)
    }

    async fn send_cc(&mut self, control: u8, value: u8) -> Result<(), MidiError> {
        self.manager.send_cc(control, value).await
    }

    async fn request_sysex(
        &mut self,
        request: &[u8],
        response_header: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, MidiError> {
        // Subscribe first: a reply arriving between send and subscribe would be lost.
        let rx = self.dispatcher.subscribe();
        self.manager.send_sysex(request)?;
        wait_for(rx, response_header, timeout).await
    }
}
