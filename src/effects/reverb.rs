// REVERB models for MG-30
// Reference: reverb.rs device observations + effects.yaml
//
// Bypass CC: 8
// Knob CCs: KNOB1-4 = CC60..CC63
//
// Model IDs confirmed from hardware byte 21 of detail SysEx response.
// Damp (0x0C) confirmed from hardware.

use std::{collections::HashMap, sync::LazyLock};

use super::{EffectOption, FirmwareGen, KnobDef};

fn knob(id: &'static str, title: &'static str, cc: u8) -> KnobDef<'static> {
    KnobDef {
        id,
        title,
        range_start: Some(0),
        range_end: Some(100),
        ctrl: cc,
    }
}

/// MG-30 reverb model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption<'static>>> = LazyLock::new(|| {
    HashMap::from([
        (
            0x00,
            EffectOption {
                id: 0x00,
                title: "Room",
                knobs: vec![
                    knob("decay", "DECAY", 60),
                    knob("tone", "TONE", 61),
                    knob("level", "LEVEL", 62),
                ],
                ..Default::default()
            },
        ),
        (
            0x04,
            EffectOption {
                id: 0x04,
                title: "Hall",
                knobs: vec![
                    knob("decay", "DECAY", 60),
                    knob("predelay", "PRE DELAY", 61),
                    knob("liveness", "LIVELINESS", 62),
                    knob("level", "LEVEL", 63),
                ],
                ..Default::default()
            },
        ),
        (
            0x06,
            EffectOption {
                id: 0x06,
                title: "Plate",
                knobs: vec![knob("decay", "DECAY", 60), knob("level", "LEVEL", 61)],
                ..Default::default()
            },
        ),
        (
            0x08,
            EffectOption {
                id: 0x08,
                title: "Spring",
                knobs: vec![knob("decay", "DECAY", 60), knob("level", "LEVEL", 61)],
                ..Default::default()
            },
        ),
        (
            0x0A,
            EffectOption {
                id: 0x0A,
                title: "Shimmer",
                knobs: vec![
                    knob("mix", "MIX", 60),
                    knob("decay", "DECAY", 61),
                    knob("shim", "SHIM", 62),
                ],
                ..Default::default()
            },
        ),
        (
            0x0C,
            EffectOption {
                id: 0x0C,
                title: "Damp",
                min_gen: FirmwareGen::Gen2,
                knobs: vec![knob("mix", "MIX", 60), knob("depth", "DEPTH", 61)],
                ..Default::default()
            },
        ),
    ])
});
