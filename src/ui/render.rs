//! Top-level render dispatch: tab bar, content area, status bar, and overlays.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Tabs,
};

use super::app::App;
use super::state::{
    ICON_OFFLINE, ICON_ONLINE, MOCK_MIDI_BANNER, OperationStatus, Overlay, SPINNER_FRAMES, Tab,
};
use super::{
    overlay_conflict, overlay_device, overlay_drum, overlay_help, overlay_looper, tab_about,
    tab_liveboard, tab_presets,
};

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
            Overlay::Help => overlay_help::render(frame, frame.area(), app),
        }
    }
}

fn render_tab_bar(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
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
    // Reserve the right edge for the persistent "MOCK MIDI" banner so users are
    // never surprised by synthetic device state. Everything else (transient
    // messages, key hints) is rendered on the left half.
    let banner_width = MOCK_MIDI_BANNER.len() as u16;
    let [left_area, banner_area] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(banner_width)]).areas(area);

    // Show transient message if still within its lifetime.
    if let Some((msg, expires)) = &app.status_message
        && app.tick < *expires
    {
        frame.render_widget(
            Line::from(Span::styled(
                format!(" {msg}"),
                Style::default().fg(Color::Yellow),
            )),
            left_area,
        );
    } else {
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
            left_area,
        );
    }

    frame.render_widget(
        Line::from(Span::styled(
            MOCK_MIDI_BANNER,
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        banner_area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    /// The mock-midi banner constant must clearly say it's mock data.
    /// Anchors the invariant so a future refactor can't accidentally empty it.
    #[test]
    fn mock_midi_banner_is_labeled() {
        assert!(MOCK_MIDI_BANNER.contains("MOCK"));
        assert!(!MOCK_MIDI_BANNER.trim().is_empty());
    }

    /// End-to-end render assertion: the full app render must place the mock
    /// banner text somewhere in the status row.
    #[test]
    fn render_places_mock_banner_in_status_row() {
        let app = App::new();
        // Height 5 so the layout has: 1 tab_row + 3 content + 1 status_row.
        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();

        let buf = terminal.backend().buffer();
        // The status row is the last line of the buffer.
        let last_row_y = buf.area().height - 1;
        let mut row = String::new();
        for x in 0..buf.area().width {
            row.push_str(buf[(x, last_row_y)].symbol());
        }
        assert!(
            row.contains("MOCK"),
            "status row missing MOCK banner: {row:?}"
        );
    }
}
