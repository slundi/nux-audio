// COMPRESSOR models for MG-30
// Reference: compression.rs device observations + effects.yaml
//
// Bypass CC: 1
// Knob CCs: KNOB1=13, KNOB2=14, KNOB3=15, KNOB4=90
//
// IDs are provisional.

use std::{collections::HashMap, sync::LazyLock};

use super::{EffectOption, KnobDef};

fn knob(id: &'static str, title: &'static str, cc: u8, end: u8) -> KnobDef<'static> {
    KnobDef {
        id,
        title,
        range_start: Some(0),
        range_end: Some(end),
        ctrl: cc,
    }
}

/// MG-30 compressor model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption<'static>>> = LazyLock::new(|| {
    HashMap::from([
        (
            0x00,
            EffectOption {
                id: 0x00,
                title: "Rose Comp",
                knobs: vec![
                    knob("sustain", "SUSTAIN", 13, 100),
                    knob("level", "LEVEL", 14, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x01,
            EffectOption {
                id: 0x01,
                title: "K Comp",
                knobs: vec![
                    knob("sustain", "SUSTAIN", 13, 100),
                    knob("level", "LEVEL", 14, 100),
                    knob("clipping", "CLIPPING", 15, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x02,
            EffectOption {
                id: 0x02,
                title: "Studio Comp",
                knobs: vec![
                    knob("threshold", "THRESHOLD", 13, 100),
                    knob("ratio", "RATIO", 14, 100),
                    knob("gain", "GAIN", 15, 100),
                    knob("release", "RELEASE", 90, 100),
                ],
                ..Default::default()
            },
        ),
    ])
});
