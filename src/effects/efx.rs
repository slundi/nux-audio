// EFX (pre-amp drive/boost/fuzz) models for MG-30
// Reference: efx.rs device observations + effects.yaml
//
// Bypass CC: 2
// Knob CCs: KNOB1=17, KNOB2=18, KNOB3=19, KNOB4=20, KNOB5=21
//
// IDs confirmed from hardware reverse-engineering.

use std::{collections::HashMap, sync::LazyLock};

use super::{EffectOption, Knob};

fn knob(id: &'static str, title: &'static str, cc: u8, end: u8) -> Knob<'static> {
    Knob {
        id,
        title,
        range_start: Some(0),
        range_end: Some(end),
        ctrl: cc,
        current_value: None,
    }
}

/// MG-30 EFX model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption<'static>>> = LazyLock::new(|| {
    HashMap::from([
        (
            0x40,
            EffectOption {
                id: 0x40,
                title: "Dist+",
                knobs: vec![
                    knob("output", "OUTPUT", 17, 100),
                    knob("sens", "SENSITIVITY", 18, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x41,
            EffectOption {
                id: 0x41,
                title: "RC Boost",
                knobs: vec![
                    knob("gain", "GAIN", 17, 100),
                    knob("vol", "VOL", 18, 100),
                    knob("bass", "BASS", 19, 100),
                    knob("treble", "TREBLE", 20, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x42,
            EffectOption {
                id: 0x42,
                title: "AC Boost",
                knobs: vec![
                    knob("gain", "GAIN", 17, 100),
                    knob("vol", "VOL", 18, 100),
                    knob("bass", "BASS", 19, 100),
                    knob("treble", "TREBLE", 20, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x43,
            EffectOption {
                id: 0x43,
                title: "Dist One",
                knobs: vec![
                    knob("level", "LEVEL", 17, 100),
                    knob("tone", "TONE", 18, 100),
                    knob("drive", "DRIVE", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x44,
            EffectOption {
                id: 0x44,
                title: "T Scream",
                knobs: vec![
                    knob("drive", "DRIVE", 17, 100),
                    knob("tone", "TONE", 18, 100),
                    knob("level", "LEVEL", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x45,
            EffectOption {
                id: 0x45,
                title: "Blues Drv",
                knobs: vec![
                    knob("level", "LEVEL", 17, 100),
                    knob("tone", "TONE", 18, 100),
                    knob("gain", "GAIN", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x46,
            EffectOption {
                id: 0x46,
                title: "Morning Drv",
                knobs: vec![
                    knob("volume", "VOLUME", 17, 100),
                    knob("drive", "DRIVE", 18, 100),
                    knob("tone", "TONE", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x47,
            EffectOption {
                id: 0x47,
                title: "EAT Dist",
                knobs: vec![
                    knob("distortion", "DISTORTION", 17, 100),
                    knob("filter", "FILTER", 18, 100),
                    knob("volume", "VOLUME", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x48,
            EffectOption {
                id: 0x48,
                title: "Red Dirt",
                knobs: vec![
                    knob("drive", "DRIVE", 17, 100),
                    knob("tone", "TONE", 18, 100),
                    knob("level", "LEVEL", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x49,
            EffectOption {
                id: 0x49,
                title: "Crunch",
                knobs: vec![
                    knob("volume", "VOLUME", 17, 100),
                    knob("tone", "TONE", 18, 100),
                    knob("gain", "GAIN", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x4A,
            EffectOption {
                id: 0x4A,
                title: "Muff Fuzz",
                knobs: vec![
                    knob("volume", "VOLUME", 17, 100),
                    knob("tone", "TONE", 18, 100),
                    knob("sustain", "SUSTAIN", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x4B,
            EffectOption {
                id: 0x4B,
                title: "Katana",
                knobs: vec![
                    knob("boost_sw", "BOOST SW", 17, 1),
                    knob("volume", "VOLUME", 18, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x4C,
            EffectOption {
                id: 0x4C,
                title: "Red Fuzz",
                // knobs not yet confirmed from hardware
                knobs: vec![],
                ..Default::default()
            },
        ),
        (
            0x4D,
            EffectOption {
                id: 0x4D,
                title: "Touch Wah",
                // knobs not yet confirmed from hardware
                knobs: vec![],
                ..Default::default()
            },
        ),
    ])
});
