//! About tab: project information and full key-bindings reference.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use super::app::App;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO: &str = env!("CARGO_PKG_REPOSITORY");

pub fn render(frame: &mut Frame, area: Rect, _app: &App) {
    let [info_area, keys_area] =
        Layout::vertical([Constraint::Length(6), Constraint::Min(0)]).areas(area);

    render_project_info(frame, info_area);
    render_keybindings(frame, keys_area);
}

fn render_project_info(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" About ")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::styled(
                "nux-audio ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("v{VERSION}"), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![Span::raw(
            "CLI/TUI tool for NUX guitar multi-effects (MG-30, Mighty series)",
        )]),
        Line::from(vec![
            Span::styled("Repo: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                REPO,
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_keybindings(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Key Bindings ")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let header = Row::new(vec![
        Cell::from("Key").style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        ),
        Cell::from("Context").style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        ),
        Cell::from("Action").style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        ),
    ]);

    let bindings: &[(&str, &str, &str)] = &[
        (
            "F1 / F2 / F3",
            "Global",
            "Switch tab (Presets / Live Board / About)",
        ),
        ("q", "Global", "Quit"),
        ("j / k  or  ↓ / ↑", "Preset list", "Move selection"),
        ("Enter", "Preset list", "Focus / unfocus chain panel"),
        ("Esc", "Chain panel", "Return to preset list"),
        ("← / →  or  h / l", "Chain panel", "Navigate effect slots"),
        ("Tab", "Presets / Live Board", "Cycle scene  A → B → C"),
        ("d", "Presets", "Device picker"),
        ("g", "Presets", "Pull preset from device → local"),
        ("s", "Presets", "Send local preset → device"),
        ("e", "Presets", "Export preset to file"),
        ("/", "Presets", "Search / filter preset list"),
        ("m", "Chain panel", "Change model for focused slot"),
        ("r", "Chain panel", "Reset focused knob to default"),
        ("u", "Chain panel", "Undo last knob change"),
        ("l", "Global", "Looper overlay"),
        ("D", "Global", "Drum machine overlay"),
        ("?", "Global", "This help screen (About tab)"),
    ];

    let rows: Vec<Row> = bindings
        .iter()
        .map(|(key, ctx, action)| {
            Row::new(vec![
                Cell::from(*key).style(Style::default().fg(Color::Cyan)),
                Cell::from(*ctx).style(Style::default().fg(Color::DarkGray)),
                Cell::from(*action),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(22),
        Constraint::Length(20),
        Constraint::Min(30),
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default());
    frame.render_widget(table, inner);
}
