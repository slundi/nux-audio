//! Help overlay — key bindings reference, generated from the active key map
//! so it reflects `keybindings.yaml`.
//!
//! Opened with the `help` action (`?` by default). Closed with its `close` action.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::action::{Action, Context};
use super::app::App;

const WIDTH: u16 = 54;
/// Width of the key column, in characters.
const KEY_COLUMN: usize = 16;

/// One help line: the actions whose keys are shown together, and what they do.
type Entry = (Context, &'static [Action], &'static str);

const SECTIONS: &[(&str, &[Entry])] = &[
    (
        "Global",
        &[
            (
                Context::Global,
                &[Action::TabPresets, Action::TabLiveBoard, Action::TabAbout],
                "Switch tabs (Presets / Live Board / About)",
            ),
            (
                Context::Global,
                &[Action::ChangeDevice],
                "Open device picker",
            ),
            (
                Context::Global,
                &[Action::OpenLooper],
                "Open looper overlay",
            ),
            (
                Context::Global,
                &[Action::OpenDrum],
                "Open drum machine overlay",
            ),
            (Context::Global, &[Action::Quit], "Quit"),
            (Context::Global, &[Action::Help], "This help"),
        ],
    ),
    (
        "Presets tab",
        &[
            (
                Context::Presets,
                &[Action::Up, Action::Down],
                "Move preset selection",
            ),
            (
                Context::Presets,
                &[Action::Left, Action::Right],
                "Move chain slot focus",
            ),
            (
                Context::Presets,
                &[Action::Confirm, Action::Close],
                "Focus / leave chain panel",
            ),
            (
                Context::Presets,
                &[Action::NextScene],
                "Cycle scene (A → B → C → A)",
            ),
            (
                Context::Presets,
                &[Action::GetPreset],
                "Pull current preset from device",
            ),
            (
                Context::Presets,
                &[Action::SendPreset],
                "Send selected preset to device",
            ),
        ],
    ),
    (
        "Live Board tab",
        &[
            (
                Context::LiveBoard,
                &[Action::Left, Action::Right],
                "Move chain slot focus",
            ),
            (
                Context::LiveBoard,
                &[Action::NextScene],
                "Cycle scene (A → B → C → A)",
            ),
        ],
    ),
    (
        "Overlays",
        &[
            (Context::Help, &[Action::Close], "Close overlay"),
            (
                Context::Conflict,
                &[Action::Override, Action::Discard, Action::Rename],
                "Conflict: Override / No / Rename",
            ),
        ],
    ),
];

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let lines = help_lines(app);
    let height = lines.len() as u16 + 2;
    let popup = centered_rect(WIDTH, height, area);
    frame.render_widget(Clear, popup);

    let close_key = app.keymap.primary(Context::Help, Action::Close);
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
                format!(" {close_key}:close "),
                Style::default().fg(Color::DarkGray),
            ))
            .centered(),
        );

    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Help text as lines: section headers followed by `keys  description`.
fn help_lines(app: &App) -> Vec<Line<'static>> {
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::UNDERLINED);
    let text_style = Style::default().fg(Color::White);

    let mut lines = Vec::new();
    for (index, (title, entries)) in SECTIONS.iter().enumerate() {
        if index > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(*title, header_style)));
        for (context, actions, description) in *entries {
            let keys = actions
                .iter()
                .map(|action| app.keymap.label(*context, *action))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(Line::from(vec![
                Span::styled(format!("  {keys:<KEY_COLUMN$} "), key_style),
                Span::styled(*description, text_style),
            ]));
        }
    }
    lines
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::keymap::KeyMap;

    fn text(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn help_shows_default_keys() {
        let mut app = App::new();
        app.keymap = KeyMap::default();
        let help = text(&help_lines(&app));
        assert!(help.contains("Quit"));
        assert!(help.contains("k/↑ j/↓"), "{help}");
        assert!(help.contains("F1 F2 F3"), "{help}");
    }

    #[test]
    fn help_follows_custom_bindings() {
        let mut app = App::new();
        app.keymap = KeyMap::from_yaml("global:\n  quit: [\"x\"]\n  help: []\n");
        let help = text(&help_lines(&app));
        let quit_line = help.lines().find(|line| line.ends_with("Quit")).unwrap();
        assert!(quit_line.trim_start().starts_with('x'), "{quit_line}");
        let help_line = help
            .lines()
            .find(|line| line.ends_with("This help"))
            .unwrap();
        assert!(help_line.trim_start().starts_with('-'), "{help_line}");
    }
}
