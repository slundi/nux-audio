//! EFX (pre-amp drive/boost/fuzz) models for MG-30
// Reference: efx.rs device observations + effects.yaml
//
// Bypass CC: 2
// Knob CCs: KNOB1=17, KNOB2=18, KNOB3=19, KNOB4=20, KNOB5=21
//
// IDs 0x40–0x4B confirmed from hardware reverse-engineering.
// IDs 0x4C–0x4D provisional — knob labels inferred from effect type,
// CC numbers are confirmed (all EFX knobs share CC17–21).

use std::{collections::HashMap, sync::LazyLock};

use super::{EffectOption, FirmwareGen, KnobDef};

fn knob(id: &str, title: &str, cc: u8, end: u8) -> KnobDef {
    KnobDef {
        id: id.to_owned(),
        title: title.to_owned(),
        range_start: Some(0),
        range_end: Some(end),
        ctrl: cc,
    }
}

/// MG-30 EFX model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption>> = LazyLock::new(|| {
    HashMap::from([
        (
            0x40,
            EffectOption {
                id: 0x40,
                title: "Dist+".into(),
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
                title: "RC Boost".into(),
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
                title: "AC Boost".into(),
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
                title: "Dist One".into(),
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
                title: "T Scream".into(),
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
                title: "Blues Drv".into(),
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
                title: "Morning Drv".into(),
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
                title: "EAT Dist".into(),
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
                title: "Red Dirt".into(),
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
                title: "Crunch".into(),
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
                title: "Muff Fuzz".into(),
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
                title: "Katana".into(),
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
                title: "Red Fuzz".into(),
                // Silicon fuzz — knob labels provisional, CCs confirmed (EFX slot CC17–19)
                knobs: vec![
                    knob("volume", "VOLUME", 17, 100),
                    knob("tone", "TONE", 18, 100),
                    knob("fuzz", "FUZZ", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x4D,
            EffectOption {
                id: 0x4D,
                title: "Touch Wah".into(),
                // Envelope auto-wah — knob labels provisional, CCs confirmed (EFX slot CC17–19)
                knobs: vec![
                    knob("sens", "SENSITIVITY", 17, 100),
                    knob("range", "RANGE", 18, 100),
                    knob("level", "LEVEL", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x4E,
            EffectOption {
                id: 0x4E,
                title: "Full-OD HP".into(),
                min_gen: FirmwareGen::Gen2,
                // ID provisional — knob order from device UI
                knobs: vec![
                    knob("volume", "VOLUME", 17, 100),
                    knob("drive", "DRIVE", 18, 100),
                    knob("tone", "TONE", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x4F,
            EffectOption {
                id: 0x4F,
                title: "Horseman OD".into(),
                min_gen: FirmwareGen::Gen2,
                // ID provisional — knob order from device UI
                knobs: vec![
                    knob("gain", "GAIN", 17, 100),
                    knob("treble", "TREBLE", 18, 100),
                    knob("output", "OUTPUT", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x50,
            EffectOption {
                id: 0x50,
                title: "Nob ODR".into(),
                min_gen: FirmwareGen::Gen2,
                // ID provisional — knob order from device UI
                knobs: vec![
                    knob("drive", "DRIVE", 17, 100),
                    knob("level", "LEVEL", 18, 100),
                    knob("spectrum", "SPECTRUM", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x51,
            EffectOption {
                id: 0x51,
                title: "Riv Bluefog".into(),
                min_gen: FirmwareGen::Gen2,
                // ID provisional — knob order from device UI
                knobs: vec![
                    knob("level", "LEVEL", 17, 100),
                    knob("gain", "GAIN", 18, 100),
                    knob("tone", "TONE", 19, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x52,
            EffectOption {
                id: 0x52,
                title: "Turbo OD".into(),
                min_gen: FirmwareGen::Gen2,
                // ID provisional — knob order from device UI
                knobs: vec![
                    knob("level", "LEVEL", 17, 100),
                    knob("tone", "TONE", 18, 100),
                    knob("drive", "DRIVE", 19, 100),
                ],
                ..Default::default()
            },
        ),
    ])
});
