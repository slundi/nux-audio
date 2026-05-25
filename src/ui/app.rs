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

use super::{tab_about, tab_liveboard, tab_presets};

// ── Tab ───────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Presets = 0,
    LiveBoard = 1,
    About = 2,
}

// ── DeviceInfo ────────────────────────────────────────────────────────────────

pub struct DeviceInfo {
    pub model: String,
    pub fw_version: String,
}

// ── App ───────────────────────────────────────────────────────────────────────

/// Central application state passed to every render function.
pub struct App {
    pub should_quit: bool,
    pub active_tab: Tab,
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
    /// Preset library loaded from XDG local storage.
    pub library: PresetLibrary,
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
            preset_selected: 0,
            active_scene: 0,
            chain_slot_focus: 0,
            chain_panel_active: false,
            device_info: None,
            library,
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

    // ── Key handling ──────────────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: event::KeyEvent) {
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
                _ => {}
            }
        }

        match self.active_tab {
            Tab::Presets => self.handle_presets_key(key),
            Tab::LiveBoard => self.handle_liveboard_key(key),
            Tab::About => {}
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
}

fn render_tab_bar(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let device_label = match &app.device_info {
        Some(info) => format!(" {} fw{}", info.model, info.fw_version),
        None => " offline".to_owned(),
    };

    // Split: tabs on left, device info on right.
    let tab_width = area.width.saturating_sub(device_label.len() as u16 + 2);
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

    let info = Line::from(vec![Span::styled(
        device_label,
        Style::default().fg(Color::Cyan),
    )])
    .right_aligned();
    frame.render_widget(info, info_area);
}

fn render_status_bar(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let hints = match app.active_tab {
        Tab::Presets => {
            if app.chain_panel_active {
                " ←/→:slot  Tab:scene  Esc:back  q:quit"
            } else {
                " j/k:move  Enter:chain  d:device  g:get  s:send  /:search  q:quit"
            }
        }
        Tab::LiveBoard => " j/k:slot  Tab:scene  q:quit",
        Tab::About => " q:quit",
    };
    let bar = Line::from(Span::styled(hints, Style::default().fg(Color::DarkGray)));
    frame.render_widget(bar, area);
}
