// VOL / Send-Return slot for MG-30
// Reference: effects.yaml
//
// Bypass CC: 10 (Send/Return)
// Knob CCs: PATCH VOLUME=CC72, SEND=CC70, RETURN=CC71

use super::KnobDef;

fn knob(id: &'static str, title: &'static str, cc: u8) -> KnobDef<'static> {
    KnobDef {
        id,
        title,
        range_start: Some(0),
        range_end: Some(100),
        ctrl: cc,
    }
}

/// Volume/Send-Return knob definitions for MG-30.
/// This slot has no selectable models — the knobs are always available.
pub fn knobs() -> Vec<KnobDef<'static>> {
    vec![
        knob("patch_vol", "PATCH VOLUME", 72),
        knob("send", "SEND", 70),
        knob("return_", "RETURN", 71),
    ]
}
