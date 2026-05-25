//! Live Board tab: full-screen CC performance view.
//!
//! Shows all 11 effect slots as large blocks with knob values.
//! Real-time CC is sent as knob values change (wired in a later phase).
//!
//! Layout:
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │ Scene: A | B | C                                                    │
//! ├────────┬────────┬────────┬────────┬─────────────────────────────────┤
//! │[WAH   ]│[CMP   ]│[EFX   ]│[AMP   ]│ ...                            │
//! │bypassed│Ratio:4 │        │TubeCl  │                                 │
//! │        │Thr:-10 │        │Gain:30 │                                 │
//! └────────┴────────┴────────┴────────┴─────────────────────────────────┘
//! ```

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::app::App;

/// MG-30 slot names in signal-chain order.
const SLOTS: [(&str, &str); 11] = [
    ("WAH", "Wah"),
    ("CMP", "Comp"),
    ("EFX", "EFX"),
    ("AMP", "Amp"),
    ("EQ", "EQ"),
    ("GTE", "Gate"),
    ("MOD", "Mod"),
    ("DLY", "Delay"),
    ("REV", "Reverb"),
    ("IR", "IR"),
    ("VOL", "Vol"),
];

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let [scene_bar, grid_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);

    render_scene_bar(frame, scene_bar, app);
    render_slot_grid(frame, grid_area, app);
}

fn render_scene_bar(frame: &mut Frame, area: Rect, app: &App) {
    let scenes = ["A", "B", "C"];
    let spans: Vec<Span> = scenes
        .iter()
        .flat_map(|&s| {
            let style = if s == app.scene_label() {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            [Span::styled(format!(" {s} "), style), Span::raw(" ")]
        })
        .collect();
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_slot_grid(frame: &mut Frame, area: Rect, app: &App) {
    // Equal-width columns for all 11 slots.
    let constraints: Vec<Constraint> = (0..SLOTS.len())
        .map(|_| Constraint::Ratio(1, SLOTS.len() as u32))
        .collect();
    let cols = Layout::horizontal(constraints).split(area);

    for (i, ((abbrev, full_name), &col_area)) in SLOTS.iter().zip(cols.iter()).enumerate() {
        render_slot_block(frame, col_area, abbrev, full_name, i, app);
    }
}

fn render_slot_block(
    frame: &mut Frame,
    area: Rect,
    abbrev: &str,
    full_name: &str,
    slot_index: usize,
    app: &App,
) {
    let is_focused = slot_index == app.chain_slot_focus;

    // Check if this slot appears in the active preset's effect order.
    let is_active = app
        .library
        .presets
        .get(app.preset_selected)
        .map(|e| {
            e.effect_order
                .iter()
                .any(|s| s.to_lowercase() == full_name.to_lowercase())
        })
        .unwrap_or(false);

    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else if is_active {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(" {abbrev} ");
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Placeholder content: show "active" / "bypassed" / "—".
    let body = if is_active {
        Paragraph::new(Line::from(Span::styled(
            "active",
            Style::default().fg(Color::Green),
        )))
    } else {
        Paragraph::new(Line::from(Span::styled(
            "—",
            Style::default().fg(Color::DarkGray),
        )))
    };

    frame.render_widget(body, inner);
}
