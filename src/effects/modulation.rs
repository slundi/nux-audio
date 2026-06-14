// MODULATION models for MG-30
// Reference: modulation.rs device observations + effects.yaml
//
// Bypass CC: 6
// Knob CCs: KNOB1-6 = CC46..CC51
//
// IDs confirmed from hardware reverse-engineering.

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

/// MG-30 modulation model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption>> = LazyLock::new(|| {
    HashMap::from([
        (
            0x10,
            EffectOption {
                id: 0x10,
                title: "CE-1".into(),
                knobs: vec![
                    knob("intensity", "INTENSITY", 46, 100),
                    knob("depth", "DEPTH", 47, 100),
                    knob("rate", "RATE", 48, 100),
                    knob("subdiv", "SUB DIV", 49, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x11,
            EffectOption {
                id: 0x11,
                title: "CE-2".into(),
                knobs: vec![
                    knob("rate", "RATE", 46, 100),
                    knob("depth", "DEPTH", 47, 100),
                    knob("subdiv", "SUB DIV", 48, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x12,
            EffectOption {
                id: 0x12,
                title: "ST. Chorus".into(),
                knobs: vec![
                    knob("intensity", "INTENSITY", 46, 100),
                    knob("width", "WIDTH", 47, 100),
                    knob("rate", "RATE", 48, 100),
                    knob("subdiv", "SUB DIV", 49, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x13,
            EffectOption {
                id: 0x13,
                title: "Vibrator".into(),
                knobs: vec![
                    knob("rate", "RATE", 46, 100),
                    knob("depth", "DEPTH", 47, 100),
                    knob("subdiv", "SUB DIV", 48, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x14,
            EffectOption {
                id: 0x14,
                title: "Detune".into(),
                knobs: vec![
                    knob("shift_l", "SHIFT LEFT", 46, 100),
                    knob("mix_l", "MIX L", 47, 100),
                    knob("shift_r", "SHIFT RIGHT", 48, 100),
                    knob("mix_r", "MIX R", 49, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x15,
            EffectOption {
                id: 0x15,
                title: "Flanger".into(),
                knobs: vec![
                    knob("level", "LEVEL", 46, 100),
                    knob("rate", "RATE", 47, 100),
                    knob("width", "WIDTH", 48, 100),
                    knob("fback", "F.BACK", 49, 100),
                    knob("subdiv", "SUB DIV", 50, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x16,
            EffectOption {
                id: 0x16,
                title: "Phase 90".into(),
                knobs: vec![
                    knob("speed", "SPEED", 46, 100),
                    knob("subdiv", "SUB DIV", 47, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x17,
            EffectOption {
                id: 0x17,
                title: "Phase 100".into(),
                knobs: vec![
                    knob("intensity", "INTENSITY", 46, 100),
                    knob("speed", "SPEED", 47, 100),
                    knob("subdiv", "SUB DIV", 48, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x18,
            EffectOption {
                id: 0x18,
                title: "S.C.F.".into(),
                knobs: vec![
                    knob("speed", "SPEED", 46, 100),
                    knob("width", "WIDTH", 47, 100),
                    knob("mode", "MODE", 48, 2), // 0=CHORUS 1=PITCH MOD 2=FLANGER
                    knob("intens", "INTENSITY", 49, 100),
                    knob("subdiv", "SUB DIV", 50, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x19,
            EffectOption {
                id: 0x19,
                title: "U-Vibe".into(),
                knobs: vec![
                    knob("speed", "SPEED", 46, 100),
                    knob("volume", "VOLUME", 47, 100),
                    knob("intens", "INTENSITY", 48, 100),
                    knob("mode", "CHORUS/VIBRATO", 49, 1),
                    knob("subdiv", "SUB DIV", 50, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x1A,
            EffectOption {
                id: 0x1A,
                title: "Tremolo".into(),
                knobs: vec![
                    knob("rate", "RATE", 46, 100),
                    knob("depth", "DEPTH", 47, 100),
                    knob("subdiv", "SUB DIV", 48, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x1B,
            EffectOption {
                id: 0x1B,
                title: "Rotar".into(),
                knobs: vec![
                    knob("balance", "BALANCE", 46, 100),
                    knob("speed", "SPEED", 47, 100),
                    knob("subdiv", "SUB DIV", 48, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x1C,
            EffectOption {
                id: 0x1C,
                title: "Mono Octave".into(),
                min_gen: FirmwareGen::Gen2,
                // ID provisional — knob order from device UI
                knobs: vec![
                    knob("sub", "SUB", 46, 100),
                    knob("dry", "DRY", 47, 100),
                    knob("up", "UP", 48, 100),
                ],
                ..Default::default()
            },
        ),
    ])
});
