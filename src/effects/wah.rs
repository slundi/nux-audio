// WAH models for MG-30
// Reference: wah.rs device observations + effects.yaml
//
// on_byte = 2 * model_id  (device byte when slot is active)
// off_byte = 0x40 + model_id  (device byte when slot is bypassed)
//
// IDs are provisional — inferred from the on_byte pattern.

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

/// MG-30 wah model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption<'static>>> = LazyLock::new(|| {
    HashMap::from([
        (
            0x01,
            EffectOption {
                id: 0x01,
                title: "Clyde",
                on_byte: 0x02,
                off_byte: 0x41,
                knobs: vec![],
                ..Default::default()
            },
        ),
        (
            0x02,
            EffectOption {
                id: 0x02,
                title: "Cry BB",
                on_byte: 0x04,
                off_byte: 0x42,
                knobs: vec![],
                ..Default::default()
            },
        ),
        (
            0x03,
            EffectOption {
                id: 0x03,
                title: "V847",
                on_byte: 0x06,
                off_byte: 0x43,
                knobs: vec![],
                ..Default::default()
            },
        ),
        (
            0x04,
            EffectOption {
                id: 0x04,
                title: "Horse Wah",
                on_byte: 0x08,
                off_byte: 0x44,
                knobs: vec![
                    knob("contour", "CONTOUR", 11, 100),
                    knob("range", "RANGE", 12, 5),
                ],
                ..Default::default()
            },
        ),
        (
            0x05,
            EffectOption {
                id: 0x05,
                title: "Octave Shift",
                on_byte: 0x0A,
                off_byte: 0x45,
                knobs: vec![knob("direction", "UP/DOWN", 12, 1)],
                ..Default::default()
            },
        ),
    ])
});
