//! Application state struct, key-event handlers, and event loop.
//!
//! Types are defined in [`super::state`].
//! Key handlers live in [`super::key_handlers`], bindings in [`super::keymap`].
//! Mouse handlers live in [`super::mouse`].
//! Render functions live in [`super::render`].
//! Device actions and session events live in [`super::session_link`].

use std::io;
use std::time::{Duration, Instant};

/// Lifetime of a transient status-bar message.
const STATUS_MESSAGE_LIFETIME: Duration = Duration::from_secs(3);

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::session::worker::{SessionCommand, SessionEvent, spawn_worker};
use crate::storage::{Config, PresetLibrary, Storage};

use super::action::GlobalAction;
use super::keymap::KeyMap;
use super::render::render;
use super::state::{
    ConflictState, DeviceInfo, DevicePickerState, DrumState, LooperState, OperationStatus, Overlay,
    Tab,
};

// ── App ───────────────────────────────────────────────────────────────────────

/// Central application state passed to every render function.
pub struct App {
    pub should_quit: bool,
    pub active_tab: Tab,
    /// Overlay shown on top of the current tab, if any.
    pub active_overlay: Option<Overlay>,
    /// Current device / operation status, drives the tab-bar icon.
    pub operation_status: OperationStatus,
    /// Row index selected in the preset list.
    pub preset_selected: usize,
    /// Active scene index: 0 = A, 1 = B, 2 = C.
    pub active_scene: u8,
    /// Focused slot index in the bottom chain panel (0-based, max 10).
    pub chain_slot_focus: usize,
    /// Whether the bottom chain panel has keyboard focus.
    pub chain_panel_active: bool,
    /// Connected device info, `None` when offline.
    pub device_info: Option<DeviceInfo>,
    pub looper: LooperState,
    pub drum: DrumState,
    pub device_picker: DevicePickerState,
    /// Active conflict data — `Some` only while `Overlay::Conflict` is shown.
    pub conflict: Option<ConflictState>,
    /// Preset library loaded from XDG local storage.
    pub library: PresetLibrary,
    /// Monotonically increasing tick counter — used for spinner animation.
    pub tick: u64,
    /// Ephemeral status message shown in the status bar.
    /// The `Instant` is the wall-clock deadline at which the message expires;
    /// using `Instant` (not `self.tick + N`) avoids a wraparound bug where a
    /// message set near `u64::MAX` would never satisfy `tick < expires`.
    pub status_message: Option<(String, Instant)>,
    #[allow(dead_code)] // used in future phases for preferences/save
    pub config: Config,
    /// Key bindings: defaults merged with `keybindings.yaml`.
    pub keymap: KeyMap,
    /// `pub(crate)` so that the mouse handler module can call `save_library`.
    pub(crate) storage: Storage,
    /// Command channel to the session worker; `None` when no worker runs (tests).
    pub(crate) session_tx: Option<UnboundedSender<SessionCommand>>,
}

impl App {
    pub fn new() -> Self {
        let mut storage = Storage::new();
        let config = storage.load_config();
        storage.apply_config(&config);
        let library = match &config.last_device {
            Some(dev) => storage.load_library(dev),
            None => PresetLibrary::new("—"),
        };
        let keymap = storage
            .load_keybindings()
            .map_or_else(KeyMap::default, |yaml| KeyMap::from_yaml(&yaml));
        for warning in &keymap.warnings {
            tracing::warn!("keybindings.yaml: {warning}");
        }
        let mut app = Self {
            should_quit: false,
            active_tab: Tab::default(),
            active_overlay: None,
            operation_status: OperationStatus::default(),
            preset_selected: 0,
            active_scene: 0,
            chain_slot_focus: 0,
            chain_panel_active: false,
            device_info: None,
            looper: LooperState::new(),
            drum: DrumState::default(),
            device_picker: DevicePickerState::default(),
            conflict: None,
            library,
            tick: 0,
            status_message: None,
            config,
            keymap,
            storage,
            session_tx: None,
        };
        if let Some(first) = app.keymap.warnings.first() {
            let message = format!(
                "keybindings.yaml: {first} ({} issue(s), see log)",
                app.keymap.warnings.len()
            );
            app.set_status(&message);
        }
        app
    }

