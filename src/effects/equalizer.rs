// EQ models for MG-30
// Reference: equalizer.rs device observations + effects.yaml
//
// Bypass CC: 4
// Knob CCs: KNOB1-12 = CC30..CC41
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

/// MG-30 EQ model catalog, keyed by model ID.
pub static MODELS: LazyLock<HashMap<u8, EffectOption<'static>>> = LazyLock::new(|| {
    HashMap::from([
        (
            0x00,
            EffectOption {
                id: 0x00,
                title: "6-Band EQ",
                knobs: vec![
                    knob("hz100", "100Hz", 30, 100),
                    knob("hz220", "220Hz", 31, 100),
                    knob("hz500", "500Hz", 32, 100),
                    knob("khz12", "1.2kHz", 33, 100),
                    knob("khz26", "2.6kHz", 34, 100),
                    knob("khz64", "6.4kHz", 35, 100),
                    knob("level", "LEVEL", 36, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x01,
            EffectOption {
                id: 0x01,
                title: "Align EQ",
                knobs: vec![
                    knob("hpf", "HPF", 30, 2), // 0=50Hz 1=80Hz 2=160Hz
                    knob("hz110", "110Hz", 31, 100),
                    knob("hz340", "340Hz", 32, 100),
                    knob("hz660", "660Hz", 33, 100),
                    knob("khz13", "1.3kHz", 34, 100),
                    knob("khz26", "2.6kHz", 35, 100),
                    knob("khz50", "5.0kHz", 36, 100),
                    knob("volume", "VOLUME", 37, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x02,
            EffectOption {
                id: 0x02,
                title: "10-Band EQ",
                knobs: vec![
                    knob("vol", "VOL", 30, 100),
                    knob("hz31", "31Hz", 31, 100),
                    knob("hz62", "62Hz", 32, 100),
                    knob("hz125", "125Hz", 33, 100),
                    knob("hz250", "250Hz", 34, 100),
                    knob("hz500", "500Hz", 35, 100),
                    knob("khz1", "1kHz", 36, 100),
                    knob("khz2", "2kHz", 37, 100),
                    knob("khz4", "4kHz", 38, 100),
                    knob("khz8", "8kHz", 39, 100),
                    knob("khz16", "16kHz", 40, 100),
                    knob("gain", "GAIN", 41, 100),
                ],
                ..Default::default()
            },
        ),
        (
            0x03,
            EffectOption {
                id: 0x03,
                title: "Para EQ",
                knobs: vec![
                    knob("b1_freq", "BAND1 FREQ", 30, 100),
                    knob("b1_gain", "BAND1 GAIN", 31, 100),
                    knob("b1_q", "BAND1 Q", 32, 100),
                    knob("b2_freq", "BAND2 FREQ", 33, 100),
                    knob("b2_gain", "BAND2 GAIN", 34, 100),
                    knob("b2_q", "BAND2 Q", 35, 100),
                    knob("b3_freq", "BAND3 FREQ", 36, 100),
                    knob("b3_gain", "BAND3 GAIN", 37, 100),
                    knob("b3_q", "BAND3 Q", 38, 100),
                ],
                ..Default::default()
            },
        ),
    ])
});
