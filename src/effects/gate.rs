// GATE model for MG-30 — single model (Noise Gate)
// Reference: gate.rs device observations + effects.yaml
//
// Bypass CC: 5
// Knob CCs: KNOB1=42 (SENSITIVITY), KNOB2=43 (DECAY)
//
// ID is provisional.

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

/// MG-30 gate model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption>> = LazyLock::new(|| {
    HashMap::from([(
        0x00,
        EffectOption {
            id: 0x00,
            title: "Noise Gate".into(),
            knobs: vec![knob("sens", "SENSITIVITY", 42), knob("decay", "DECAY", 43)],
            ..Default::default()
        },
    )])
});
