//! Logical UI actions routed through [`super::app::App::handle_action`].
//!
//! Every variant here must have a corresponding arm in `handle_action`. When
//! you add a new feature (preset delete, per-slot bypass toggle from the TUI,
//! etc.), add the enum variant AND its dispatch arm in the same change — do
//! not pre-declare aspirational variants, or `handle_action` will silently
//! drop the action.

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum GlobalAction {
    Quit,
    ChangeDevice,
    Help,
    GetPreset,
    SendPreset,
}

// ── Bindable actions ──────────────────────────────────────────────────────────

/// Input context in which a key binding is looked up.
///
/// `Global` is consulted first (when no overlay is open), then the context of
/// the active tab. When an overlay is open, only the overlay's context is used.
/// The snake_case names are the top-level keys of `keybindings.yaml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Context {
    Global,
    Presets,
    LiveBoard,
    Help,
    DevicePicker,
    Looper,
    Drum,
    Conflict,
}

impl Context {
    /// Tab contexts are shadowed by `Global`: a key bound globally never
    /// reaches them, so such a binding would be dead.
    pub fn is_tab(self) -> bool {
        matches!(self, Self::Presets | Self::LiveBoard)
    }
}

/// A user-bindable action. Which actions a context accepts is defined by the
/// default bindings in [`super::keymap`]; the snake_case names are the keys of
/// each context in `keybindings.yaml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Global
    Quit,
    Help,
    ChangeDevice,
    TabPresets,
    TabLiveBoard,
    TabAbout,
    OpenLooper,
    OpenDrum,
    // Navigation shared by several contexts
    Up,
    Down,
    Left,
    Right,
    Increase,
    Decrease,
    Confirm,
    Close,
    NextScene,
    TogglePlay,
    // Presets tab
    GetPreset,
    SendPreset,
    // Looper overlay
    ToggleRecord,
    Clear,
    // Conflict overlay
    Override,
    Discard,
    Rename,
}
