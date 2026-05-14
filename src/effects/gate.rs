// GATE model for MG-30 — single model (Noise Gate)
// Reference: gate.rs device observations + effects.yaml
//
// Bypass CC: 5
// Knob CCs: KNOB1=42 (SENSITIVITY), KNOB2=43 (DECAY)
//
// ID is provisional.

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

/// MG-30 gate model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption<'static>>> = LazyLock::new(|| {
    HashMap::from([(
        0x00,
        EffectOption {
            id: 0x00,
            title: "Noise Gate",
            knobs: vec![knob("sens", "SENSITIVITY", 42), knob("decay", "DECAY", 43)],
            ..Default::default()
        },
    )])
});