    pub fn scene_label(&self) -> &'static str {
        match self.active_scene {
            0 => "A",
            1 => "B",
            _ => "C",
        }
    }

    pub fn preset_count(&self) -> usize {
        self.library.presets.len()
    }

    /// Open an overlay, running any setup needed (e.g. device scan for picker).
    pub fn open_overlay(&mut self, overlay: Overlay) {
        if overlay == Overlay::DevicePicker {
            let devices = crate::device::Device::detect();
            self.device_picker = DevicePickerState {
                selected: 0,
                devices: devices.into_iter().map(|d| d.model).collect(),
            };
        }
        self.active_overlay = Some(overlay);
    }

    // ── Action dispatch ───────────────────────────────────────────────────────

    /// Execute a logical action, decoupled from the physical key that triggered it.
    ///
    /// The match is exhaustive — see [`super::action::GlobalAction`]: every new
    /// variant must have its dispatch arm added here in the same commit.
    pub fn handle_action(&mut self, action: GlobalAction) {
        match action {
            GlobalAction::Quit => self.should_quit = true,
            GlobalAction::ChangeDevice => self.open_overlay(Overlay::DevicePicker),
            GlobalAction::Help => self.open_overlay(Overlay::Help),
            GlobalAction::GetPreset => self.action_get_from_device(),
            GlobalAction::SendPreset => self.action_send_to_device(),
        }
    }

    /// Set a transient status message that disappears after [`STATUS_MESSAGE_LIFETIME`].
    pub(super) fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_owned(), Instant::now() + STATUS_MESSAGE_LIFETIME));
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Set up the terminal, run the event loop, and restore the terminal on exit.
///
/// Must be called from inside a tokio runtime (e.g. via `block_in_place`): the
/// session worker is spawned on it. The worker stops when the TUI exits.
pub fn run_tui() -> io::Result<()> {
    let runtime = tokio::runtime::Handle::try_current().map_err(io::Error::other)?;
    let (commands, events) = spawn_worker(&runtime);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_event_loop(&mut terminal, commands, events);

    // Always restore terminal, even on error.
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    commands: UnboundedSender<SessionCommand>,
    mut events: UnboundedReceiver<SessionEvent>,
) -> io::Result<()> {
    let mut app = App::new();
    app.session_tx = Some(commands);
    let mut last_area = Rect::default();
    loop {
        // Apply worker results first so this frame shows them.
        while let Ok(event) = events.try_recv() {
            app.handle_session_event(event);
        }
        terminal.draw(|f| {
            last_area = f.area();
            render(f, &app);
        })?;
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => app.handle_key(key),
                Event::Mouse(mouse) => app.handle_mouse(mouse, last_area),
                _ => {}
            }
        }
        app.tick = app.tick.wrapping_add(1);
        if app.should_quit {
            return Ok(());
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    use super::*;

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_action_quit() {
        let mut app = App::new();
        assert!(!app.should_quit);
        app.handle_action(GlobalAction::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn test_action_help_opens_overlay() {
        let mut app = App::new();
        assert!(app.active_overlay.is_none());
        app.handle_action(GlobalAction::Help);
        assert_eq!(app.active_overlay, Some(Overlay::Help));
    }

    #[test]
    fn test_action_get_preset_offline_sets_status() {
        let mut app = App::new();
        app.operation_status = OperationStatus::Offline;
        assert!(app.status_message.is_none());
        app.handle_action(GlobalAction::GetPreset);
        assert!(app.status_message.is_some());
    }

    #[test]
    fn test_action_send_preset_offline_sets_status() {
        let mut app = App::new();
        app.operation_status = OperationStatus::Offline;
        assert!(app.status_message.is_none());
        app.handle_action(GlobalAction::SendPreset);
        assert!(app.status_message.is_some());
    }

    /// A freshly-set status must expire in the future — not immediately — and
    /// within a reasonable window of the configured lifetime. Guards against
    /// the previous tick-based expiry which could wrap and produce an already-
    /// expired timestamp near `u64::MAX`.
    #[test]
    fn test_set_status_expires_in_the_future() {
        let mut app = App::new();
        let before = Instant::now();
        app.set_status("hello");
        let (msg, expires) = app.status_message.as_ref().expect("status was set");
        assert_eq!(msg, "hello");
        assert!(
            *expires > before,
            "expiry must be after the moment set_status was called"
        );
        // Upper bound with a generous margin for test-runner scheduling jitter.
        assert!(
            *expires <= Instant::now() + STATUS_MESSAGE_LIFETIME + Duration::from_secs(1),
            "expiry must be within lifetime + jitter margin"
        );
    }

    #[test]
    fn test_handle_key_q_quits() {
        let mut app = App::new();
        app.keymap = KeyMap::default();
        app.handle_key(make_key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_handle_key_question_mark_opens_help() {
        let mut app = App::new();
        app.keymap = KeyMap::default();
        app.handle_key(make_key(KeyCode::Char('?')));
        assert_eq!(app.active_overlay, Some(Overlay::Help));
    }

    #[test]
    fn test_handle_key_esc_closes_help_overlay() {
        let mut app = App::new();
        app.keymap = KeyMap::default();
        app.active_overlay = Some(Overlay::Help);
        app.handle_key(make_key(KeyCode::Esc));
        assert!(app.active_overlay.is_none());
    }

    #[test]
    fn test_handle_key_f1_switches_tab() {
        let mut app = App::new();
        app.keymap = KeyMap::default();
        app.active_tab = Tab::About;
        app.handle_key(make_key(KeyCode::F(1)));
        assert_eq!(app.active_tab, Tab::Presets);
    }

    #[test]
    fn test_handle_key_f2_switches_tab() {
        let mut app = App::new();
        app.keymap = KeyMap::default();
        app.handle_key(make_key(KeyCode::F(2)));
        assert_eq!(app.active_tab, Tab::LiveBoard);
    }
}
