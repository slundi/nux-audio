use anyhow::{anyhow, Result};

pub mod amp;
pub mod compression;
pub mod delay;
mod drums;
pub mod efx;
pub mod equalizer;
pub mod gate;
pub mod ir;
pub mod modulation;
pub mod reverb;
pub mod volume;
pub mod wah;

#[derive(Default, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum EffectCategory {
    Wah = 0,
    Comp = 1,
    Efx = 2,
    Amp = 3,
    Eq = 4,
    Gate = 5,
    Mod = 6,
    Delay = 7,
    Reverb = 8,
    Ir = 9,
    Sr = 10,
    #[default]
    Vol = 11,
}

impl EffectCategory {
    /// Function to convert a u8 to EffectCategory
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(EffectCategory::Wah),
            1 => Some(EffectCategory::Comp),
            2 => Some(EffectCategory::Efx),
            3 => Some(EffectCategory::Amp),
            4 => Some(EffectCategory::Eq),
            5 => Some(EffectCategory::Gate),
            6 => Some(EffectCategory::Mod),
            7 => Some(EffectCategory::Delay),
            8 => Some(EffectCategory::Reverb),
            9 => Some(EffectCategory::Ir),
            10 => Some(EffectCategory::Sr),
            11 => Some(EffectCategory::Vol),
            _ => None, // Handle any value that doesn't match an enum variant
        }
    }
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct EffectOption<'a> {
    /// Model ID as transmitted in SysEx messages.
    /// IDs marked [confirmed] in effects.yaml are from hardware reverse-engineering.
    /// IDs marked [provisional] are inferred from sequential ordering.
    pub id: u8,
    pub title: &'a str,
    pub on_byte: u8,
    pub off_byte: u8,
    pub category_color: u32,
    pub knobs: Vec<KnobDef<'a>>,
    pub dominant_color: Option<u32>,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Effect<'a> {
    // pub byte: Option<u8>,
    pub category: EffectCategory,
    pub start_off_byte: u8,
    pub start_on_byte: u8,
    pub category_color: u32,
    pub options: Vec<EffectOption<'a>>,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct KnobDef<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub range_start: Option<u8>, // Always 0
    pub range_end: Option<u8>,
    pub ctrl: u8,
}

/// Runtime state for a single knob (current value received from device).
#[derive(Clone, PartialEq, Debug)]
pub struct KnobState {
    pub def_id: &'static str,
    pub value: u8,
}

pub fn extract_effects(response: &[u8]) -> Effect<'_> {
    todo!()
}

pub fn extract_effects_order(data: &[u8]) -> Result<Vec<EffectCategory>> {
    let sliced = &data[147..165];
    let mut effects: Vec<EffectCategory> = Vec::with_capacity(11);
    let mut i = 0usize;
    while i < sliced.len() {
        if i.is_multiple_of(3) {
            // Process single byte (like 1st, 4th, 7th, etc.)
            match EffectCategory::from_u8(data[i]) {
                Some(ec) => effects.push(ec),
                None => return Err(anyhow!("Cannot get effect category with {}", data[i])),
            }
            i += 1; // Move to the next index
        } else {
            // Process byte pair (like 2nd and 3rd, 5th and 6th, etc.)
            let combined = u16::from_be_bytes([sliced[i], sliced[i + 1]]) / 2;
            match EffectCategory::from_u8(combined as u8) {
                Some(ec) => effects.push(ec),
                None => return Err(anyhow!("Cannot get effect category with {}", combined)), // Unknown
            }
            i += 2; // Move forward by 2 to process the next pair of bytes
        }
    }
    Ok(effects)
}
