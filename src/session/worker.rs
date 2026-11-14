//! Session worker: an async task that owns a [`NuxSession`] and serves a
//! synchronous front-end (the TUI) over channels.
//!
//! The front-end sends [`SessionCommand`]s with `UnboundedSender::send` (non-blocking,
//! no `.await`) and drains [`SessionEvent`]s with `try_recv` once per frame, so device
//! round-trips (up to the 3 s response timeout) never freeze the UI. Commands are
//! processed one at a time, in order. Dropping the command sender stops the worker,
//! which drops the session and closes the MIDI ports.

use super::NuxSession;
use crate::error::SessionError;
use crate::midi::MidiTransport;
use nux_audio::effects::FirmwareGen;
use nux_audio::preset::Preset;
use std::future::Future;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

/// Request from the front-end to the worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCommand {
    /// Connect to `device` (model code or port-name fragment), replacing any open session.
    Connect { device: String },
    /// Fetch the preset currently loaded on the device.
    FetchCurrentPreset,
}

/// Which operation a [`SessionEvent::Failed`] refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOp {
    Connect,
    FetchPreset,
}

/// What the front-end needs to show about a connected device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSummary {
    /// Registry model code (e.g. `"MG-30"`).
    pub model: String,
    /// Firmware version string (e.g. `"v602"`).
    pub firmware: String,
    /// Hardware generation, when the family has several.
    pub fw_gen: Option<FirmwareGen>,
}

impl DeviceSummary {
    fn of<T: MidiTransport>(session: &NuxSession<T>) -> Self {
        let driver = session.driver();
        Self {
            model: session.device().model.clone(),
            firmware: driver.firmware().to_owned(),
            fw_gen: driver.firmware_gen(),
        }
    }
}

/// Outcome reported by the worker; exactly one per [`SessionCommand`].
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// The handshake succeeded.
    Connected(DeviceSummary),
    /// The current preset was fetched and decoded.
    PresetFetched(Preset),
    /// `op` failed; `message` is ready to display. A failed `Connect` leaves the
    /// worker disconnected; other failures keep the session open.
    Failed { op: SessionOp, message: String },
}

/// Serve `commands` until the sender is dropped (or the event receiver is gone).
///
/// `connect` opens a session for a device name; it is a parameter so tests can
/// connect through `MockMidi` instead of real ports.
pub async fn run_worker<T, C, Fut>(
    connect: C,
    mut commands: UnboundedReceiver<SessionCommand>,
    events: UnboundedSender<SessionEvent>,
) where
    T: MidiTransport,
    C: Fn(String) -> Fut,
    Fut: Future<Output = Result<NuxSession<T>, SessionError>>,
{
    let mut session: Option<NuxSession<T>> = None;
    while let Some(command) = commands.recv().await {
        let event = match command {
            SessionCommand::Connect { device } => {
                // Release the current ports before opening new ones.
                session = None;
                match connect(device).await {
                    Ok(opened) => {
                        let summary = DeviceSummary::of(&opened);
                        session = Some(opened);
                        SessionEvent::Connected(summary)
                    }
                    Err(e) => SessionEvent::Failed {
                        op: SessionOp::Connect,
                        message: e.to_string(),
                    },
                }
            }
            SessionCommand::FetchCurrentPreset => match session.as_mut() {
                None => SessionEvent::Failed {
                    op: SessionOp::FetchPreset,
                    message: "not connected".to_owned(),
                },
                Some(open) => match open.fetch_preset(None).await {
                    Ok(preset) => SessionEvent::PresetFetched(preset),
                    Err(e) => SessionEvent::Failed {
                        op: SessionOp::FetchPreset,
                        message: e.to_string(),
                    },
                },
            },
        };
        if events.send(event).is_err() {
            break; // front-end gone
        }
    }
}

