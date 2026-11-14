//! Bridge between the TUI and the session worker ([`crate::session::worker`]).
//!
//! Device actions (connect, get, send) turn into [`SessionCommand`]s sent without
//! blocking; [`App::handle_session_event`] applies the [`SessionEvent`]s the event
//! loop drains each frame. While a command is in flight the status is
//! [`OperationStatus::Busy`] and further device actions are refused.

use crate::export::PresetFile;
use crate::session::worker::{SessionCommand, SessionEvent, SessionOp};
use crate::storage::LibraryEntry;
use nux_audio::preset::Preset;

use super::app::App;
use super::state::{ConflictState, OperationStatus, Overlay};

const NOT_CONNECTED: &str = "Not connected — press d to select a device";

impl App {
    /// Connect to `model` through the worker (replacing any current connection).
    pub(super) fn connect_to(&mut self, model: String) {
        if self.refuse_if_busy() {
            return;
        }
        let label = format!("Connecting to {model}…");
        if self.send_command(SessionCommand::Connect { device: model }) {
            self.device_info = None;
            self.operation_status = OperationStatus::Busy { label };
        }
    }

    /// Pull the preset currently loaded on the device into the local library.
    ///
    /// The result arrives later as [`SessionEvent::PresetFetched`]; if a local entry
    /// already exists for that slot, the conflict overlay lets the user choose
    /// Override (`o`) / No (`n`) / Rename (`r`).
    pub(super) fn action_get_from_device(&mut self) {
        if self.refuse_if_busy() {
            return;
        }
        if self.operation_status != OperationStatus::Online {
            self.set_status(NOT_CONNECTED);
            return;
        }
        if self.send_command(SessionCommand::FetchCurrentPreset) {
            self.operation_status = OperationStatus::Busy {
                label: "Fetching preset…".to_owned(),
            };
        }
    }

    /// Push the selected local preset to the device.
    ///
    /// Not possible yet: the write-preset SysEx has not been reverse-engineered
    /// (the CLI's `preset send` refuses for the same reason). Say so instead of
    /// pretending, and leave the library untouched.
    pub(super) fn action_send_to_device(&mut self) {
        if self.refuse_if_busy() {
            return;
        }
        if self.operation_status != OperationStatus::Online {
            self.set_status(NOT_CONNECTED);
            return;
        }
        self.set_status("Sending presets is not supported yet (write-preset SysEx unknown)");
    }

