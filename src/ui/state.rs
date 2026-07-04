//! Application state types and constants.
//!
//! All enums, structs, and constants that make up the observable state of the
//! TUI. Consumed by event handlers in [`super::app`] and renderers in
//! [`super::render`].

use crate::storage::LibraryEntry;

// ── Status icons (Nerd Fonts) ─────────────────────────────────────────────────

/// Device offline / disconnected.
pub const ICON_OFFLINE: &str = "󰣱";
/// Device connected and idle.
pub const ICON_ONLINE: &str = "󰣲";
/// Braille spinner frames shown when an operation is in progress.
pub const SPINNER_FRAMES: [&str; 8] = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];

/// Persistent status-bar indicator warning that the TUI's device data is mock.
///
/// The ratatui TUI does not yet exchange MIDI messages with the device
/// (roadmap phase 6). Every "Online" state and preset shown is synthetic —
/// this banner keeps that fact visible so users don't act on stale/fake data.
///
/// Remove once real MIDI is wired into the event loop.
pub const MOCK_MIDI_BANNER: &str = "[MOCK MIDI — phase 6] ";

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

#[derive(Clone, Default, PartialEq, Eq)]
pub enum OperationStatus {
    #[default]
    Offline,
    Online,
    /// A MIDI operation is in progress. `label` describes it (e.g. "Fetching presets…").
    #[allow(dead_code)]
    Busy {
        label: String,
    },
}

// ── Device info ───────────────────────────────────────────────────────────────

pub struct DeviceInfo {
    pub model: String,
    pub fw_version: String,
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