/// Spawn a worker on `runtime` that connects to real MIDI hardware.
///
/// Returns the command sender and event receiver for the front-end.
pub fn spawn_worker(
    runtime: &tokio::runtime::Handle,
) -> (
    UnboundedSender<SessionCommand>,
    UnboundedReceiver<SessionEvent>,
) {
    let (command_tx, command_rx) = unbounded_channel();
    let (event_tx, event_rx) = unbounded_channel();
    runtime.spawn(run_worker(
        |device: String| async move { NuxSession::connect(Some(&device)).await },
        command_rx,
        event_tx,
    ));
    (command_tx, event_rx)
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::{basic_response, detail_response, device, version_response};
    use super::*;
    use crate::midi::MockMidi;

    /// Worker whose connector hands out MockMidi sessions answering `responses`
    /// (after the version reply). Returns the channels and the task handle.
    fn start(
        responses: Vec<Vec<u8>>,
    ) -> (
        UnboundedSender<SessionCommand>,
        UnboundedReceiver<SessionEvent>,
        tokio::task::JoinHandle<()>,
    ) {
        let (command_tx, command_rx) = unbounded_channel();
        let (event_tx, event_rx) = unbounded_channel();
        let connect = move |model: String| {
            let mut queue = vec![version_response("v602")];
            queue.extend(responses.clone());
            async move {
                NuxSession::handshake(MockMidi::with_responses(queue), device(&model, "NUX test"))
                    .await
            }
        };
        let handle = tokio::spawn(run_worker(connect, command_rx, event_tx));
        (command_tx, event_rx, handle)
    }

    fn connect(model: &str) -> SessionCommand {
        SessionCommand::Connect {
            device: model.to_owned(),
        }
    }

    #[tokio::test]
    async fn connect_reports_device_summary() {
        let (commands, mut events, _handle) = start(vec![]);
        commands.send(connect("MG-30")).unwrap();
        match events.recv().await.unwrap() {
            SessionEvent::Connected(summary) => assert_eq!(
                summary,
                DeviceSummary {
                    model: "MG-30".to_owned(),
                    firmware: "v602".to_owned(),
                    fw_gen: Some(FirmwareGen::Gen2),
                }
            ),
            other => panic!("expected Connected, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn connect_failure_is_reported_with_message() {
        let (commands, mut events, _handle) = start(vec![]);
        commands.send(connect("MP-3")).unwrap(); // no driver for MP-3
        match events.recv().await.unwrap() {
            SessionEvent::Failed {
                op: SessionOp::Connect,
                message,
            } => assert!(message.contains("not supported"), "{message}"),
            other => panic!("expected Failed(Connect), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn fetch_requires_a_connection() {
        let (commands, mut events, _handle) = start(vec![]);
        commands.send(SessionCommand::FetchCurrentPreset).unwrap();
        assert!(matches!(
            events.recv().await.unwrap(),
            SessionEvent::Failed {
                op: SessionOp::FetchPreset,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn fetch_after_connect_returns_preset_in_order() {
        let (commands, mut events, _handle) = start(vec![basic_response(9, 1), detail_response(9)]);
        // Both commands queued at once: processed in order, one event each.
        commands.send(connect("MG-30")).unwrap();
        commands.send(SessionCommand::FetchCurrentPreset).unwrap();
        assert!(matches!(
            events.recv().await.unwrap(),
            SessionEvent::Connected(_)
        ));
        match events.recv().await.unwrap() {
            SessionEvent::PresetFetched(preset) => {
                assert_eq!(preset.number, 9);
                assert_eq!(preset.active_scene_number, Some(1));
            }
            other => panic!("expected PresetFetched, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn fetch_failure_keeps_session_open() {
        // No preset responses queued → the basic query times out.
        let (commands, mut events, _handle) = start(vec![]);
        commands.send(connect("MG-30")).unwrap();
        commands.send(SessionCommand::FetchCurrentPreset).unwrap();
        events.recv().await.unwrap();
        match events.recv().await.unwrap() {
            SessionEvent::Failed {
                op: SessionOp::FetchPreset,
                message,
            } => assert!(message.contains("current preset query failed"), "{message}"),
            other => panic!("expected Failed(FetchPreset), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn dropping_the_sender_stops_the_worker() {
        let (commands, _events, handle) = start(vec![]);
        drop(commands);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn dropping_the_receiver_stops_the_worker() {
        let (commands, events, handle) = start(vec![]);
        drop(events);
        commands.send(connect("MG-30")).unwrap();
        handle.await.unwrap();
    }
}
