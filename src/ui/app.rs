//! Application state, event loop, and top-level render dispatch.

use std::io;
use std::time::Duration;

use chrono::Utc;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Tabs,
};

use crate::storage::{Config, LibraryEntry, PresetLibrary, Storage};

use super::{
    overlay_conflict, overlay_device, overlay_drum, overlay_looper, tab_about, tab_liveboard,
    tab_presets,
};

// ── Status icons (Nerd Fonts) ─────────────────────────────────────────────────

/// Device offline / disconnected.
pub const ICON_OFFLINE: &str = "󰣱";
/// Device connected and idle.
pub const ICON_ONLINE: &str = "󰣲";
/// Braille spinner frames shown when an operation is in progress.
pub const SPINNER_FRAMES: [&str; 8] = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];

// ── Tab ───────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Presets = 0,
    LiveBoard = 1,
    About = 2,
}

// ── Overlay ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    DevicePicker,
    Looper,
    Drum,
    Conflict,
}

// ── Conflict state ────────────────────────────────────────────────────────────

/// State carried while the conflict overlay is visible.
pub struct ConflictState {
    /// The preset fetched from the device that would replace `existing`.
    pub incoming: LibraryEntry,
    /// The existing library entry that occupies the same slot.
    pub existing: LibraryEntry,
    /// Index of the existing entry in `library.presets`.
    pub existing_idx: usize,
}

// ── Operation status ──────────────────────────────────────────────────────────

#[derive(Clone, Default, PartialEq, Eq)]
pub enum OperationStatus {
    #[default]
    Offline,
    Online,
    /// A MIDI operation is in progress. `label` describes it (e.g. "Fetching presets…").
    #[allow(dead_code)]
    Busy {
        label: String,
    },
}

// ── Device info ───────────────────────────────────────────────────────────────

pub struct DeviceInfo {
    pub model: String,
    pub fw_version: String,
}

// ── Looper state ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct LooperState {
    pub is_recording: bool,
    pub is_playing: bool,
    /// Tempo in BPM (20–240).
    pub bpm: u16,
    /// Output volume (0–127).
    pub volume: u8,
}

impl LooperState {
    fn new() -> Self {
        Self {
            bpm: 120,
            volume: 80,
            ..Default::default()
        }
    }
}

// ── Drum machine state ────────────────────────────────────────────────────────

pub const DRUM_PATTERNS: &[&str] = &[
    "Rock 1",
    "Rock 2",
    "Blues Shuffle",
    "Latin",
    "Pop 1",
    "Pop 2",
    "Jazz",
    "Funk",
];

pub struct DrumState {
    pub pattern_index: usize,
    /// Tempo in BPM (20–240).
    pub bpm: u16,
    /// Output volume (0–127).
    pub volume: u8,
    pub is_playing: bool,
}

impl Default for DrumState {
    fn default() -> Self {
        Self {
            pattern_index: 0,
            bpm: 120,
            volume: 80,
            is_playing: false,
        }
    }
}

// ── Device picker state ───────────────────────────────────────────────────────

#[derive(Default)]
pub struct DevicePickerState {
    pub selected: usize,
    /// Model names of detected MIDI devices.
    pub devices: Vec<String>,
}

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
    storage: Storage,
}

