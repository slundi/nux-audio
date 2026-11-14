//! Application state types and constants.
//!
//! All enums, structs, and constants that make up the observable state of the
//! TUI. Consumed by event handlers in [`super::app`] and renderers in
//! [`super::render`].

use crate::session::worker::DeviceSummary;
use crate::storage::LibraryEntry;

// ── Status icons (Nerd Fonts) ─────────────────────────────────────────────────

/// Device offline / disconnected.
pub const ICON_OFFLINE: &str = "󰣱";
/// Device connected and idle.
pub const ICON_ONLINE: &str = "󰣲";
/// Braille spinner frames shown when an operation is in progress.
pub const SPINNER_FRAMES: [&str; 8] = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];

/// Status-bar warning shown while an overlay that does not talk to the device yet
/// (looper, drum machine) is open, so users don't expect the pedal to react.
///
/// Device connection and preset fetch are real; remove this once the looper and
/// drum SysEx are wired through the session.
pub const UI_ONLY_BANNER: &str = "[UI only — not sent to device] ";

// ── Tab ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Presets = 0,
    LiveBoard = 1,
    About = 2,
}

// ── Overlay ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    DevicePicker,
    Looper,
    Drum,
    Conflict,
    Help,
}

// ── Conflict state ────────────────────────────────────────────────────────────

/// State carried while the conflict overlay is visible.
pub struct ConflictState {
    /// The preset fetched from the device that would replace `existing`.
    pub incoming: LibraryEntry,
    /// The existing library entry that occupies the same slot.
    pub existing: LibraryEntry,
    /// Index of the existing entry in `library.presets`.
    pub existing_idx: usize,
}

// ── Operation status ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum OperationStatus {
    #[default]
    Offline,
    Online,
    /// A device operation is in progress. `label` describes it (e.g. "Fetching preset…").
    Busy {
        label: String,
    },
}

// ── Device info ───────────────────────────────────────────────────────────────

/// The connected device, as reported by the session worker.
pub type DeviceInfo = DeviceSummary;

/// Text shown at the right of the tab bar (shared by render and mouse hit-testing).
pub fn device_label(info: Option<&DeviceInfo>) -> String {
    match info {
        Some(info) => format!(" {} {}", info.model, info.firmware),
        None => " offline".to_owned(),
    }
}

// ── Looper state ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct LooperState {
    pub is_recording: bool,
    pub is_playing: bool,
    /// Tempo in BPM (20–240).
    pub bpm: u16,
    /// Output volume (0–127).
    pub volume: u8,
}

impl LooperState {
    pub fn new() -> Self {
        Self {
            bpm: 120,
            volume: 80,
            ..Default::default()
        }
    }
}

// ── Drum machine state ────────────────────────────────────────────────────────

pub const DRUM_PATTERNS: &[&str] = &[
    "Rock 1",
    "Rock 2",
    "Blues Shuffle",
    "Latin",
    "Pop 1",
    "Pop 2",
    "Jazz",
    "Funk",
];

pub struct DrumState {
    pub pattern_index: usize,
    /// Tempo in BPM (20–240).
    pub bpm: u16,
    /// Output volume (0–127).
    pub volume: u8,
    pub is_playing: bool,
}

impl Default for DrumState {
    fn default() -> Self {
        Self {
            pattern_index: 0,
            bpm: 120,
            volume: 80,
            is_playing: false,
        }
    }
}

// ── Device picker state ───────────────────────────────────────────────────────

#[derive(Default)]
pub struct DevicePickerState {
    pub selected: usize,
    /// Model names of detected MIDI devices.
    pub devices: Vec<String>,
}
