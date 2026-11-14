//! Top-level render dispatch: tab bar, content area, status bar, and overlays.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Tabs,
};

use super::action::{Action, Context};
use super::app::App;
use super::state::{
    ICON_OFFLINE, ICON_ONLINE, OperationStatus, Overlay, SPINNER_FRAMES, Tab, UI_ONLY_BANNER,
    device_label,
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
    let device_label = device_label(app.device_info.as_ref());
    // Width: icon(1) + space(1) + label.
    let right_width = 2 + device_label.len() as u16;
    let tab_width = area.width.saturating_sub(right_width);

    let [tabs_area, info_area] =
        Layout::horizontal([Constraint::Length(tab_width), Constraint::Min(0)]).areas(area);

    let tabs = Tabs::new(tab_titles(app))
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
    // While an overlay that does not reach the device is open, reserve the right
    // edge for the "UI only" banner. Messages and key hints use the rest.
    let banner = ui_only_overlay(app).then_some(UI_ONLY_BANNER);
    let banner_width = banner.map_or(0, |text| text.chars().count() as u16);
    let [left_area, banner_area] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(banner_width)]).areas(area);

    // Show transient message if still within its lifetime.
    if let Some((msg, expires)) = &app.status_message
        && std::time::Instant::now() < *expires
    {
        frame.render_widget(
            Line::from(Span::styled(
                format!(" {msg}"),
                Style::default().fg(Color::Yellow),
            )),
            left_area,
        );
    } else if let OperationStatus::Busy { label } = &app.operation_status {
        frame.render_widget(
            Line::from(Span::styled(
                format!(" {label}"),
                Style::default().fg(Color::Yellow),
            )),
            left_area,
        );
    } else {
        let hints = key_hints(app);
        frame.render_widget(
            Line::from(Span::styled(hints, Style::default().fg(Color::DarkGray))),
            left_area,
        );
    }

    if let Some(text) = banner {
        frame.render_widget(
            Line::from(Span::styled(
                text,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            banner_area,
        );
    }
}

/// Whether the active overlay is UI-only (its actions are not sent to the device).
fn ui_only_overlay(app: &App) -> bool {
    matches!(app.active_overlay, Some(Overlay::Looper | Overlay::Drum))
}

/// Tab titles with the (configurable) key that selects each tab.
fn tab_titles(app: &App) -> Vec<String> {
    [
        (Action::TabPresets, "Presets"),
        (Action::TabLiveBoard, "Live Board"),
        (Action::TabAbout, "About"),
    ]
    .iter()
    .map(|(action, title)| format!("[{}] {title}", app.keymap.primary(Context::Global, *action)))
    .collect()
}

/// Key-hint text for the left side of the status bar, built from the active bindings.
fn key_hints(app: &App) -> String {
    use Action::{
        ChangeDevice, Close, Confirm, Down, GetPreset, Left, NextScene, OpenDrum, OpenLooper, Quit,
        Right, SendPreset, Up,
    };
    let keymap = &app.keymap;
    if let Some(overlay) = app.active_overlay {
        return keymap.hints(&[(Context::of_overlay(overlay), &[Close], "cancel")]);
    }
    let quit = (Context::Global, &[Quit][..], "quit");
    match app.active_tab {
        Tab::Presets if app.chain_panel_active => keymap.hints(&[
            (Context::Presets, &[Left, Right], "slot"),
            (Context::Presets, &[NextScene], "scene"),
            (Context::Presets, &[Close], "back"),
            quit,
        ]),
        Tab::Presets => keymap.hints(&[
            (Context::Presets, &[Down, Up], "move"),
            (Context::Presets, &[Confirm], "chain"),
            (Context::Presets, &[GetPreset], "get"),
            (Context::Presets, &[SendPreset], "send"),
            (Context::Global, &[ChangeDevice], "device"),
            (Context::Global, &[OpenLooper], "looper"),
            (Context::Global, &[OpenDrum], "drums"),
            quit,
        ]),
        Tab::LiveBoard => keymap.hints(&[
            (Context::LiveBoard, &[Left, Right], "slot"),
            (Context::LiveBoard, &[NextScene], "scene"),
            (Context::Global, &[ChangeDevice], "device"),
            (Context::Global, &[OpenLooper], "looper"),
            (Context::Global, &[OpenDrum], "drums"),
            quit,
        ]),
        Tab::About => keymap.hints(&[quit]),
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::DeviceInfo;
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    /// Render the full app at 100×20 and return the status row (last line).
    fn status_row(app: &App) -> String {
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, app)).unwrap();
        let buf = terminal.backend().buffer();
        let last_row_y = buf.area().height - 1;
        (0..buf.area().width)
            .map(|x| buf[(x, last_row_y)].symbol())
            .collect()
    }

    #[test]
    fn ui_only_banner_is_labeled() {
        assert!(UI_ONLY_BANNER.contains("not sent to device"));
    }

    /// Device status and presets are real now: no banner on the main screens.
    #[test]
    fn no_banner_without_ui_only_overlay() {
        let app = App::new();
        assert!(!status_row(&app).contains("UI only"));
    }

    #[test]
    fn banner_shown_for_looper_and_drum_overlays() {
        for overlay in [Overlay::Looper, Overlay::Drum] {
            let mut app = App::new();
            app.active_overlay = Some(overlay);
            let row = status_row(&app);
            assert!(
                row.contains("UI only"),
                "{overlay:?}: status row missing banner: {row:?}"
            );
        }
    }

    #[test]
    fn hints_and_tab_titles_follow_the_keymap() {
        let mut app = App::new();
        app.keymap = crate::ui::keymap::KeyMap::from_yaml(
            "global:\n  quit: [\"x\"]\n  tab_about: [\"<F9>\"]\n",
        );
        assert!(key_hints(&app).contains("x:quit"), "{}", key_hints(&app));
        assert_eq!(tab_titles(&app)[2], "[F9] About");
        app.active_overlay = Some(Overlay::Help);
        assert_eq!(key_hints(&app), " Esc:cancel");
    }

    #[test]
    fn busy_label_replaces_key_hints() {
        let mut app = App::new();
        app.operation_status = OperationStatus::Busy {
            label: "Fetching preset…".to_owned(),
        };
        let row = status_row(&app);
        assert!(row.contains("Fetching preset"), "{row:?}");
        assert!(!row.contains("q:quit"), "{row:?}");
    }

    #[test]
    fn tab_bar_shows_connected_device() {
        let mut app = App::new();
        app.device_info = Some(DeviceInfo {
            model: "MG-30".to_owned(),
            firmware: "v602".to_owned(),
            fw_gen: None,
        });
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();
        let buf = terminal.backend().buffer();
        let first_row: String = (0..buf.area().width)
            .map(|x| buf[(x, 0)].symbol())
            .collect();
        assert!(first_row.contains("MG-30 v602"), "{first_row:?}");
    }
}
