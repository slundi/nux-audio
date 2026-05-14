// IR (Impulse Response / Cabinet) models for MG-30
// Reference: ir.rs device observations + effects.yaml
//
// Bypass CC: 9
// Shared knob CCs (electric guitar cabs):
//   CC64=MIC  CC65=MIC AXIS  CC66=LEVEL  CC67=LO CUT  CC68=HI CUT
// Bass/acoustic cabs skip MIC and MIC AXIS (no_mic=true in effects.yaml).
//
// IDs are provisional — sequential from 0x00.

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

fn electric_knobs() -> Vec<Knob<'static>> {
    vec![
        knob("mic", "MIC", 64, 7), // DYN421/S57/U87/R122/R121/C414/C3000/B52
        knob("mic_axis", "MIC AXIS", 65, 2), // CENTER/MIDDLE/EDGE
        knob("level", "LEVEL", 66, 100),
        knob("lo_cut", "LO CUT", 67, 100),
        knob("hi_cut", "HI CUT", 68, 100),
    ]
}

fn other_knobs() -> Vec<Knob<'static>> {
    vec![
        knob("level", "LEVEL", 66, 100),
        knob("lo_cut", "LO CUT", 67, 100),
        knob("hi_cut", "HI CUT", 68, 100),
    ]
}

/// MG-30 IR/cabinet model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption<'static>>> = LazyLock::new(|| {
    // Electric guitar cabs
    let mut map: HashMap<u8, EffectOption<'static>> = HashMap::from([
        (
            0x00,
            EffectOption {
                id: 0x00,
                title: "JZ 120",
                knobs: electric_knobs(),
                ..Default::default()
            },
        ),
        (
            0x01,
            EffectOption {
                id: 0x01,
                title: "DR112",
                knobs: electric_knobs(),
                ..Default::default()
            },
        ),
        (
            0x02,
            EffectOption {
                id: 0x02,
                title: "BS410",
                knobs: electric_knobs(),
                ..Default::default()
            },
        ),
        (
            0x03,
            EffectOption {
                id: 0x03,
                title: "A212",
                knobs: electric_knobs(),
                ..Default::default()
            },
        ),
        (
            0x04,
            EffectOption {
                id: 0x04,
                title: "TR212",
                knobs: electric_knobs(),
                ..Default::default()
            },
        ),
        (
            0x05,
            EffectOption {
                id: 0x05,
                title: "1960",
                knobs: electric_knobs(),
                ..Default::default()
            },
        ),
        (
            0x06,
            EffectOption {
                id: 0x06,
                title: "GB412",
                knobs: electric_knobs(),
                ..Default::default()
            },
        ),
        (
            0x07,
            EffectOption {
                id: 0x07,
                title: "V412",
                knobs: electric_knobs(),
                ..Default::default()
            },
        ),
    ]);

    // Bass cabs (no mic/axis knobs)
    map.extend([
        (
            0x08,
            EffectOption {
                id: 0x08,
                title: "AGL DB810",
                knobs: other_knobs(),
                ..Default::default()
            },
        ),
        (
            0x09,
            EffectOption {
                id: 0x09,
                title: "AMP SV810",
                knobs: other_knobs(),
                ..Default::default()
            },
        ),
        (
            0x0A,
            EffectOption {
                id: 0x0A,
                title: "AMP SV410",
                knobs: other_knobs(),
                ..Default::default()
            },
        ),
        (
            0x0B,
            EffectOption {
                id: 0x0B,
                title: "AMP SV212",
                knobs: other_knobs(),
                ..Default::default()
            },
        ),
        (
            0x0C,
            EffectOption {
                id: 0x0C,
                title: "MKB 410",
                knobs: other_knobs(),
                ..Default::default()
            },
        ),
        (
            0x0D,
            EffectOption {
                id: 0x0D,
                title: "TRC 410",
                knobs: other_knobs(),
                ..Default::default()
            },
        ),
        (
            0x0E,
            EffectOption {
                id: 0x0E,
                title: "EDEN 410",
                knobs: other_knobs(),
                ..Default::default()
            },
        ),
        (
            0x0F,
            EffectOption {
                id: 0x0F,
                title: "Bassguy 410",
                knobs: other_knobs(),
                ..Default::default()
            },
        ),
    ]);

    // Acoustic cabs (no mic/axis knobs)
    map.extend([
        (
            0x10,
            EffectOption {
                id: 0x10,
                title: "M-D45",
                knobs: other_knobs(),
                ..Default::default()
            },
        ),
        (
            0x11,
            EffectOption {
                id: 0x11,
                title: "G-HBird",
                knobs: other_knobs(),
                ..Default::default()
            },
        ),
        (
            0x12,
            EffectOption {
                id: 0x12,
                title: "G-J15",
                knobs: other_knobs(),
                ..Default::default()
            },
        ),
    ]);

    // User IR slot
    map.insert(
        0x13,
        EffectOption {
            id: 0x13,
            title: "User IR",
            knobs: other_knobs(),
            ..Default::default()
        },
    );

    map
});
