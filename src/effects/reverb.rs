// REVERB models for MG-30
// Reference: reverb.rs device observations + effects.yaml
//
// Bypass CC: 8
// Knob CCs: KNOB1-4 = CC60..CC63
//
// IDs are provisional — sequential from 0x30.

use std::{collections::HashMap, sync::LazyLock};

use super::{EffectOption, Knob};

fn knob(id: &'static str, title: &'static str, cc: u8) -> Knob<'static> {
    Knob {
        id,
        title,
        range_start: Some(0),
        range_end: Some(100),
        ctrl: cc,
        current_value: None,
    }
}

/// MG-30 reverb model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption<'static>>> = LazyLock::new(|| {
    HashMap::from([
        (
            0x30,
            EffectOption {
                id: 0x30,
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
            0x31,
            EffectOption {
                id: 0x31,
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
            0x32,
            EffectOption {
                id: 0x32,
                title: "Plate",
                knobs: vec![knob("decay", "DECAY", 60), knob("level", "LEVEL", 61)],
                ..Default::default()
            },
        ),
        (
            0x33,
            EffectOption {
                id: 0x33,
                title: "Spring",
                knobs: vec![knob("decay", "DECAY", 60), knob("level", "LEVEL", 61)],
                ..Default::default()
            },
        ),
        (
            0x34,
            EffectOption {
                id: 0x34,
                title: "Shimmer",
                knobs: vec![
                    knob("mix", "MIX", 60),
                    knob("decay", "DECAY", 61),
                    knob("shim", "SHIM", 62),
                ],
                ..Default::default()
            },
        ),
    ])
});
