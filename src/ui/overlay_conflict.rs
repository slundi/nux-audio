//! Preset conflict overlay.
//!
//! Shown when `g` (get from device) would overwrite a preset that already
//! exists in the local library.  The user chooses one of three actions:
//!
//! ```text
//! ╔══════════════════════════════════════════════════╗
//! ║  Preset Conflict                                 ║
//! ║  ────────────────────────────────────────────── ║
//! ║  A local preset already occupies this slot:     ║
//! ║                                                 ║
//! ║  "Crunch Lead"  (slot 001)                      ║
//! ║                                                 ║
//! ║  Incoming from device: "Device Preset"          ║
//! ║  ────────────────────────────────────────────── ║
//! ║        [O]verride   [N]o   [R]ename+timestamp   ║
//! ╚══════════════════════════════════════════════════╝
//! ```

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::action::{Action, Context};
use super::app::App;

const WIDTH: u16 = 52;
const HEIGHT: u16 = 12;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let Some(conflict) = &app.conflict else {
        return;
    };

    let popup = centered_rect(WIDTH, HEIGHT, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Preset Conflict ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::Red))
        .title_bottom(
            Line::from(Span::styled(
                app.keymap.hints(&[
                    (Context::Conflict, &[Action::Override], "override"),
                    (Context::Conflict, &[Action::Discard], "no"),
                    (Context::Conflict, &[Action::Rename], "rename+timestamp"),
                    (Context::Conflict, &[Action::Close], "cancel"),
                ]),
                Style::default().fg(Color::DarkGray),
            ))
            .centered(),
        );

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let existing = &conflict.existing;
    let incoming = &conflict.incoming;

    let slot_label = existing
        .slot
        .map(|s| format!("slot {:03}", s + 1))
        .unwrap_or_else(|| "local".to_owned());

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "A local preset already occupies this slot:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("\"{}\"", existing.name),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ({})", slot_label),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Incoming: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("\"{}\"", incoming.name),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(content), inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
