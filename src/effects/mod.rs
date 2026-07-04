use crate::error::NuxError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub mod user;
pub mod volume;
pub mod wah;

/// MIDI CC value that activates an effect slot (boolean true in MIDI).
pub const CC_BYPASS_ON: u8 = 0x7F;
/// MIDI CC value that bypasses an effect slot (boolean false in MIDI).
pub const CC_BYPASS_OFF: u8 = 0x00;

/// Hardware generation of the connected MG-30.
///
/// GEN1 = firmware ≤ 4.x (218-byte detail response, original model set).
/// GEN2 = firmware ≥ 5.x (222-byte detail response, expanded model set).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub enum FirmwareGen {
    /// First generation firmware (≤ 4.x); 218-byte detail response.
    #[default]
    Gen1,
    /// Second generation firmware (≥ 5.x / version ≥ 600); 222-byte detail response.
    Gen2,
}

impl FirmwareGen {
    /// Derive generation from the version string returned by the device (e.g. `"v602"`).
    ///
    /// Strips a leading `v`, parses the remainder as an integer, and compares against
    /// the confirmed GEN2 threshold: **≥ 600** → `Gen2`, otherwise `Gen1`.
    /// Defaults to `Gen1` on any parse failure (conservative).
    pub fn from_version_str(version: &str) -> Self {
        let stripped = version.strip_prefix('v').unwrap_or(version);
        match stripped.parse::<u16>() {
            Ok(n) if n >= 600 => Self::Gen2,
            _ => Self::Gen1,
        }
    }
}

/// Signal-chain slot category for MG-30 effects.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[repr(u8)]
pub enum EffectCategory {
    /// Wah / auto-wah filter slot (CC 0).
    Wah = 0,
    /// Compressor / limiter slot (CC 1).
    Comp = 1,
    /// Effects (distortion, overdrive, boost, etc.) slot (CC 2).
    Efx = 2,
    /// Amp model slot (CC 3).
    Amp = 3,
    /// Equalizer slot (CC 4).
    Eq = 4,
    /// Noise gate slot (CC 5).
    Gate = 5,
    /// Modulation effects (chorus, flanger, phaser, etc.) slot (CC 6).
    Mod = 6,
    /// Delay / echo slot (CC 7).
    Delay = 7,
    /// Reverb slot (CC 8).
    Reverb = 8,
    /// Impulse Response (cabinet simulation) slot (CC 9).
    Ir = 9,
    /// Send/Return (effects loop) slot (CC 10); no bypass toggle.
    Sr = 10,
    /// Master volume slot (CC 11); no bypass toggle.
    #[default]
    Vol = 11,
}

impl EffectCategory {
    /// Returns the MIDI CC number that controls the bypass state of this slot.
    ///
    /// The MG-30 maps each slot's bypass toggle to a CC equal to its discriminant
    /// value (Wah=CC0, Comp=CC1, … Sr=CC10).  [`Vol`](EffectCategory::Vol) has
    /// no bypass toggle and returns `None`.
    ///
    /// Use [`CC_BYPASS_ON`] (0x7F) to activate and [`CC_BYPASS_OFF`] (0x00) to
    /// bypass the slot.  Send via [`MidiManager::send_cc`](crate::midi::MidiManager::send_cc).
    pub fn bypass_cc(self) -> Option<u8> {
        match self {
            EffectCategory::Vol => None,
            _ => Some(self as u8),
        }
    }

    /// Return the canonical display name for this category.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wah => "Wah",
            Self::Comp => "Comp",
            Self::Efx => "Efx",
            Self::Amp => "Amp",
            Self::Eq => "Eq",
            Self::Gate => "Gate",
            Self::Mod => "Mod",
            Self::Delay => "Delay",
            Self::Reverb => "Reverb",
            Self::Ir => "Ir",
            Self::Sr => "Sr",
            Self::Vol => "Vol",
        }
    }

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
            _ => None,
        }
    }
}

impl std::str::FromStr for EffectCategory {
    type Err = ();

