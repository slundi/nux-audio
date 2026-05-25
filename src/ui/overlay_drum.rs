//! Drum machine overlay.
//!
//! Opened with `D` (Shift+d). Select pattern, adjust BPM, toggle play/stop.
//!
//! ```text
//! ╔══════════════════════════════════╗
//! ║  Drum Machine        ▶ playing  ║
//! ║  ────────────────────────────── ║
//! ║    Rock 1                        ║
//! ║  ▶ Blues Shuffle                 ║
//! ║    Latin                         ║
//! ║  ────────────────────────────── ║
//! ║  BPM  120   Vol  80             ║
//! ╚══════════════════════════════════╝
//! ```

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::app::App;
use super::state::DRUM_PATTERNS;

const WIDTH: u16 = 44;
const HEIGHT: u16 = 16;
/// How many patterns to show in the visible list window.
const VISIBLE: usize = 6;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(WIDTH, HEIGHT, area);
    frame.render_widget(Clear, popup);

    let playing_span = if app.drum.is_playing {
        Span::styled("  ▶ playing", Style::default().fg(Color::Green))
    } else {
        Span::styled("  ■ stopped", Style::default().fg(Color::DarkGray))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled(
                " Drum Machine",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            playing_span,
            Span::raw(" "),
        ]))
        .border_style(Style::default().fg(Color::Magenta))
        .title_bottom(
            Line::from(Span::styled(
                " j/k:pattern  ←→:bpm  Space:play  Esc:close ",
                Style::default().fg(Color::DarkGray),
            ))
            .centered(),
        );

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Split inner into list + footer.
    let [list_area, sep_area, footer_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    render_pattern_list(frame, list_area, app);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(list_area.width as usize),
            Style::default().fg(Color::DarkGray),
        )),
        sep_area,
    );

    let bpm_vol = Line::from(vec![
        Span::styled("BPM ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:3}", app.drum.bpm),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled("Vol ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:3}", app.drum.volume),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(bpm_vol), footer_area);
}

fn render_pattern_list(frame: &mut Frame, area: Rect, app: &App) {
    let selected = app.drum.pattern_index;
    // Compute visible window so the selected item is always in view.
    let start = selected
        .saturating_sub(VISIBLE / 2)
        .min(DRUM_PATTERNS.len().saturating_sub(VISIBLE));

    let lines: Vec<Line> = DRUM_PATTERNS
        .iter()
        .enumerate()
        .skip(start)
        .take(VISIBLE)
        .map(|(i, name)| {
            let is_sel = i == selected;
            let prefix = if is_sel { "▶  " } else { "   " };
            let style = if is_sel {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(Span::styled(format!("{prefix}{name}"), style))
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
