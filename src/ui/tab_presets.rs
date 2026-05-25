//! Presets tab: single-pane preset list + bottom effect-chain panel.
//!
//! Layout:
//! ```text
//! ┌─────────────────────────────────────────┐
//! │ Src  Slot  Name              Effects     │
//! │ 📟    001  Crunch Lead       [AMP·DLY·REV]
//! │ 📄    —    My Blues Driver   [WAH·AMP·MOD]
//! ├─────────────────────────────────────────┤
//! │ Scene: A  Effect chain                  │
//! │ [WAH]──[CMP]──[EFX]──[AMP]──...        │
//! └─────────────────────────────────────────┘
//! ```

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

use crate::storage::LibraryEntry;

use super::app::App;

/// MG-30 signal-chain slot display names, in hardware order.
const CHAIN_SLOTS: [&str; 11] = [
    "WAH", "CMP", "EFX", "AMP", "EQ ", "GTE", "MOD", "DLY", "REV", "IR ", "VOL",
];

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let [list_area, chain_area] =
        Layout::vertical([Constraint::Percentage(65), Constraint::Percentage(35)]).areas(area);

    render_preset_list(frame, list_area, app);
    render_chain_panel(frame, chain_area, app);
}

// ── Preset list ───────────────────────────────────────────────────────────────

fn render_preset_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Presets ")
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.library.presets.is_empty() {
        let msg = Paragraph::new(Line::from(vec![
            Span::styled("No local presets. ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "Press g to pull from device.",
                Style::default().fg(Color::Yellow),
            ),
        ]))
        .centered();
        frame.render_widget(msg, inner);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Src").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Slot").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Name").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Effects").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().fg(Color::Yellow));

    let rows: Vec<Row> = app
        .library
        .presets
        .iter()
        .map(|entry| {
            let src = source_icon(entry);
            let slot = entry
                .slot
                .map(|s| format!("{:03}", s + 1))
                .unwrap_or_else(|| "—".to_owned());
            let name = entry.name.clone();
            let effects = effect_summary(entry);
            Row::new(vec![
                Cell::from(src),
                Cell::from(slot),
                Cell::from(name),
                Cell::from(effects),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(4), // src
        Constraint::Length(5), // slot
        Constraint::Min(24),   // name
        Constraint::Min(20),   // effects
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .highlight_symbol("▶ ");

    let mut state = TableState::default().with_selected(Some(app.preset_selected));
    frame.render_stateful_widget(table, inner, &mut state);
}

fn source_icon(entry: &LibraryEntry) -> &'static str {
    match entry.slot {
        Some(_) => "📟",
        None => "📄",
    }
}

fn effect_summary(entry: &LibraryEntry) -> String {
    if entry.effect_order.is_empty() {
        return "—".to_owned();
    }
    let abbrevs: Vec<String> = entry
        .effect_order
        .iter()
        .map(|s| abbrev_category(s))
        .collect();
    format!("[{}]", abbrevs.join("·"))
}

fn abbrev_category(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "wah" => "WAH".into(),
        "comp" => "CMP".into(),
        "efx" => "EFX".into(),
        "amp" => "AMP".into(),
        "eq" => "EQ".into(),
        "gate" => "GTE".into(),
        "mod" => "MOD".into(),
        "delay" => "DLY".into(),
        "reverb" => "REV".into(),
        "ir" => "IR".into(),
        "vol" => "VOL".into(),
        other => other[..3.min(other.len())].to_uppercase(),
    }
}

// ── Effect chain panel ────────────────────────────────────────────────────────

fn render_chain_panel(frame: &mut Frame, area: Rect, app: &App) {
    let selected_name = app
        .library
        .presets
        .get(app.preset_selected)
        .map(|e| e.name.as_str())
        .unwrap_or("—");

    let scene_title = format!(" Chain: {}  [Scene {}] ", selected_name, app.scene_label());
    let border_style = if app.chain_panel_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(scene_title)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Active slot names from the selected preset's effect_order.
    let active_slots: Vec<String> = app
        .library
        .presets
        .get(app.preset_selected)
        .map(|e| e.effect_order.iter().map(|s| abbrev_category(s)).collect())
        .unwrap_or_default();

    let chain_line = build_chain_line(&active_slots, app.chain_slot_focus, app.chain_panel_active);

    let hint = if app.chain_panel_active {
        Line::from(Span::styled(
            " ← → to navigate slots · Enter to exit",
            Style::default().fg(Color::DarkGray),
        ))
    } else {
        Line::from(Span::styled(
            " Enter to focus chain panel",
            Style::default().fg(Color::DarkGray),
        ))
    };

    let content = vec![Line::from(""), chain_line, Line::from(""), hint];
    let para = Paragraph::new(content);
    frame.render_widget(para, inner);
}

/// Build a styled line showing all 11 slots. Active slots are green, focused
/// slot is highlighted in yellow, inactive slots are dim.
fn build_chain_line(active_slots: &[String], focus: usize, panel_active: bool) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    for (i, &slot_name) in CHAIN_SLOTS.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("──", Style::default().fg(Color::DarkGray)));
        }

        // Check if this slot is in the active chain.
        let slot_abbrev = slot_name.trim();
        let is_active = active_slots.iter().any(|a| a.trim() == slot_abbrev);
        let is_focused = panel_active && i == focus;

        let label = format!("[{slot_name}]");
        let style = if is_focused {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if is_active {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        spans.push(Span::styled(label, style));
    }

    Line::from(spans)
}