    /// Case-insensitive parse from display name or common alias.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "wah" => Ok(Self::Wah),
            "comp" | "compression" => Ok(Self::Comp),
            "efx" => Ok(Self::Efx),
            "amp" => Ok(Self::Amp),
            "eq" | "equalizer" => Ok(Self::Eq),
            "gate" => Ok(Self::Gate),
            "mod" | "modulation" => Ok(Self::Mod),
            "delay" => Ok(Self::Delay),
            "reverb" => Ok(Self::Reverb),
            "ir" => Ok(Self::Ir),
            "sr" => Ok(Self::Sr),
            "vol" | "volume" => Ok(Self::Vol),
            _ => Err(()),
        }
    }
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct EffectOption {
    /// Model ID as transmitted in SysEx messages.
    /// IDs marked "confirmed" are from hardware reverse-engineering;
    /// IDs marked "provisional" are inferred from sequential ordering.
    pub id: u8,
    pub title: String,
    pub on_byte: u8,
    pub off_byte: u8,
    pub category_color: u32,
    pub knobs: Vec<KnobDef>,
    pub dominant_color: Option<u32>,
    /// Minimum firmware generation required for this model.
    /// Defaults to `Gen1` (available on all hardware).
    pub min_gen: FirmwareGen,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Effect {
    pub category: EffectCategory,
    pub start_off_byte: u8,
    pub start_on_byte: u8,
    pub category_color: u32,
    pub options: Vec<EffectOption>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct KnobDef {
    pub id: String,
    pub title: String,
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

/// Return the static model catalog for a category, or `None` for Vol/Sr.
fn models_map(category: EffectCategory) -> Option<&'static HashMap<u8, EffectOption>> {
    Some(match category {
        EffectCategory::Wah => &wah::MODELS,
        EffectCategory::Comp => &compression::MODELS,
        EffectCategory::Efx => &efx::MODELS,
        EffectCategory::Amp => &amp::MODELS,
        EffectCategory::Eq => &equalizer::MODELS,
        EffectCategory::Gate => &gate::MODELS,
        EffectCategory::Mod => &modulation::MODELS,
        EffectCategory::Delay => &delay::MODELS,
        EffectCategory::Reverb => &reverb::MODELS,
        EffectCategory::Ir => &ir::MODELS,
        EffectCategory::Vol | EffectCategory::Sr => return None,
    })
}

/// Look up an effect model by ID, respecting firmware generation.
///
/// Returns `None` if the model is not in the catalog, or if it requires a newer
/// generation than `gen` (i.e. `model.min_gen > gen`).
pub fn get_model(
    category: EffectCategory,
    model_id: u8,
    fw_gen: FirmwareGen,
) -> Option<&'static EffectOption> {
    // built-in first
    if let Some(opt) = models_map(category)?.get(&model_id) {
        return (opt.min_gen <= fw_gen).then_some(opt);
    }
    // user extensions
    user::get(category, model_id)
}

/// Parse a MG-30 `CURRENT_PRESET_BASIC` SysEx response into an `Effect`.
///
/// Expected response header: `[0xF0, 0x43, 0x58, 0x70, 0x15, 0x02, ...]`
///
/// Byte offsets are provisional and must be verified against hardware captures:
///   `CATEGORY_OFFSET` — byte encoding the effect slot (maps to `EffectCategory`)
///   `MODEL_ID_OFFSET` — byte encoding the selected model within that slot
pub fn extract_effects(response: &[u8], fw_gen: FirmwareGen) -> Result<Effect, NuxError> {
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

    let option = get_model(category, model_id, fw_gen).ok_or(NuxError::UnknownEffectModel {
        category_byte,
        model_id,
    })?;

    Ok(Effect {
        category,
        options: vec![option.clone()],
        ..Default::default()
    })
}

/// Parse the MG-30 signal-chain order from a full detail SysEx response.
///
/// The device encodes 11 effect slot IDs into bytes 147–164 of the response
/// using a 3-byte group format:
///   - byte at offset `3k`     → single category value
///   - bytes at offset `3k+1,3k+2` → big-endian u16 divided by 2 → category value
///
/// Requires the full (unstripped) SysEx response including the `F0 43` header.
pub fn extract_effects_order(data: &[u8]) -> Result<Vec<EffectCategory>, NuxError> {
    const SLICE_START: usize = 147;
    const SLICE_END: usize = 165;
    if data.len() < SLICE_END {
        return Err(NuxError::EffectResponseTooShort {
            expected: SLICE_END,
            actual: data.len(),
        });
    }
    let sliced = &data[SLICE_START..SLICE_END];
    let mut effects: Vec<EffectCategory> = Vec::with_capacity(11);
    let mut i = 0usize;
    while i < sliced.len() {
        if i.is_multiple_of(3) {
            // Single-byte slot at positions 0, 3, 6, 9, 12, 15
            match EffectCategory::from_u8(sliced[i]) {
                Some(ec) => effects.push(ec),
                None => return Err(NuxError::UnknownEffectCategory(sliced[i])),
            }
            i += 1;
        } else {
            // Two-byte slot at positions 1-2, 4-5, 7-8, 10-11, 13-14, 16-17
            let combined = u16::from_be_bytes([sliced[i], sliced[i + 1]]) / 2;
            match EffectCategory::from_u8(combined as u8) {
                Some(ec) => effects.push(ec),
                None => return Err(NuxError::UnknownEffectCategory(combined as u8)),
            }
            i += 2;
        }
    }
    Ok(effects)
}

/// Look up an effect model by name (case-insensitive) within a category's static catalog,
/// filtered by firmware generation.
///
/// Returns `(model_id, &EffectOption)` for the first model whose `title` matches
/// `name` and whose `min_gen <= gen`.  Returns `None` when the category has no
/// catalog (`Vol`, `Sr`) or when no matching model title is found.
pub fn find_model_by_name(
    category: EffectCategory,
    name: &str,
    fw_gen: FirmwareGen,
) -> Option<(u8, &'static EffectOption)> {
    // built-in first
    if let Some(result) = models_map(category)?
        .iter()
        .filter(|(_, opt)| opt.min_gen <= fw_gen)
        .find(|(_, opt)| opt.title.eq_ignore_ascii_case(name))
        .map(|(k, v)| (*k, v))
    {
        return Some(result);
    }
    // user extensions
    user::find_by_name(category, name, fw_gen)
}

/// Find the MIDI CC number for a knob by its `id` string within a category's model catalog.
///
/// Searches all models in the category and returns the first matching CC.  Within a slot,
/// the same knob id always maps to the same CC number regardless of model, so the first
/// match is authoritative.
///
/// Returns `None` when the category has no static catalog (`Vol`, `Sr`) or when no model
/// in the catalog defines a knob with the given `id`.  In those cases the caller should
/// ask the user to supply the CC number directly.
pub fn find_cc_for_knob(category: EffectCategory, knob_id: &str) -> Option<u8> {
    for opt in models_map(category)?.values() {
        if let Some(knob) = opt.knobs.iter().find(|k| k.id == knob_id) {
            return Some(knob.ctrl);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bypass_cc_matches_discriminant() {
        // Each non-Vol slot's bypass CC must equal its repr(u8) discriminant.
        assert_eq!(EffectCategory::Wah.bypass_cc(), Some(0));
        assert_eq!(EffectCategory::Comp.bypass_cc(), Some(1));
        assert_eq!(EffectCategory::Efx.bypass_cc(), Some(2));
        assert_eq!(EffectCategory::Amp.bypass_cc(), Some(3));
        assert_eq!(EffectCategory::Eq.bypass_cc(), Some(4));
        assert_eq!(EffectCategory::Gate.bypass_cc(), Some(5));
        assert_eq!(EffectCategory::Mod.bypass_cc(), Some(6));
        assert_eq!(EffectCategory::Delay.bypass_cc(), Some(7));
        assert_eq!(EffectCategory::Reverb.bypass_cc(), Some(8));
        assert_eq!(EffectCategory::Ir.bypass_cc(), Some(9));
        assert_eq!(EffectCategory::Sr.bypass_cc(), Some(10));
    }

    #[test]
    fn test_bypass_cc_vol_is_none() {
        assert_eq!(EffectCategory::Vol.bypass_cc(), None);
    }

    #[test]
    fn test_bypass_constants() {
        assert_eq!(CC_BYPASS_ON, 0x7F);
        assert_eq!(CC_BYPASS_OFF, 0x00);
    }

    #[test]
    fn test_from_u8_roundtrip() {
        for v in 0u8..=11 {
            let cat = EffectCategory::from_u8(v).expect("all values 0–11 should be valid");
            assert_eq!(cat as u8, v);
        }
        assert!(EffectCategory::from_u8(12).is_none());
    }

    // ── FirmwareGen ───────────────────────────────────────────────────────────

    #[test]
    fn test_firmware_gen_ordering() {
        assert!(FirmwareGen::Gen1 < FirmwareGen::Gen2);
    }

    #[test]
    fn test_firmware_gen_from_version_str() {
        assert_eq!(FirmwareGen::from_version_str("v602"), FirmwareGen::Gen2);
        assert_eq!(FirmwareGen::from_version_str("v600"), FirmwareGen::Gen2);
        assert_eq!(FirmwareGen::from_version_str("v599"), FirmwareGen::Gen1);
        assert_eq!(FirmwareGen::from_version_str("v500"), FirmwareGen::Gen1);
        // without 'v' prefix
        assert_eq!(FirmwareGen::from_version_str("602"), FirmwareGen::Gen2);
        // unparsable → conservative Gen1
        assert_eq!(FirmwareGen::from_version_str("invalid"), FirmwareGen::Gen1);
        assert_eq!(FirmwareGen::from_version_str(""), FirmwareGen::Gen1);
    }

    #[test]
    fn test_get_model_gen2_only_hidden_from_gen1() {
        // Damp (0x0C) is Gen2-only — must not be visible to Gen1
        assert!(get_model(EffectCategory::Reverb, 0x0C, FirmwareGen::Gen1).is_none());
        assert!(get_model(EffectCategory::Reverb, 0x0C, FirmwareGen::Gen2).is_some());
    }

    #[test]
    fn test_get_model_gen1_visible_to_gen2() {
        // Room (0x00) is Gen1 — must also be visible to Gen2
        assert!(get_model(EffectCategory::Reverb, 0x00, FirmwareGen::Gen1).is_some());
        assert!(get_model(EffectCategory::Reverb, 0x00, FirmwareGen::Gen2).is_some());
    }

    #[test]
    fn test_get_model_unknown_id() {
        assert!(get_model(EffectCategory::Reverb, 0xFF, FirmwareGen::Gen2).is_none());
    }

    #[test]
    fn test_get_model_vol_sr_none() {
        assert!(get_model(EffectCategory::Vol, 0x00, FirmwareGen::Gen2).is_none());
        assert!(get_model(EffectCategory::Sr, 0x00, FirmwareGen::Gen2).is_none());
    }

    // ── Catalog validation ────────────────────────────────────────────────────

    fn total_model_count() -> usize {
        wah::MODELS.len()
            + compression::MODELS.len()
            + efx::MODELS.len()
            + amp::MODELS.len()
            + equalizer::MODELS.len()
            + gate::MODELS.len()
            + modulation::MODELS.len()
            + delay::MODELS.len()
            + reverb::MODELS.len()
            + ir::MODELS.len()
    }

    #[test]
    fn test_catalog_has_100_plus_models() {
        let count = total_model_count();
        assert!(
            count >= 100,
            "expected ≥100 models across all slots, got {count}"
        );
    }

    /// Each model's stored id must equal its HashMap key.
    #[test]
    fn test_catalog_ids_match_keys() {
        macro_rules! check {
            ($map:expr, $slot:literal) => {
                for (key, opt) in $map.iter() {
                    assert_eq!(
                        *key, opt.id,
                        "{} model key 0x{key:02X} has mismatched id 0x{:02X}",
                        $slot, opt.id
                    );
                }
            };
        }
        check!(wah::MODELS, "Wah");
        check!(compression::MODELS, "Comp");
        check!(efx::MODELS, "Efx");
        check!(amp::MODELS, "Amp");
        check!(equalizer::MODELS, "Eq");
        check!(gate::MODELS, "Gate");
        check!(modulation::MODELS, "Mod");
        check!(delay::MODELS, "Delay");
        check!(reverb::MODELS, "Reverb");
        check!(ir::MODELS, "Ir");
    }

    /// All EFX models (including newly-filled 0x4C/0x4D) must have knobs defined.
    #[test]
    fn test_efx_all_models_have_knobs() {
        for (id, model) in efx::MODELS.iter() {
            assert!(
                !model.knobs.is_empty(),
                "EFX model 0x{id:02X} ({}) has no knobs",
                model.title
            );
        }
    }

    /// Knob CC numbers for each slot must fall within the slot's documented range.
    ///
    /// Ranges from MIDI_CC_assignments.md:
    ///   Wah CC11–12 | Comp CC13–15,90 | Efx CC17–21 | Amp CC22–29
    ///   Eq CC30–41  | Gate CC42–45    | Mod CC46–51  | Delay CC52–59
    ///   Reverb CC60–63 | Ir CC64–69  | Vol CC70–72
    fn assert_ccs_in_range(models: &HashMap<u8, EffectOption>, slot: &str, valid: &[u8]) {
        for (id, opt) in models {
            for knob in &opt.knobs {
                assert!(
                    valid.contains(&knob.ctrl),
                    "{slot} model 0x{id:02X} ({}) knob '{}' has unexpected CC {}",
                    opt.title,
                    knob.id,
                    knob.ctrl
                );
            }
        }
    }

    #[test]
    fn test_wah_knob_ccs_in_range() {
        assert_ccs_in_range(&wah::MODELS, "Wah", &[11, 12]);
    }

    #[test]
    fn test_comp_knob_ccs_in_range() {
        assert_ccs_in_range(&compression::MODELS, "Comp", &[13, 14, 15, 90]);
    }

    #[test]
    fn test_efx_knob_ccs_in_range() {
        assert_ccs_in_range(&efx::MODELS, "Efx", &[17, 18, 19, 20, 21]);
    }

    #[test]
    fn test_amp_knob_ccs_in_range() {
        assert_ccs_in_range(&amp::MODELS, "Amp", &[22, 23, 24, 25, 26, 27, 28, 29]);
    }

    #[test]
    fn test_eq_knob_ccs_in_range() {
        let eq_ccs: Vec<u8> = (30..=41).collect();
        assert_ccs_in_range(&equalizer::MODELS, "Eq", &eq_ccs);
    }

    #[test]
    fn test_gate_knob_ccs_in_range() {
        assert_ccs_in_range(&gate::MODELS, "Gate", &[42, 43, 44, 45]);
    }

    #[test]
    fn test_mod_knob_ccs_in_range() {
        let mod_ccs: Vec<u8> = (46..=51).collect();
        assert_ccs_in_range(&modulation::MODELS, "Mod", &mod_ccs);
    }

    #[test]
    fn test_delay_knob_ccs_in_range() {
        let delay_ccs: Vec<u8> = (52..=59).collect();
        assert_ccs_in_range(&delay::MODELS, "Delay", &delay_ccs);
    }

    #[test]
    fn test_reverb_knob_ccs_in_range() {
        let reverb_ccs: Vec<u8> = (60..=63).collect();
        assert_ccs_in_range(&reverb::MODELS, "Reverb", &reverb_ccs);
    }

    #[test]
    fn test_ir_knob_ccs_in_range() {
        let ir_ccs: Vec<u8> = (64..=69).collect();
        assert_ccs_in_range(&ir::MODELS, "Ir", &ir_ccs);
    }

    /// Gate must expose all four knobs from effects.yaml (SENSITIVITY, DECAY, HOLD, MODE).
    /// The permissive `assert_ccs_in_range` check does not catch missing knobs, so this
    /// test guards against silent drift between effects.yaml and the compile-time catalog.
    #[test]
    fn test_gate_has_all_documented_knobs() {
        let model = gate::MODELS.get(&0x00).expect("Noise Gate model missing");
        let ccs: Vec<u8> = model.knobs.iter().map(|k| k.ctrl).collect();
        assert!(ccs.contains(&42), "gate missing SENSITIVITY (CC42)");
        assert!(ccs.contains(&43), "gate missing DECAY (CC43)");
        assert!(ccs.contains(&44), "gate missing HOLD (CC44)");
        assert!(ccs.contains(&45), "gate missing MODE (CC45)");
    }

    /// Electric IR cabs must expose CC69 (KNOB 6); bass/acoustic cabs must NOT
    /// (they carry `no_mic: true` in effects.yaml and use `shared_other_knobs`).
    #[test]
    fn test_ir_electric_cabs_have_knob6() {
        // 0x00 (JZ 120) is an electric cab in effects.yaml.
        let electric = ir::MODELS.get(&0x00).expect("JZ 120 model missing");
        let ccs: Vec<u8> = electric.knobs.iter().map(|k| k.ctrl).collect();
        assert!(
            ccs.contains(&69),
            "electric IR cab missing KNOB 6 (CC69): {ccs:?}"
        );
    }

    #[test]
    fn test_ir_bass_cabs_have_no_knob6() {
        // 0x08 (AGL DB810) is a bass cab (no_mic: true) in effects.yaml.
        let bass = ir::MODELS.get(&0x08).expect("AGL DB810 model missing");
        let ccs: Vec<u8> = bass.knobs.iter().map(|k| k.ctrl).collect();
        assert!(
            !ccs.contains(&69),
            "bass IR cab must not have KNOB 6 (CC69): {ccs:?}"
        );
    }

    /// No two effect categories may claim the same knob CC. Same-CC-across-
    /// models-within-a-category is fine (models share knob layouts by design),
    /// but cross-category collision means one physical knob would drive two
    /// effect slots — an ambiguity the device firmware cannot resolve.
    #[test]
    fn test_no_knob_cc_collides_across_categories() {
        let mut owner: HashMap<u8, EffectCategory> = HashMap::new();
        for cat_val in 0..=11u8 {
            let category = EffectCategory::from_u8(cat_val).unwrap();
            let Some(models) = models_map(category) else {
                continue;
            };
            for opt in models.values() {
                for knob in &opt.knobs {
                    if let Some(&prev) = owner.get(&knob.ctrl) {
                        assert!(
                            prev == category,
                            "CC{} collision: {:?} and {:?} both claim it (knob '{}' in {} model 0x{:02X} '{}')",
                            knob.ctrl,
                            prev,
                            category,
                            knob.id,
                            category.as_str(),
                            opt.id,
                            opt.title,
                        );
                    } else {
                        owner.insert(knob.ctrl, category);
                    }
                }
            }
        }
    }

    /// Knob CCs must not overlap the bypass-CC range (0..=10, matching each
    /// non-Vol category's discriminant). A knob at CC5, for instance, would
    /// be interpreted by the device as a gate-bypass toggle.
    #[test]
    fn test_knob_ccs_do_not_overlap_bypass_range() {
        for cat_val in 0..=11u8 {
            let category = EffectCategory::from_u8(cat_val).unwrap();
            let Some(models) = models_map(category) else {
                continue;
            };
            for opt in models.values() {
                for knob in &opt.knobs {
                    assert!(
                        knob.ctrl > 10,
                        "knob '{}' in {} model 0x{:02X} '{}' uses CC{} — that's in the bypass-CC range (0..=10)",
                        knob.id,
                        category.as_str(),
                        opt.id,
                        opt.title,
                        knob.ctrl,
                    );
                }
            }
        }
    }

    #[test]
    fn test_find_model_by_name_found() {
        // "Dist+" is in the EFX catalog at id 0x40
        let result = find_model_by_name(EffectCategory::Efx, "Dist+", FirmwareGen::Gen2);
        assert!(result.is_some());
        let (id, opt) = result.unwrap();
        assert_eq!(id, 0x40);
        assert_eq!(opt.title, "Dist+");
    }

    #[test]
    fn test_find_model_by_name_case_insensitive() {
        let result = find_model_by_name(EffectCategory::Efx, "dist+", FirmwareGen::Gen2);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, 0x40);
    }

    #[test]
    fn test_find_model_by_name_not_found() {
        assert!(
            find_model_by_name(EffectCategory::Amp, "Nonexistent Model", FirmwareGen::Gen2)
                .is_none()
        );
    }

    #[test]
    fn test_find_model_by_name_vol_sr_none() {
        assert!(find_model_by_name(EffectCategory::Vol, "Volume", FirmwareGen::Gen2).is_none());
        assert!(find_model_by_name(EffectCategory::Sr, "Send", FirmwareGen::Gen2).is_none());
    }

    #[test]
    fn test_find_model_by_name_gen_filter() {
        // "Damp" (0x0C) is Gen2-only — must not appear when querying Gen1
        assert!(find_model_by_name(EffectCategory::Reverb, "Damp", FirmwareGen::Gen1).is_none());
        assert!(find_model_by_name(EffectCategory::Reverb, "Damp", FirmwareGen::Gen2).is_some());
    }

    #[test]
    fn test_from_str_known_names() {
        assert_eq!("amp".parse::<EffectCategory>(), Ok(EffectCategory::Amp));
        assert_eq!("AMP".parse::<EffectCategory>(), Ok(EffectCategory::Amp));
        assert_eq!(
            "compression".parse::<EffectCategory>(),
            Ok(EffectCategory::Comp)
        );
        assert_eq!(
            "modulation".parse::<EffectCategory>(),
            Ok(EffectCategory::Mod)
        );
        assert_eq!("volume".parse::<EffectCategory>(), Ok(EffectCategory::Vol));
        assert!("unknown".parse::<EffectCategory>().is_err());
    }

    #[test]
    fn test_find_cc_for_knob_amp_gain() {
        // "gain" is the first amp knob — CC22 across all amp models that have it
        let cc = find_cc_for_knob(EffectCategory::Amp, "gain");
        assert!(cc.is_some());
        assert!(
            (22..=29).contains(&cc.unwrap()),
            "amp gain should be CC22–29, got {:?}",
            cc
        );
    }

    #[test]
    fn test_find_cc_for_knob_missing() {
        assert_eq!(
            find_cc_for_knob(EffectCategory::Amp, "nonexistent_knob"),
            None
        );
    }

    #[test]
    fn test_find_cc_for_knob_vol_sr_none() {
        assert_eq!(find_cc_for_knob(EffectCategory::Vol, "volume"), None);
        assert_eq!(find_cc_for_knob(EffectCategory::Sr, "level"), None);
    }

    #[test]
    fn test_find_cc_for_knob_efx_sens() {
        // "sens" appears in multiple EFX models; result must be within the EFX CC range
        let cc = find_cc_for_knob(EffectCategory::Efx, "sens");
        assert!(cc.is_some());
        assert!(
            (17..=21).contains(&cc.unwrap()),
            "EFX sens CC should be 17–21, got {:?}",
            cc
        );
    }

    /// as_str() must be non-empty and stable for every variant.
    #[test]
    fn test_as_str_non_empty() {
        for v in 0u8..=11 {
            let cat = EffectCategory::from_u8(v).unwrap();
            assert!(!cat.as_str().is_empty());
        }
    }
}
