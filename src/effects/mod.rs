use crate::error::NuxError;

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

/// Parse a MG-30 `CURRENT_PRESET_BASIC` SysEx response into an `Effect`.
///
/// Expected response header: `[0xF0, 0x43, 0x58, 0x70, 0x15, 0x02, ...]`
///
/// Byte offsets are provisional and must be verified against hardware captures:
///   `CATEGORY_OFFSET` — byte encoding the effect slot (maps to `EffectCategory`)
///   `MODEL_ID_OFFSET` — byte encoding the selected model within that slot
pub fn extract_effects(response: &[u8]) -> Result<Effect<'static>, NuxError> {
    // TODO: verify these offsets from hardware SysEx captures
    const CATEGORY_OFFSET: usize = 6;
    const MODEL_ID_OFFSET: usize = 7;

    if response.len() <= MODEL_ID_OFFSET {
        return Err(NuxError::EffectResponseTooShort {
            expected: MODEL_ID_OFFSET + 1,
            actual: response.len(),
        });
    }

    let category_byte = response[CATEGORY_OFFSET];
    let model_id = response[MODEL_ID_OFFSET];

    let category = EffectCategory::from_u8(category_byte)
        .ok_or(NuxError::UnknownEffectCategory(category_byte))?;

    let option = match &category {
        EffectCategory::Wah => wah::MODELS.get(&model_id),
        EffectCategory::Comp => compression::MODELS.get(&model_id),
        EffectCategory::Efx => efx::MODELS.get(&model_id),
        EffectCategory::Amp => amp::MODELS.get(&model_id),
        EffectCategory::Eq => equalizer::MODELS.get(&model_id),
        EffectCategory::Gate => gate::MODELS.get(&model_id),
        EffectCategory::Mod => modulation::MODELS.get(&model_id),
        EffectCategory::Delay => delay::MODELS.get(&model_id),
        EffectCategory::Reverb => reverb::MODELS.get(&model_id),
        EffectCategory::Ir => ir::MODELS.get(&model_id),
        // Vol/Sr have no model catalog — knobs are always available
        EffectCategory::Vol | EffectCategory::Sr => None,
    }
    .ok_or(NuxError::UnknownEffectModel {
        category_byte,
        model_id,
    })?;

    Ok(Effect {
        category,
        options: vec![option.clone()],
        ..Default::default()
    })
}

pub fn extract_effects_order(data: &[u8]) -> Result<Vec<EffectCategory>, NuxError> {
    let sliced = &data[147..165];
    let mut effects: Vec<EffectCategory> = Vec::with_capacity(11);
    let mut i = 0usize;
    while i < sliced.len() {
        if i.is_multiple_of(3) {
            // Process single byte (like 1st, 4th, 7th, etc.)
            match EffectCategory::from_u8(data[i]) {
                Some(ec) => effects.push(ec),
                None => return Err(NuxError::UnknownEffectCategory(data[i])),
            }
            i += 1; // Move to the next index
        } else {
            // Process byte pair (like 2nd and 3rd, 5th and 6th, etc.)
            let combined = u16::from_be_bytes([sliced[i], sliced[i + 1]]) / 2;
            match EffectCategory::from_u8(combined as u8) {
                Some(ec) => effects.push(ec),
                None => return Err(NuxError::UnknownEffectCategory(combined as u8)),
            }
            i += 2; // Move forward by 2 to process the next pair of bytes
        }
    }
    Ok(effects)
}
