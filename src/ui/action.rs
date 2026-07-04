//! Logical UI actions routed through [`super::app::App::handle_action`].
//!
//! Every variant here must have a corresponding arm in `handle_action`. When
//! you add a new feature (preset delete, per-slot bypass toggle from the TUI,
//! etc.), add the enum variant AND its dispatch arm in the same change — do
//! not pre-declare aspirational variants, or `handle_action` will silently
//! drop the action.

#[derive(Debug)]
pub enum GlobalAction {
    Quit,
    ChangeDevice,
    Help,
    GetPreset,
    SendPreset,
}