impl App {
    pub fn new() -> Self {
        let storage = Storage::new();
        let config = storage.load_config();
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

    /// Set a transient status message that disappears after ~3 seconds (30 ticks).
    fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_owned(), self.tick + 30));
    }

    // ── Mouse handling ────────────────────────────────────────────────────────

    pub fn handle_mouse(&mut self, mouse: event::MouseEvent, frame_area: Rect) {
        let col = mouse.column;
        let row = mouse.row;
        let pos = Position { x: col, y: row };

        // Recompute the same top-level layout used in render().
        let [tab_row, content, _status] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(frame_area);

        // Overlays absorb all mouse events.
        if let Some(overlay) = self.active_overlay {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    self.handle_mouse_overlay_click(overlay, col, row, frame_area);
                }
                MouseEventKind::ScrollUp => self.handle_mouse_overlay_scroll(overlay, -1),
                MouseEventKind::ScrollDown => self.handle_mouse_overlay_scroll(overlay, 1),
                _ => {}
            }
            return;
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if tab_row.contains(pos) {
                    self.handle_mouse_tab_click(col, tab_row);
                } else if content.contains(pos) {
                    self.handle_mouse_content_click(col, row, content);
                }
            }
            MouseEventKind::ScrollUp if content.contains(pos) => {
                self.handle_mouse_content_scroll(-1);
            }
            MouseEventKind::ScrollDown if content.contains(pos) => {
                self.handle_mouse_content_scroll(1);
            }
            _ => {}
        }
    }

    fn handle_mouse_tab_click(&mut self, col: u16, tab_row: Rect) {
        // Mirror the right_width computation in render_tab_bar.
        let device_label_len = match &self.device_info {
            Some(info) => format!(" {} fw{}", info.model, info.fw_version).len(),
            None => " offline".len(),
        };
        let right_width = (2 + device_label_len) as u16;
        let tabs_width = tab_row.width.saturating_sub(right_width);

        if col < tab_row.x || col >= tab_row.x + tabs_width {
            return; // clicked the info area on the right
        }
        let offset = col - tab_row.x;

        // Tabs: "[F1] Presets"(12) + " "(1) + "[F2] Live Board"(15) + " "(1) + "[F3] About"(10)
        if offset < 13 {
            self.active_tab = Tab::Presets;
        } else if offset < 29 {
            self.active_tab = Tab::LiveBoard;
        } else {
            self.active_tab = Tab::About;
        }
    }

    fn handle_mouse_content_click(&mut self, col: u16, row: u16, content: Rect) {
        let pos = Position { x: col, y: row };
        match self.active_tab {
            Tab::Presets => {
                let [list_area, chain_area] =
                    Layout::vertical([Constraint::Percentage(65), Constraint::Percentage(35)])
                        .areas(content);

                if list_area.contains(pos) {
                    self.handle_mouse_preset_list_click(row, list_area);
                } else if chain_area.contains(pos) {
                    self.handle_mouse_chain_click(col, chain_area);
                }
            }
            Tab::LiveBoard => {
                let [scene_bar, grid_area] =
                    Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(content);

                if scene_bar.contains(pos) {
                    // Click scene bar → cycle scene.
                    self.active_scene = (self.active_scene + 1) % 3;
                } else if grid_area.contains(pos) && grid_area.width > 0 {
                    // Click a slot column in the live board grid.
                    let slot_idx = ((col - grid_area.x) as usize * 11) / grid_area.width as usize;
                    self.chain_slot_focus = slot_idx.min(10);
                }
            }
            Tab::About => {}
        }
    }

    fn handle_mouse_content_scroll(&mut self, delta: i32) {
        if self.active_tab == Tab::Presets && !self.chain_panel_active {
            if delta < 0 {
                self.preset_selected = self.preset_selected.saturating_sub(1);
            } else {
                let max = self.preset_count().saturating_sub(1);
                self.preset_selected = (self.preset_selected + 1).min(max);
            }
        }
    }

    fn handle_mouse_preset_list_click(&mut self, row: u16, list_area: Rect) {
        // Inner area starts after the top border (y+1) and the header row (+1 more).
        let first_data_y = list_area.y + 2;
        if row < first_data_y {
            return;
        }
        let clicked_idx = (row - first_data_y) as usize;
        if clicked_idx < self.library.presets.len() {
            self.preset_selected = clicked_idx;
            self.chain_panel_active = false;
        }
    }

    fn handle_mouse_chain_click(&mut self, col: u16, chain_area: Rect) {
        // Inner x starts after the left border.
        let inner_x = chain_area.x + 1;
        if col < inner_x {
            return;
        }
        // Chain line layout: [WAH]──[CMP]──... Each slot occupies 7 chars.
        // Slot n starts at inner_x + n*7.
        let slot_idx = ((col - inner_x) / 7) as usize;
        if slot_idx <= 10 {
            self.chain_slot_focus = slot_idx;
            self.chain_panel_active = true;
        }
    }

    fn handle_mouse_overlay_click(
        &mut self,
        overlay: Overlay,
        col: u16,
        row: u16,
        frame_area: Rect,
    ) {
        match overlay {
            Overlay::DevicePicker => {
                // overlay_device: WIDTH=44, HEIGHT=12; items start at popup.y+2 (border+blank).
                let popup = popup_rect(44, 12, frame_area);
                let first_item_y = popup.y + 2;
                let last_y = popup.y + popup.height - 1;
                if row < first_item_y || row >= last_y {
                    return;
                }
                let clicked = (row - first_item_y) as usize;
                if clicked < self.device_picker.devices.len() {
                    self.device_picker.selected = clicked;
                }
            }
            Overlay::Drum => {
                // overlay_drum: WIDTH=44, HEIGHT=16; VISIBLE=6 pattern rows from popup.y+1.
                let popup = popup_rect(44, 16, frame_area);
                let first_item_y = popup.y + 1;
                let visible = 6usize;
                if row < first_item_y || row >= first_item_y + visible as u16 {
                    return;
                }
                let clicked = (row - first_item_y) as usize;
                let start = self
                    .drum
                    .pattern_index
                    .saturating_sub(visible / 2)
                    .min(DRUM_PATTERNS.len().saturating_sub(visible));
                let item = start + clicked;
                if item < DRUM_PATTERNS.len() {
                    self.drum.pattern_index = item;
                }
            }
            Overlay::Conflict => {
                // Bottom border row contains [O]verride / [N]o / [R]ename+timestamp.
                // Divide the popup width into three equal click zones.
                let popup = popup_rect(52, 12, frame_area);
                if row == popup.y + popup.height - 1 {
                    let third = popup.width / 3;
                    if col < popup.x + third {
                        if let Some(conflict) = self.conflict.take() {
                            self.library.presets[conflict.existing_idx] = conflict.incoming;
                            let _ = self.storage.save_library(&self.library);
                            self.set_status("Preset overridden");
                        }
                        self.active_overlay = None;
                    } else if col < popup.x + 2 * third {
                        self.conflict = None;
                        self.active_overlay = None;
                        self.set_status("Pull cancelled");
                    } else {
                        if let Some(mut conflict) = self.conflict.take() {
                            let ts = conflict.incoming.modified.format("%Y%m%d-%H%M%S");
                            conflict.incoming.name = format!("{} ({})", conflict.incoming.name, ts);
                            conflict.incoming.slot = None;
                            self.library.presets.push(conflict.incoming);
                            let _ = self.storage.save_library(&self.library);
                            self.set_status("Preset saved with timestamp");
                        }
                        self.active_overlay = None;
                    }
                }
            }
            Overlay::Looper => {} // Transport buttons: keyboard preferred.
        }
    }

    fn handle_mouse_overlay_scroll(&mut self, overlay: Overlay, delta: i32) {
        match overlay {
            Overlay::DevicePicker => {
                if delta < 0 {
                    self.device_picker.selected = self.device_picker.selected.saturating_sub(1);
                } else {
                    let max = self.device_picker.devices.len().saturating_sub(1);
                    self.device_picker.selected = (self.device_picker.selected + 1).min(max);
                }
            }
            Overlay::Drum => {
                if delta < 0 {
                    self.drum.pattern_index = self.drum.pattern_index.saturating_sub(1);
                } else {
                    self.drum.pattern_index =
                        (self.drum.pattern_index + 1).min(DRUM_PATTERNS.len() - 1);
                }
            }
            Overlay::Looper => {
                // Scroll adjusts volume in the looper overlay.
                if delta < 0 {
                    self.looper.volume = (self.looper.volume + 1).min(127);
                } else {
                    self.looper.volume = self.looper.volume.saturating_sub(1);
                }
            }
            Overlay::Conflict => {}
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
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return the centered popup rect used by overlay renderers.
/// Mirrors the `centered_rect` helper in each overlay file.
fn popup_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
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

// ── Top-level render ──────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, app: &App) {
    let [tab_row, content, status_row] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_tab_bar(frame, tab_row, app);

    match app.active_tab {
        Tab::Presets => tab_presets::render(frame, content, app),
        Tab::LiveBoard => tab_liveboard::render(frame, content, app),
        Tab::About => tab_about::render(frame, content, app),
    }

    render_status_bar(frame, status_row, app);

    // Overlays rendered last so they appear on top.
    if let Some(overlay) = app.active_overlay {
        match overlay {
            Overlay::DevicePicker => overlay_device::render(frame, frame.area(), app),
            Overlay::Looper => overlay_looper::render(frame, frame.area(), app),
            Overlay::Drum => overlay_drum::render(frame, frame.area(), app),
            Overlay::Conflict => overlay_conflict::render(frame, frame.area(), app),
        }
    }
}

fn render_tab_bar(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    // Status icon left of device label.
    let (icon, icon_color) = match &app.operation_status {
        OperationStatus::Offline => (ICON_OFFLINE, Color::Red),
        OperationStatus::Online => (ICON_ONLINE, Color::Green),
        OperationStatus::Busy { .. } => (
            SPINNER_FRAMES[app.tick as usize % SPINNER_FRAMES.len()],
            Color::Yellow,
        ),
    };
    let device_label = match &app.device_info {
        Some(info) => format!(" {} fw{}", info.model, info.fw_version),
        None => " offline".to_owned(),
    };
    // Width: icon(1) + space(1) + label.
    let right_width = 2 + device_label.len() as u16;
    let tab_width = area.width.saturating_sub(right_width);

    let [tabs_area, info_area] =
        Layout::horizontal([Constraint::Length(tab_width), Constraint::Min(0)]).areas(area);

    let tabs = Tabs::new(vec!["[F1] Presets", "[F2] Live Board", "[F3] About"])
        .select(app.active_tab as usize)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::raw(" "));
    frame.render_widget(tabs, tabs_area);

    let info = Line::from(vec![
        Span::styled(icon, Style::default().fg(icon_color)),
        Span::styled(device_label, Style::default().fg(Color::Cyan)),
    ])
    .right_aligned();
    frame.render_widget(info, info_area);
}

fn render_status_bar(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    // Show transient message if still within its lifetime.
    if let Some((msg, expires)) = &app.status_message
        && app.tick < *expires
    {
        frame.render_widget(
            Line::from(Span::styled(
                format!(" {msg}"),
                Style::default().fg(Color::Yellow),
            )),
            area,
        );
        return;
    }

    let hints = if app.active_overlay.is_some() {
        " Esc:cancel"
    } else {
        match app.active_tab {
            Tab::Presets => {
                if app.chain_panel_active {
                    " ←/→:slot  Tab:scene  Esc:back  q:quit"
                } else {
                    " j/k:move  Enter:chain  g:get  s:send  d:device  l:looper  D:drums  q:quit"
                }
            }
            Tab::LiveBoard => " ←/→:slot  Tab:scene  d:device  l:looper  D:drums  q:quit",
            Tab::About => " q:quit",
        }
    };
    frame.render_widget(
        Line::from(Span::styled(hints, Style::default().fg(Color::DarkGray))),
        area,
    );
}
