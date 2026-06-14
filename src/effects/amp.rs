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

fn knob(id: &str, title: &str, cc: u8) -> KnobDef {
    KnobDef {
        id: id.to_owned(),
        title: title.to_owned(),
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
fn gain() -> KnobDef {
    knob("gain", "GAIN", GAIN)
}
fn master() -> KnobDef {
    knob("master", "MASTER", MASTER)
}
fn bass() -> KnobDef {
    knob("bass", "BASS", BASS)
}
fn cut() -> KnobDef {
    knob("cut", "CUT", BASS)
} // same CC as bass
fn middle() -> KnobDef {
    knob("middle", "MIDDLE", MIDDLE)
}
fn mid_freq() -> KnobDef {
    knob("mid_freq", "M FREQ", MIDDLE)
} // same CC as middle
fn treble() -> KnobDef {
    knob("treble", "TREBLE", TREBLE)
}
fn presence() -> KnobDef {
    knob("presence", "PRESENCE", PRESENCE)
}
fn tone_p() -> KnobDef {
    knob("tone", "TONE", PRESENCE)
} // same CC as presence
fn bright() -> KnobDef {
    knob("bright_sw", "BRIGHT SW", BIAS)
} // same CC as bias
fn bias() -> KnobDef {
    knob("bias", "BIAS", BIAS)
}
fn level() -> KnobDef {
    knob("level", "LEVEL", LEVEL)
}

/// MG-30 amp model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption>> = LazyLock::new(|| {
    HashMap::from([
        // --- Electric / Clean ---
        (
            0x00,
            EffectOption {
                id: 0x00,
                title: "Jazz Clean".into(),
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
                title: "Deluxe Rvb".into(),
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
                title: "Bass Mate".into(),
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
                title: "Tweedy".into(),
                knobs: vec![gain(), tone_p(), master(), level()],
                ..Default::default()
            },
        ),
        (
            0x04,
            EffectOption {
                id: 0x04,
                title: "Twin Rvb".into(),
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
                title: "Hiwire".into(),
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
                title: "Cali Crunch".into(),
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
                title: "Class A 15".into(),
                knobs: vec![gain(), master(), cut(), bass(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x08,
            EffectOption {
                id: 0x08,
                title: "Class A 30".into(),
                knobs: vec![gain(), master(), cut(), bass(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x09,
            EffectOption {
                id: 0x09,
                title: "Plexi 100W".into(),
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
                title: "Plexi 45".into(),
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
                title: "Brit 800".into(),
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
                title: "1987X 50W".into(),
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
                title: "SLO 100".into(),
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
                title: "Fireman HBE".into(),
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
                title: "Dual Rect".into(),
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
                title: "Die VH4".into(),
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
                title: "Vibro King".into(),
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
                title: "Budda".into(),
                knobs: vec![gain(), master(), cut(), bass(), middle(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x13,
            EffectOption {
                id: 0x13,
                title: "Mr Z 38".into(),
                knobs: vec![gain(), master(), cut(), bass(), middle(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x14,
            EffectOption {
                id: 0x14,
                title: "Super Rvb".into(),
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
                title: "Brit Blues".into(),
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
                title: "Match".into(),
                knobs: vec![gain(), master(), cut(), bass(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x17,
            EffectOption {
                id: 0x17,
                title: "Brit 2000".into(),
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
                title: "Uber".into(),
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
                title: "AGL".into(),
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
                title: "Bassguy".into(),
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
                title: "MLD".into(),
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
                title: "Optima Air".into(),
                knobs: vec![gain(), master(), bass(), middle(), treble(), level()],
                ..Default::default()
            },
        ),
        (
            0x1D,
            EffectOption {
                id: 0x1D,
                title: "Stageman".into(),
                knobs: vec![gain(), master(), bass(), middle(), treble(), level()],
                ..Default::default()
            },
        ),
    ])
});
