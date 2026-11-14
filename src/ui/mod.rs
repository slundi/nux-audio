//! TUI module: application state, event loop, and tab renderers.
//!
//! Key bindings are user-configurable, see [`keymap`] and `keybindings.example.yaml`.
//!
//! Entry point: [`run_tui`]. Call it from a blocking context to start the
//! interactive terminal UI.

pub mod action;
pub mod key;

mod app;
mod key_handlers;
mod keymap;
mod mouse;
mod overlay_conflict;
mod overlay_device;
mod overlay_drum;
mod overlay_help;
mod overlay_looper;
mod render;
mod session_link;
mod state;
mod tab_about;
mod tab_liveboard;
mod tab_presets;

pub use app::run_tui;
