//! Help overlay — key bindings reference.
//!
//! Opened with `?`. Closed with `Esc`.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::app::App;

const WIDTH: u16 = 54;
const HEIGHT: u16 = 24;

pub fn render(frame: &mut Frame, area: Rect, _app: &App) {
    let popup = centered_rect(WIDTH, HEIGHT, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Help ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::Yellow))
        .title_bottom(
            Line::from(Span::styled(
                " Esc:close ",
                Style::default().fg(Color::DarkGray),
            ))
            .centered(),
        );

    let key = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let hdr = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::UNDERLINED);
    let dim = Style::default().fg(Color::White);
    let gap = Line::from("");

    let lines = vec![
        Line::from(Span::styled("Global", hdr)),
        Line::from(vec![
            Span::styled("  F1/F2/F3   ", key),
            Span::styled("Switch tabs (Presets / Live Board / About)", dim),
        ]),
        Line::from(vec![
            Span::styled("  d          ", key),
            Span::styled("Open device picker", dim),
        ]),
        Line::from(vec![
            Span::styled("  l          ", key),
            Span::styled("Open looper overlay", dim),
        ]),
        Line::from(vec![
            Span::styled("  D          ", key),
            Span::styled("Open drum machine overlay", dim),
        ]),
        Line::from(vec![
            Span::styled("  q          ", key),
            Span::styled("Quit", dim),
        ]),
        Line::from(vec![
            Span::styled("  ?          ", key),
            Span::styled("This help", dim),
        ]),
        gap.clone(),
        Line::from(Span::styled("Presets tab", hdr)),
        Line::from(vec![
            Span::styled("  j/k  ↑/↓  ", key),
            Span::styled("Move preset selection", dim),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", key),
            Span::styled("Toggle chain panel focus", dim),
        ]),
        Line::from(vec![
            Span::styled("  Tab        ", key),
            Span::styled("Cycle scene (A → B → C → A)", dim),
        ]),
        Line::from(vec![
            Span::styled("  g          ", key),
            Span::styled("Pull current preset from device", dim),
        ]),
        Line::from(vec![
            Span::styled("  s          ", key),
            Span::styled("Send selected preset to device", dim),
        ]),
        gap.clone(),
        Line::from(Span::styled("Live Board tab", hdr)),
        Line::from(vec![
            Span::styled("  h/l  ←/→  ", key),
            Span::styled("Move chain slot focus", dim),
        ]),
        Line::from(vec![
            Span::styled("  Tab        ", key),
            Span::styled("Cycle scene (A → B → C → A)", dim),
        ]),
        gap.clone(),
        Line::from(Span::styled("Overlays", hdr)),
        Line::from(vec![
            Span::styled("  Esc        ", key),
            Span::styled("Close overlay", dim),
        ]),
        Line::from(vec![
            Span::styled("  o / n / r  ", key),
            Span::styled("Conflict: Override / No / Rename", dim),
        ]),
    ];

    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    frame.render_widget(Paragraph::new(lines), inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}
