//! Application state struct, key-event handlers, actions, and event loop.
//!
//! Types are defined in [`super::state`].
//! Mouse handlers live in [`super::mouse`].
//! Render functions live in [`super::render`].

use std::io;
use std::time::Duration;

use chrono::Utc;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};

use crate::storage::{Config, LibraryEntry, PresetLibrary, Storage};

use super::render::render;
use super::state::{
    ConflictState, DRUM_PATTERNS, DeviceInfo, DevicePickerState, DrumState, LooperState,
    OperationStatus, Overlay, Tab,
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
    /// The `u64` is the tick at which the message expires.
    pub status_message: Option<(String, u64)>,
    #[allow(dead_code)] // used in future phases for preferences/save
    pub config: Config,
    /// `pub(crate)` so that the mouse handler module can call `save_library`.
    pub(crate) storage: Storage,
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
        Self {
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
            storage,
        }
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

    // ── Key handling ──────────────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: event::KeyEvent) {
        // Overlay absorbs all keys when active.
        if let Some(overlay) = self.active_overlay {
            self.handle_overlay_key(overlay, key);
            return;
        }

        // Global bindings (no modifiers).
        if key.modifiers == KeyModifiers::NONE {
            match key.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                    return;
                }
                KeyCode::F(1) => {
                    self.active_tab = Tab::Presets;
                    return;
                }
                KeyCode::F(2) => {
                    self.active_tab = Tab::LiveBoard;
                    return;
                }
                KeyCode::F(3) => {
                    self.active_tab = Tab::About;
                    return;
                }
                KeyCode::Char('d') => {
                    self.open_overlay(Overlay::DevicePicker);
                    return;
                }
                KeyCode::Char('l') => {
                    self.open_overlay(Overlay::Looper);
                    return;
                }
                _ => {}
            }
            // 'D' is Shift+d — check separately.
        }
        if key.modifiers == KeyModifiers::SHIFT && key.code == KeyCode::Char('D') {
            self.open_overlay(Overlay::Drum);
            return;
        }

        match self.active_tab {
            Tab::Presets => self.handle_presets_key(key),
            Tab::LiveBoard => self.handle_liveboard_key(key),
            Tab::About => {}
        }
    }

    fn handle_overlay_key(&mut self, overlay: Overlay, key: event::KeyEvent) {
        // Esc always closes.
        if key.modifiers == KeyModifiers::NONE && key.code == KeyCode::Esc {
            self.active_overlay = None;
            return;
        }
        match overlay {
            Overlay::DevicePicker => self.handle_device_picker_key(key),
            Overlay::Looper => self.handle_looper_key(key),
            Overlay::Drum => self.handle_drum_key(key),
            Overlay::Conflict => self.handle_conflict_key(key),
        }
    }

    fn handle_device_picker_key(&mut self, key: event::KeyEvent) {
        if key.modifiers != KeyModifiers::NONE {
            return;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.device_picker.selected = self.device_picker.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.device_picker.devices.len().saturating_sub(1);
                self.device_picker.selected = (self.device_picker.selected + 1).min(max);
            }
            KeyCode::Enter => {
                if let Some(model) = self
                    .device_picker
                    .devices
                    .get(self.device_picker.selected)
                    .cloned()
                {
                    self.device_info = Some(DeviceInfo {
                        model: model.clone(),
                        fw_version: "?.?.?".into(),
                    });
                    self.operation_status = OperationStatus::Online;
                    self.library = self.storage.load_library(&model);
                    self.preset_selected = 0;
                    self.active_overlay = None;
                }
            }
            _ => {}
        }
    }

    fn handle_looper_key(&mut self, key: event::KeyEvent) {
        if key.modifiers != KeyModifiers::NONE {
            return;
        }
        match key.code {
            KeyCode::Char('r') => {
                self.looper.is_recording = !self.looper.is_recording;
                if self.looper.is_recording {
                    self.looper.is_playing = false;
                }
            }
            KeyCode::Char(' ') => {
                self.looper.is_playing = !self.looper.is_playing;
                if self.looper.is_playing {
                    self.looper.is_recording = false;
                }
            }
            KeyCode::Char('c') => {
                self.looper.is_playing = false;
                self.looper.is_recording = false;
            }
            KeyCode::Right | KeyCode::Char('+') => {
                self.looper.bpm = (self.looper.bpm + 1).min(240);
            }
            KeyCode::Left | KeyCode::Char('-') => {
                self.looper.bpm = self.looper.bpm.saturating_sub(1).max(20);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.looper.volume = (self.looper.volume + 1).min(127);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.looper.volume = self.looper.volume.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn handle_drum_key(&mut self, key: event::KeyEvent) {
        if key.modifiers != KeyModifiers::NONE {
            return;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.drum.pattern_index = self.drum.pattern_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.drum.pattern_index =
                    (self.drum.pattern_index + 1).min(DRUM_PATTERNS.len() - 1);
            }
            KeyCode::Right | KeyCode::Char('+') => {
                self.drum.bpm = (self.drum.bpm + 1).min(240);
            }
            KeyCode::Left | KeyCode::Char('-') => {
                self.drum.bpm = self.drum.bpm.saturating_sub(1).max(20);
            }
            KeyCode::Char(' ') => {
                self.drum.is_playing = !self.drum.is_playing;
            }
            _ => {}
        }
    }

    fn handle_presets_key(&mut self, key: event::KeyEvent) {
        if key.modifiers != KeyModifiers::NONE {
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.chain_panel_active = false;
            }
            KeyCode::Enter => {
                self.chain_panel_active = !self.chain_panel_active;
            }
            KeyCode::Tab => {
                self.active_scene = (self.active_scene + 1) % 3;
            }
            KeyCode::Char('g') if !self.chain_panel_active => {
                self.action_get_from_device();
            }
            KeyCode::Char('s') if !self.chain_panel_active => {
                self.action_send_to_device();
            }
            KeyCode::Up | KeyCode::Char('k') if !self.chain_panel_active => {
                self.preset_selected = self.preset_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') if !self.chain_panel_active => {
                let max = self.preset_count().saturating_sub(1);
                self.preset_selected = (self.preset_selected + 1).min(max);
            }
            KeyCode::Left | KeyCode::Char('h') if self.chain_panel_active => {
                self.chain_slot_focus = self.chain_slot_focus.saturating_sub(1);
            }
            KeyCode::Right | KeyCode::Char('l') if self.chain_panel_active => {
                self.chain_slot_focus = (self.chain_slot_focus + 1).min(10);
            }
            _ => {}
        }
    }

    fn handle_liveboard_key(&mut self, key: event::KeyEvent) {
        if key.modifiers != KeyModifiers::NONE {
            return;
        }
        match key.code {
            KeyCode::Tab => {
                self.active_scene = (self.active_scene + 1) % 3;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.chain_slot_focus = self.chain_slot_focus.saturating_sub(1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.chain_slot_focus = (self.chain_slot_focus + 1).min(10);
            }
            _ => {}
        }
    }

    fn handle_conflict_key(&mut self, key: event::KeyEvent) {
        if key.modifiers != KeyModifiers::NONE {
            return;
        }
        match key.code {
            // Override: replace the existing entry with the incoming one.
            KeyCode::Char('o') | KeyCode::Char('O') => {
                if let Some(conflict) = self.conflict.take() {
                    self.library.presets[conflict.existing_idx] = conflict.incoming;
                    let _ = self.storage.save_library(&self.library);
                    self.set_status("Preset overridden");
                }
                self.active_overlay = None;
            }
            // No: discard the incoming preset, keep the existing one.
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.conflict = None;
                self.active_overlay = None;
                self.set_status("Pull cancelled");
            }
            // Rename: save the incoming as a new local-only entry with a timestamp suffix.
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if let Some(mut conflict) = self.conflict.take() {
                    let ts = conflict.incoming.modified.format("%Y%m%d-%H%M%S");
                    conflict.incoming.name = format!("{} ({})", conflict.incoming.name, ts);
                    conflict.incoming.slot = None; // local copy — not tied to a slot
                    self.library.presets.push(conflict.incoming);
                    let _ = self.storage.save_library(&self.library);
                    self.set_status("Preset saved with timestamp");
                }
                self.active_overlay = None;
            }
            _ => {}
        }
    }

    // ── Device actions ────────────────────────────────────────────────────────

    /// Pull the current preset from the connected device into the local library.
    ///
    /// If a local entry already exists for the same device slot, the conflict
    /// overlay is opened to let the user choose [O]verride / [N]o / [R]ename.
    fn action_get_from_device(&mut self) {
        if self.operation_status != OperationStatus::Online {
            self.set_status("Not connected — press d to select a device");
            return;
        }

        let fw_gen = self
            .device_info
            .as_ref()
            .map(|d| d.fw_version.clone())
            .unwrap_or_else(|| "Gen2".to_owned());

        // Build a placeholder entry (real data would come via async MIDI fetch).
        let incoming = LibraryEntry {
            slot: Some(0),
            firmware_gen: fw_gen,
            modified: Utc::now(),
            name: "Device Preset".to_owned(),
            tempo_ms: 500,
            parallel: 0,
            active_scene: 0,
            effect_order: vec![
                "Wah".to_owned(),
                "Amp".to_owned(),
                "Delay".to_owned(),
                "Reverb".to_owned(),
            ],
            scenes: vec![],
            effects: vec![],
        };

        // Check for a slot collision.
        if let Some(idx) = self
            .library
            .presets
            .iter()
            .position(|p| p.slot == incoming.slot)
        {
            let existing = self.library.presets[idx].clone();
            self.conflict = Some(ConflictState {
                incoming,
                existing,
                existing_idx: idx,
            });
            self.active_overlay = Some(Overlay::Conflict);
        } else {
            self.library.presets.push(incoming);
            let _ = self.storage.save_library(&self.library);
            self.set_status("Preset pulled from device");
        }
    }

    /// Push the currently selected local preset to the connected device.
    fn action_send_to_device(&mut self) {
        if self.operation_status != OperationStatus::Online {
            self.set_status("Not connected — press d to select a device");
            return;
        }
        if self.library.presets.is_empty() {
            self.set_status("No preset selected");
            return;
        }

        let entry = &mut self.library.presets[self.preset_selected];
        // If local-only, assign it to slot 0 (first available); real
        // implementation would send CC/SysEx and read the assigned slot back.
        if entry.slot.is_none() {
            entry.slot = Some(0);
        }
        entry.modified = Utc::now();
        let _ = self.storage.save_library(&self.library);
        self.set_status("Preset sent to device");
    }

    /// Set a transient status message that disappears after ~3 seconds (30 ticks).
    pub(super) fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_owned(), self.tick + 30));
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Set up the terminal, run the event loop, and restore the terminal on exit.
pub fn run_tui() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_event_loop(&mut terminal);

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

fn run_event_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    let mut last_area = Rect::default();
    loop {
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
