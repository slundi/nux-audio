// DELAY models for MG-30
// Reference: delay.rs device observations + effects.yaml
//
// Bypass CC: 7
// Knob CCs: KNOB1-8 = CC52..CC59
//
// IDs are provisional — sequential from 0x20.

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

/// MG-30 delay model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption<'static>>> = LazyLock::new(|| {
    HashMap::from([
        (
            0x20,
            EffectOption {
                id: 0x20,
                title: "Analog Delay",
                knobs: vec![
                    knob("repeat", "REPEAT", 52, 100),
                    knob("echo", "ECHO", 53, 100),
                    knob("intens", "INTENSITY", 54, 100),
                    knob("subdiv", "SUB DIV", 55, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x21,
            EffectOption {
                id: 0x21,
                title: "Digi Delay",
                knobs: vec![
                    knob("level", "EFFECT LEVEL", 52, 100),
                    knob("fback", "FEEDBACK", 53, 100),
                    knob("time", "DELAY TIME", 54, 100),
                    knob("subdiv", "SUB DIV", 55, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x22,
            EffectOption {
                id: 0x22,
                title: "Modulation",
                knobs: vec![
                    knob("time", "TIME", 52, 100),
                    knob("level", "LEVEL", 53, 100),
                    knob("mod", "MODULATION", 54, 100),
                    knob("repeat", "REPEAT", 55, 100),
                    knob("subdiv", "SUB DIV", 56, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x23,
            EffectOption {
                id: 0x23,
                title: "Tape Echo",
                knobs: vec![
                    knob("time", "TIME", 52, 100),
                    knob("level", "LEVEL", 53, 100),
                    knob("repeat", "REPEAT", 54, 100),
                    knob("subdiv", "SUB DIV", 55, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x24,
            EffectOption {
                id: 0x24,
                title: "Reverse Delay",
                knobs: vec![
                    knob("time", "TIME", 52, 100),
                    knob("mix", "MIX", 53, 100),
                    knob("fback", "FEEDBACK", 54, 100),
                    knob("subdiv", "SUB DIV", 55, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x25,
            EffectOption {
                id: 0x25,
                title: "Pan Delay",
                knobs: vec![
                    knob("time", "TIME", 52, 100),
                    knob("repeat", "REPEAT", 53, 100),
                    knob("level", "DELAY LEVEL", 54, 100),
                    knob("subdiv", "SUB DIV", 55, 6),
                ],
                ..Default::default()
            },
        ),
        (
            0x26,
            EffectOption {
                id: 0x26,
                title: "Duotime",
                knobs: vec![
                    knob("level", "LEVEL", 52, 100),
                    knob("time1", "TIME1", 53, 100),
                    knob("subdiv1", "SUB DIV1", 54, 6),
                    knob("repeat1", "REPEAT1", 55, 100),
                    knob("time2", "TIME2", 56, 100),
                    knob("subdiv2", "SUB DIV2", 57, 6),
                    knob("repeat2", "REPEAT2", 58, 100),
                ],
                ..Default::default()
            },
        ),
    ])
});
