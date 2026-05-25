//! Application state, event loop, and top-level render dispatch.

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Tabs,
};

use crate::storage::{Config, PresetLibrary, Storage};

use super::{overlay_device, overlay_drum, overlay_looper, tab_about, tab_liveboard, tab_presets};

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
    /// Preset library loaded from XDG local storage.
    pub library: PresetLibrary,
    /// Monotonically increasing tick counter — used for spinner animation.
    pub tick: u64,
    #[allow(dead_code)] // used in future phases for preferences/save
    pub config: Config,
    #[allow(dead_code)] // used in future phases for save/load operations
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
            library,
            tick: 0,
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
    loop {
        terminal.draw(|f| render(f, &app))?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key);
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
    let hints = if app.active_overlay.is_some() {
        " Esc:close overlay"
    } else {
        match app.active_tab {
            Tab::Presets => {
                if app.chain_panel_active {
                    " ←/→:slot  Tab:scene  Esc:back  q:quit"
                } else {
                    " j/k:move  Enter:chain  d:device  l:looper  D:drums  q:quit"
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
