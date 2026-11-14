//! Looper overlay.
//!
//! Opened with `l`. Controls record / play / stop / clear and BPM / volume.
//!
//! ```text
//! ╔═══════════════════════════════╗
//! ║  Looper                       ║
//! ║  ─────────────────────────── ║
//! ║  ● REC   ▶ PLAY   ■ STOP     ║
//! ║          × CLEAR             ║
//! ║  ─────────────────────────── ║
//! ║  BPM  120  ←/→  ↑/↓  Vol 80 ║
//! ╚═══════════════════════════════╝
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

const WIDTH: u16 = 40;
const HEIGHT: u16 = 12;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(WIDTH, HEIGHT, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Looper ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(Color::Cyan))
        .title_bottom(
            Line::from(Span::styled(
                app.keymap.hints(&[
                    (Context::Looper, &[Action::ToggleRecord], "rec"),
                    (Context::Looper, &[Action::TogglePlay], "play"),
                    (Context::Looper, &[Action::Clear], "clear"),
                    (Context::Looper, &[Action::Up, Action::Down], "vol"),
                    (
                        Context::Looper,
                        &[Action::Decrease, Action::Increase],
                        "bpm",
                    ),
                    (Context::Looper, &[Action::Close], "close"),
                ]),
                Style::default().fg(Color::DarkGray),
            ))
            .centered(),
        );

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let looper = &app.looper;

    // Transport buttons row.
    let rec_style = if looper.is_recording {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let play_style = if looper.is_playing {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let transport = Line::from(vec![
        Span::styled("● REC", rec_style),
        Span::raw("   "),
        Span::styled("▶ PLAY", play_style),
        Span::raw("   "),
        Span::styled(
            "■ STOP",
            if !looper.is_playing && !looper.is_recording {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
    ]);

    let bpm_vol = Line::from(vec![
        Span::styled("BPM ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:3}", looper.bpm),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled("Vol ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:3}", looper.volume),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let content = vec![
        Line::from(""),
        transport,
        Line::from(""),
        Line::from(Span::styled(
            "─────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        bpm_vol,
    ];

    frame.render_widget(Paragraph::new(content).centered(), inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
