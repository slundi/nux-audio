// AMP models for MG-30 — 25 electric + 3 bass + 2 acoustic = 30 total
// Reference: amp.rs device observations + effects.yaml
//
// Bypass CC: 3
// All knob CCs are positional and shared across all amp models:
//   CC22=GAIN  CC23=MASTER  CC24=BASS/CUT  CC25=MIDDLE/MID_FREQ
//   CC26=TREBLE  CC27=PRESENCE/TONE/CUT  CC28=BIAS/BRIGHT_SW  CC29=LEVEL
//
// IDs are provisional — sequential 0x00..0x1D matching order in amp.rs comments.

use std::{collections::HashMap, sync::LazyLock};

use super::{EffectOption, KnobDef};

fn knob(id: &'static str, title: &'static str, cc: u8) -> KnobDef<'static> {
    KnobDef {
        id,
        title,
        range_start: Some(0),
        range_end: Some(100),
        ctrl: cc,
    }
}

// CC number constants for amp knob positions
const GAIN: u8 = 22;
const MASTER: u8 = 23;
const BASS: u8 = 24;
const MIDDLE: u8 = 25;
const TREBLE: u8 = 26;
const PRESENCE: u8 = 27;
const BIAS: u8 = 28;
const LEVEL: u8 = 29;

// Label aliases that reuse the same CC but with different names
fn gain() -> KnobDef<'static> {
    knob("gain", "GAIN", GAIN)
}
fn master() -> KnobDef<'static> {
    knob("master", "MASTER", MASTER)
}
fn bass() -> KnobDef<'static> {
    knob("bass", "BASS", BASS)
}
fn cut() -> KnobDef<'static> {
    knob("cut", "CUT", BASS)
} // same CC as bass
fn middle() -> KnobDef<'static> {
    knob("middle", "MIDDLE", MIDDLE)
}
fn mid_freq() -> KnobDef<'static> {
    knob("mid_freq", "M FREQ", MIDDLE)
} // same CC as middle
fn treble() -> KnobDef<'static> {
    knob("treble", "TREBLE", TREBLE)
}
fn presence() -> KnobDef<'static> {
    knob("presence", "PRESENCE", PRESENCE)
}
fn tone_p() -> KnobDef<'static> {
    knob("tone", "TONE", PRESENCE)
} // same CC as presence
fn bright() -> KnobDef<'static> {
    knob("bright_sw", "BRIGHT SW", BIAS)
} // same CC as bias
fn bias() -> KnobDef<'static> {
    knob("bias", "BIAS", BIAS)
}
fn level() -> KnobDef<'static> {
    knob("level", "LEVEL", LEVEL)
}

/// MG-30 amp model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption<'static>>> = LazyLock::new(|| {
    HashMap::from([
        // --- Electric / Clean ---
        (
            0x00,
            EffectOption {
                id: 0x00,
                title: "Jazz Clean",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    bright(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x01,
            EffectOption {
                id: 0x01,
                title: "Deluxe Rvb",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x02,
            EffectOption {
                id: 0x02,
                title: "Bass Mate",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x03,
            EffectOption {
                id: 0x03,
                title: "Tweedy",
                knobs: vec![gain(), tone_p(), master(), level()],
                ..Default::default()
            },
        ),
        (
            0x04,
            EffectOption {
                id: 0x04,
                title: "Twin Rvb",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    bright(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        // --- Electric / Crunch ---
        (
            0x05,
            EffectOption {
                id: 0x05,
                title: "Hiwire",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x06,
            EffectOption {
                id: 0x06,
                title: "Cali Crunch",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x07,
            EffectOption {
                id: 0x07,
                title: "Class A 15",
                knobs: vec![gain(), master(), cut(), bass(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x08,
            EffectOption {
                id: 0x08,
                title: "Class A 30",
                knobs: vec![gain(), master(), cut(), bass(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x09,
            EffectOption {
                id: 0x09,
                title: "Plexi 100W",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x0A,
            EffectOption {
                id: 0x0A,
                title: "Plexi 45",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        // --- Electric / High Gain ---
        (
            0x0B,
            EffectOption {
                id: 0x0B,
                title: "Brit 800",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x0C,
            EffectOption {
                id: 0x0C,
                title: "1987X 50W",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x0D,
            EffectOption {
                id: 0x0D,
                title: "SLO 100",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x0E,
            EffectOption {
                id: 0x0E,
                title: "Fireman HBE",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x0F,
            EffectOption {
                id: 0x0F,
                title: "Dual Rect",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x10,
            EffectOption {
                id: 0x10,
                title: "Die VH4",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x11,
            EffectOption {
                id: 0x11,
                title: "Vibro King",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x12,
            EffectOption {
                id: 0x12,
                title: "Budda",
                knobs: vec![gain(), master(), cut(), bass(), middle(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x13,
            EffectOption {
                id: 0x13,
                title: "Mr Z 38",
                knobs: vec![gain(), master(), cut(), bass(), middle(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x14,
            EffectOption {
                id: 0x14,
                title: "Super Rvb",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    bright(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x15,
            EffectOption {
                id: 0x15,
                title: "Brit Blues",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x16,
            EffectOption {
                id: 0x16,
                title: "Match",
                knobs: vec![gain(), master(), cut(), bass(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x17,
            EffectOption {
                id: 0x17,
                title: "Brit 2000",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x18,
            EffectOption {
                id: 0x18,
                title: "Uber",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    bias(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        // --- Bass ---
        (
            0x19,
            EffectOption {
                id: 0x19,
                title: "AGL",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    mid_freq(),
                    treble(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x1A,
            EffectOption {
                id: 0x1A,
                title: "Bassguy",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    treble(),
                    presence(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        (
            0x1B,
            EffectOption {
                id: 0x1B,
                title: "MLD",
                knobs: vec![
                    gain(),
                    master(),
                    bass(),
                    middle(),
                    mid_freq(),
                    treble(),
                    level(),
                ],
                ..Default::default()
            },
        ),
        // --- Acoustic ---
        (
            0x1C,
            EffectOption {
                id: 0x1C,
                title: "Optima Air",
                knobs: vec![gain(), master(), bass(), middle(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x1D,
            EffectOption {
                id: 0x1D,
                title: "Stageman",
                knobs: vec![gain(), master(), bass(), middle(), treble(), level()],
                ..Default::default()
            },
        ),
    ])
});