    /// Apply one event from the session worker.
    pub fn handle_session_event(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::Connected(summary) => {
                self.library = self.storage.load_library(&summary.model);
                self.preset_selected = 0;
                self.set_status(&format!(
                    "Connected to {} ({})",
                    summary.model, summary.firmware
                ));
                self.device_info = Some(summary);
                self.operation_status = OperationStatus::Online;
            }
            SessionEvent::PresetFetched(preset) => {
                self.operation_status = OperationStatus::Online;
                self.receive_preset(&preset);
            }
            SessionEvent::Failed {
                op: SessionOp::Connect,
                message,
            } => {
                self.device_info = None;
                self.operation_status = OperationStatus::Offline;
                self.set_status(&format!("Connection failed: {message}"));
            }
            SessionEvent::Failed {
                op: SessionOp::FetchPreset,
                message,
            } => {
                self.operation_status = if self.device_info.is_some() {
                    OperationStatus::Online
                } else {
                    OperationStatus::Offline
                };
                self.set_status(&format!("Fetch failed: {message}"));
            }
        }
    }

    /// Store a fetched preset in the library, or open the conflict overlay when
    /// its slot is already taken.
    fn receive_preset(&mut self, preset: &Preset) {
        let Some(info) = &self.device_info else {
            return; // disconnected meanwhile; nothing to attribute the preset to
        };
        let file = PresetFile::from_preset(preset, &info.model);
        let captured_on = info
            .fw_gen
            .map_or_else(|| info.firmware.clone(), |fw_gen| fw_gen.to_string());
        let mut incoming = LibraryEntry::from_preset_file(&file, &captured_on);
        incoming.slot = Some(preset.number);

        if let Some(existing_idx) = self
            .library
            .presets
            .iter()
            .position(|entry| entry.slot == incoming.slot)
        {
            let existing = self.library.presets[existing_idx].clone();
            self.conflict = Some(ConflictState {
                incoming,
                existing,
                existing_idx,
            });
            self.active_overlay = Some(Overlay::Conflict);
        } else {
            self.library.presets.push(incoming);
            let _ = self.storage.save_library(&self.library);
            self.set_status(&format!(
                "Preset {} pulled from device",
                preset.get_preset_code()
            ));
        }
    }

    /// Send `command` to the worker; on failure report it and return `false`.
    fn send_command(&mut self, command: SessionCommand) -> bool {
        let sent = self
            .session_tx
            .as_ref()
            .is_some_and(|commands| commands.send(command).is_ok());
        if !sent {
            self.set_status("Device worker is not running");
        }
        sent
    }

    /// Report and return `true` while a device operation is in flight.
    fn refuse_if_busy(&mut self) -> bool {
        if let OperationStatus::Busy { label } = &self.operation_status {
            let message = format!("Busy: {label}");
            self.set_status(&message);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::worker::DeviceSummary;
    use crate::storage::Config;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use nux_audio::effects::FirmwareGen;
    use std::path::PathBuf;
    use tokio::sync::mpsc::{UnboundedReceiver, error::TryRecvError, unbounded_channel};

    /// App wired to a test command channel, with its library stored in a private
    /// temp dir so tests never touch the user's XDG data.
    struct Harness {
        app: App,
        commands: UnboundedReceiver<SessionCommand>,
        dir: PathBuf,
    }

    impl Harness {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("nux-tui-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let (command_tx, commands) = unbounded_channel();
            let mut app = App::new();
            app.storage.apply_config(&Config {
                presets_dir: Some(dir.clone()),
                ..Config::default()
            });
            app.session_tx = Some(command_tx);
            Self { app, commands, dir }
        }

        /// Harness already connected to an MG-30 (empty library in the temp dir).
        fn connected(name: &str) -> Self {
            let mut harness = Self::new(name);
            harness
                .app
                .handle_session_event(SessionEvent::Connected(mg30()));
            harness
        }

        fn status(&self) -> &str {
            self.app
                .status_message
                .as_ref()
                .map_or("", |(msg, _)| msg.as_str())
        }
    }

    impl Drop for Harness {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn mg30() -> DeviceSummary {
        DeviceSummary {
            model: "MG-30".to_owned(),
            firmware: "v602".to_owned(),
            fw_gen: Some(FirmwareGen::Gen2),
        }
    }

    fn preset(number: u8, name: &str) -> Preset {
        Preset {
            number,
            name: name.to_owned(),
            tempo: 500,
            ..Default::default()
        }
    }

    fn enter() -> KeyEvent {
        KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    // ── Commands ──────────────────────────────────────────────────────────────

    #[test]
    fn picker_enter_sends_connect_and_shows_busy() {
        let mut harness = Harness::new("picker");
        harness.app.device_picker.devices = vec!["MG-30".to_owned()];
        harness.app.active_overlay = Some(Overlay::DevicePicker);
        harness.app.handle_key(enter());

        assert_eq!(
            harness.commands.try_recv().unwrap(),
            SessionCommand::Connect {
                device: "MG-30".to_owned()
            }
        );
        assert!(harness.app.active_overlay.is_none());
        assert!(matches!(
            harness.app.operation_status,
            OperationStatus::Busy { ref label } if label.contains("MG-30")
        ));
    }

    #[test]
    fn get_when_online_requests_fetch() {
        let mut harness = Harness::connected("get-online");
        harness.app.action_get_from_device();
        assert_eq!(
            harness.commands.try_recv().unwrap(),
            SessionCommand::FetchCurrentPreset
        );
        assert!(matches!(
            harness.app.operation_status,
            OperationStatus::Busy { .. }
        ));
    }

    #[test]
    fn get_when_offline_sends_nothing() {
        let mut harness = Harness::new("get-offline");
        harness.app.action_get_from_device();
        assert_eq!(harness.commands.try_recv(), Err(TryRecvError::Empty));
        assert!(harness.status().contains("Not connected"));
    }

    #[test]
    fn actions_refused_while_busy() {
        let mut harness = Harness::connected("busy");
        harness.app.action_get_from_device();
        harness.commands.try_recv().unwrap();

        harness.app.action_get_from_device();
        harness.app.connect_to("MG-30".to_owned());
        assert_eq!(harness.commands.try_recv(), Err(TryRecvError::Empty));
        assert!(harness.status().starts_with("Busy:"));
    }

    #[test]
    fn send_reports_unsupported_and_keeps_library() {
        let mut harness = Harness::connected("send");
        harness
            .app
            .handle_session_event(SessionEvent::PresetFetched(preset(4, "Lead")));
        let before = harness.app.library.presets.clone();

        harness.app.action_send_to_device();
        assert!(harness.status().contains("not supported"));
        assert_eq!(harness.app.library.presets, before);
        assert_eq!(harness.commands.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn missing_worker_is_reported() {
        let mut harness = Harness::new("no-worker");
        harness.app.session_tx = None;
        harness.app.connect_to("MG-30".to_owned());
        assert_eq!(harness.app.operation_status, OperationStatus::Offline);
        assert!(harness.status().contains("not running"));
    }

    // ── Events ────────────────────────────────────────────────────────────────

    #[test]
    fn connected_event_goes_online_with_device_library() {
        let harness = Harness::connected("connected");
        assert_eq!(harness.app.operation_status, OperationStatus::Online);
        assert_eq!(harness.app.device_info, Some(mg30()));
        assert_eq!(harness.app.library.device, "MG-30");
        assert!(harness.status().contains("Connected to MG-30 (v602)"));
    }

    #[test]
    fn connect_failure_goes_offline_with_message() {
        let mut harness = Harness::new("connect-failed");
        harness.app.handle_session_event(SessionEvent::Failed {
            op: SessionOp::Connect,
            message: "no supported Nux device detected".to_owned(),
        });
        assert_eq!(harness.app.operation_status, OperationStatus::Offline);
        assert!(harness.app.device_info.is_none());
        assert!(harness.status().contains("no supported Nux device"));
    }

    #[test]
    fn fetch_failure_stays_online() {
        let mut harness = Harness::connected("fetch-failed");
        harness.app.handle_session_event(SessionEvent::Failed {
            op: SessionOp::FetchPreset,
            message: "current preset query failed".to_owned(),
        });
        assert_eq!(harness.app.operation_status, OperationStatus::Online);
        assert!(harness.status().starts_with("Fetch failed"));
    }

    #[test]
    fn fetched_preset_is_saved_with_its_slot() {
        let mut harness = Harness::connected("fetched");
        harness
            .app
            .handle_session_event(SessionEvent::PresetFetched(preset(5, "My Tone")));

        let entry = harness.app.library.presets.last().unwrap();
        assert_eq!(entry.slot, Some(5));
        assert_eq!(entry.name, "My Tone");
        assert_eq!(entry.tempo_ms, 500);
        assert_eq!(entry.firmware_gen, "Gen2");
        assert!(harness.status().contains("02B"));
        assert!(harness.app.storage.library_path("MG-30").exists());
    }

    #[test]
    fn fetched_preset_on_taken_slot_opens_conflict() {
        let mut harness = Harness::connected("conflict");
        harness
            .app
            .handle_session_event(SessionEvent::PresetFetched(preset(5, "Old")));
        harness
            .app
            .handle_session_event(SessionEvent::PresetFetched(preset(5, "New")));

        assert_eq!(harness.app.active_overlay, Some(Overlay::Conflict));
        let conflict = harness.app.conflict.as_ref().unwrap();
        assert_eq!(conflict.existing.name, "Old");
        assert_eq!(conflict.incoming.name, "New");
        assert_eq!(harness.app.library.presets.len(), 1);
    }
}
