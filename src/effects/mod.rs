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
    // pub id: Option<String>,
    pub title: &'a str,
    pub on_byte: u8,
    pub off_byte: u8,
    pub category_color: u32,
    pub knobs: Vec<Knob<'a>>,
    // pub options: Vec<EffectOption>, // for dominant color?
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
pub struct Knob<'a> {
    pub id: &'a str,
    pub title: &'a str,

    pub range_start: Option<u8>, // Always 0
    pub range_end: Option<u8>,

    pub ctrl: u8,
    pub current_value: Option<u8>, // use Cell? if knob instances are static?
}

pub struct Wah<'a> {
    // pub id: String,
    pub title: &'a str,
    pub on_byte: u8,
    pub off_byte: u8,
    pub knobs: Vec<Knob<'a>>,
}

// const DEFAULT_GATE: EffectOption = EffectOption {
//     id: Some("DEFAULT_GATE".to_owned()),
//     title: "Default Gate".to_owned(),
//     on_byte: 0x01,
//     off_byte: 0x41,
//     category_color: 0, //TODO:
//     knobs: vec![
//         Knob {
//             id: "sens",
//             title: "SENS",
//             range_start: 0,
//             range_end: 100,
//             ctrl: 44,
//             current_value: None,
//         },
//         Knob {
//             id: "decay",
//             title: "DECAY",
//             range_start: 0,
//             range_end: 100,
//             ctrl: 45,
//             current_value: None,
//         },
//     ],
//     dominant_color: Some(0x9ccc65),
// };

// We return Option<MatchedEffect> because the effect might not be found.
// We take `effect_option` by reference to avoid taking ownership.
// pub fn get_main_effect_group<'a>(effect_option: &'a EffectOption<'a>) -> Option<&Effect<'a>> {
//     // Look up the category in EFFECT_CONFIG
//     let effect_category_config = EFFECT_CONFIG.get(&effect_option.category)?;
//     // `?` here propagates `None` if the category is not found.
//     // If found, `effect_category_config` is an `&EffectCategoryConfig`.

//     // Then, search within the options for a matching ID
//     effect_category_config
//         .options
//         .iter() // Iterate over references to MatchedEffect
//         .find(|opt| opt.id == effect_option.id)
//         // `find` returns `Option<Effect>`, which is exactly what we want.
// }

pub fn extract_effects(response: &[u8]) -> Effect {
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
