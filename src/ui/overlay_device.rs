//! Device picker overlay.
//!
//! Opened with `d`. Lists MIDI devices detected by [`crate::device::Device::detect`].
//! Selecting a device and pressing `Enter` sets it as the active device.
//!
//! ```text
//! ╔══════════════════════════════════╗
//! ║  Select Device                   ║
//! ║  ────────────────────────────── ║
//! ║  ▶ NUX MG-30                    ║
//! ║    NUX Mighty Plug Pro           ║
//! ║                                  ║
//! ║  No devices — connect via USB    ║
//! ╚══════════════════════════════════╝
//! ```

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::app::App;

const WIDTH: u16 = 44;
const HEIGHT: u16 = 12;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(WIDTH, HEIGHT, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Select Device ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::Yellow))
        .title_bottom(
            Line::from(Span::styled(
                " Enter:connect  Esc:cancel ",
                Style::default().fg(Color::DarkGray),
            ))
            .centered(),
        );

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if app.device_picker.devices.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "󰣱  No devices detected",
                Style::default().fg(Color::Red),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Connect your NUX device via USB",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "and press d again to refresh.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .centered();
        frame.render_widget(msg, inner);
        return;
    }

    let mut lines: Vec<Line> = vec![Line::from("")];
    for (i, name) in app.device_picker.devices.iter().enumerate() {
        let is_selected = i == app.device_picker.selected;
        let prefix = if is_selected { "▶  " } else { "   " };
        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(format!("{prefix}{name}"), style)));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
