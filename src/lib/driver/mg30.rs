//! NUX MG-30 driver.
//!
//! The firmware generation (see [`FirmwareGen`]) changes two things:
//! - **catalogs**: some models only exist on GEN2 (filtered via `EffectOption::min_gen`);
//! - **preset detail layout**: fw 4.0.3 (GEN1) sends 218 bytes, fw ≥ 5 (GEN2) sends 222
//!   with most fields shifted. Both layouts are decoded here; see `docs/mg30.md`.
//!
//! Requests, response headers and the basic preset response are the same on both.

use super::{DeviceDriver, Request};
use crate::effects::{
    EffectCategory, EffectOption, FirmwareGen, KnobDef, extract_effects_order, find_knob_def,
    find_model_by_name, get_model,
};
use crate::error::NuxAudioError;
use crate::nux_sys_ex_commands::{
    CURRENT_PRESET_BASIC, CURRENT_PRESET_BASIC_RESPONSE_PATTERN,
    CURRENT_PRESET_DETAIL_RESPONSE_PATTERN, DEVICE_VERSION_REQUEST,
    DEVICE_VERSION_RESPONSE_PATTERN, mg30_change_effect_model, mg30_request_preset,
};
use crate::preset::{Preset, SceneSnapshot};
use crate::string::clean_string;

/// Registry model code of the MG-30.
pub const MODEL_CODE: &str = "MG-30";

/// Highest preset slot (128 presets, 7-bit slot byte).
const MAX_PRESET_SLOT: u8 = 0x7F;

/// Version request and its response header (same on all generations).
pub fn version_request() -> Request {
    Request {
        payload: DEVICE_VERSION_REQUEST.to_vec(),
        response_header: &DEVICE_VERSION_RESPONSE_PATTERN,
    }
}

/// Extract the firmware version (e.g. `"v602"`) from the version response.
///
/// `response` is the full SysEx message as received (`F0 43 58 10 <ascii…> F7`):
/// the 4 header bytes and the trailing `F7` are skipped, NUL padding is trimmed, and
/// the text before the `"MG-30"` model marker is returned.
pub fn extract_version(response: &[u8]) -> Result<String, NuxAudioError> {
    if response.len() < 6 {
        return Err(NuxAudioError::ResponseTooShort {
            expected: 6,
            actual: response.len(),
        });
    }
    let version_bytes = &response[4..response.len() - 1];
    let version = String::from_utf8(version_bytes.to_vec())?;
    let version = version.trim_end_matches('\0');
    let version = version.trim().to_string();

    if let Some(value) = version.split_once(MODEL_CODE) {
        return Ok(clean_string(value.0));
    }
    Err(NuxAudioError::VersionParse(version))
}

fn ensure_len(response: &[u8], expected: usize) -> Result<(), NuxAudioError> {
    if response.len() < expected {
        return Err(NuxAudioError::ResponseTooShort {
            expected,
            actual: response.len(),
        });
    }
    Ok(())
}

/// Decode a fw 4.0.3 (GEN1) preset detail response.
///
/// Layout from the mg30-controller Dart app (0-indexed, framing included):
/// | Bytes     | Field                                   |
/// |-----------|-----------------------------------------|
/// | 6         | Preset number                           |
/// | 143–144   | Tempo (ms/beat = `[143]*64 + [144]`)    |
/// | 146       | Parallel blend (0–127)                  |
/// | 147–164   | Effect signal-chain order (18 bytes)    |
/// | 165–188   | Preset name (24 bytes, 3-byte pairs)    |
/// | 208–216   | Scenes 1–3 on/off bytes (3 × 3)         |
pub fn decode_detail_gen1(response: &[u8]) -> Result<Preset, NuxAudioError> {
    ensure_len(response, 217)?;
    Ok(Preset {
        number: response[6],
        tempo: (response[143] as u16) * 64 + response[144] as u16,
        parallel: response[146],
        effect_order: extract_effects_order(response)?,
        name: Preset::extract_name(&response[165..189])?,
        scenes: vec![
            SceneSnapshot([response[208], response[209], response[210]]),
            SceneSnapshot([response[211], response[212], response[213]]),
            SceneSnapshot([response[214], response[215], response[216]]),
        ],
        ..Default::default()
    })
}

/// Decode a fw ≥ 5 (GEN2) preset detail response.
///
/// Only fields confirmed on GEN2 hardware are decoded (0-indexed, framing included):
/// | Bytes     | Field                                                |
/// |-----------|------------------------------------------------------|
/// | 6         | Preset number                                        |
/// | 144       | Tempo sync (`0x00` = off, non-zero = on; empirical)  |
/// | 147       | BPM (`(byte + 128) / 2`, so 64–127 BPM)              |
/// | 169–192   | Preset name (24 bytes, 3-byte pairs, as GEN1)        |
///
/// Effect order, scenes and parallel blend are not mapped yet for GEN2 and are left
/// empty / zero rather than guessed.
pub fn decode_detail_gen2(response: &[u8]) -> Result<Preset, NuxAudioError> {
    ensure_len(response, 193)?;
    let bpm = (response[147] as u16 + 128) / 2;
    Ok(Preset {
        number: response[6],
        tempo: 60_000 / bpm,
        tempo_sync: Some(response[144] != 0x00),
        name: Preset::extract_name(&response[169..193])?,
        ..Default::default()
    })
}

/// MG-30 driver for one firmware generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mg30Driver {
    firmware: String,
    fw_gen: FirmwareGen,
}

impl Mg30Driver {
    /// Driver for firmware `firmware` (e.g. `"v602"`); the generation is derived from it.
    pub fn new(firmware: String) -> Self {
        let fw_gen = FirmwareGen::from_version_str(&firmware);
        Self { firmware, fw_gen }
    }
}

impl DeviceDriver for Mg30Driver {
    fn model_code(&self) -> &'static str {
        MODEL_CODE
    }

    fn firmware(&self) -> &str {
        &self.firmware
    }

    fn firmware_gen(&self) -> Option<FirmwareGen> {
        Some(self.fw_gen)
    }

    fn bypass_cc(&self, category: EffectCategory) -> Option<u8> {
        category.bypass_cc()
    }

    fn knob(&self, category: EffectCategory, knob_id: &str) -> Option<&'static KnobDef> {
        find_knob_def(category, knob_id)
    }

    fn model_by_id(&self, category: EffectCategory, model_id: u8) -> Option<&'static EffectOption> {
        get_model(category, model_id, self.fw_gen)
    }

    fn model_by_name(
        &self,
        category: EffectCategory,
        name: &str,
    ) -> Option<(u8, &'static EffectOption)> {
        find_model_by_name(category, name, self.fw_gen)
    }

    fn change_model(&self, category: EffectCategory, model_id: u8) -> Vec<u8> {
        mg30_change_effect_model(category as u8, model_id).to_vec()
    }

    fn current_preset_request(&self) -> Request {
        Request {
            payload: CURRENT_PRESET_BASIC.to_vec(),
            response_header: &CURRENT_PRESET_BASIC_RESPONSE_PATTERN,
        }
    }

    fn decode_current_preset(&self, response: &[u8]) -> Result<Preset, NuxAudioError> {
        Preset::extract_current_preset_basic_data(response)
    }

    fn preset_detail_request(&self, slot: u8) -> Result<Request, NuxAudioError> {
        if slot > MAX_PRESET_SLOT {
            return Err(NuxAudioError::InvalidPresetIndex(slot));
        }
        Ok(Request {
            payload: mg30_request_preset(slot).to_vec(),
            response_header: &CURRENT_PRESET_DETAIL_RESPONSE_PATTERN,
        })
    }

    fn decode_preset_detail(&self, response: &[u8]) -> Result<Preset, NuxAudioError> {
        match self.fw_gen {
            FirmwareGen::Gen1 => decode_detail_gen1(response),
            FirmwareGen::Gen2 => decode_detail_gen2(response),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen1() -> Mg30Driver {
        Mg30Driver::new("v402".to_owned())
    }

    fn gen2() -> Mg30Driver {
        Mg30Driver::new("v602".to_owned())
    }

    fn version_reply(text: &[u8]) -> Vec<u8> {
        let mut reply = DEVICE_VERSION_RESPONSE_PATTERN.to_vec();
        reply.extend_from_slice(text);
        reply.push(0xF7);
        reply
    }

    // ── Version ───────────────────────────────────────────────────────────────

    #[test]
    fn extract_version_mg30() {
        assert_eq!(
            extract_version(&version_reply(b"v602 MG-30")).unwrap(),
            "v602"
        );
    }

    #[test]
    fn extract_version_trims_nul_padding() {
        assert_eq!(
            extract_version(&version_reply(b"v402MG-30\0\0\0")).unwrap(),
            "v402"
        );
    }

    #[test]
    fn extract_version_too_short() {
        assert!(matches!(
            extract_version(&[0xF0, 0x43, 0x58, 0x10, 0xF7]),
            Err(NuxAudioError::ResponseTooShort {
                expected: 6,
                actual: 5
            })
        ));
    }

    #[test]
    fn extract_version_without_model_marker() {
        assert!(matches!(
            extract_version(&version_reply(b"v100 XYZ")),
            Err(NuxAudioError::VersionParse(text)) if text == "v100 XYZ"
        ));
    }

    #[test]
    fn extract_version_invalid_utf8() {
        assert!(matches!(
            extract_version(&version_reply(&[0xC3, 0x28, b'x'])),
            Err(NuxAudioError::Utf8(_))
        ));
    }

    // ── Generation detection & catalogs ───────────────────────────────────────

    #[test]
    fn new_derives_generation_from_firmware() {
        assert_eq!(gen1().firmware_gen(), Some(FirmwareGen::Gen1));
        assert_eq!(gen2().firmware_gen(), Some(FirmwareGen::Gen2));
        assert_eq!(gen2().firmware(), "v602");
        assert_eq!(gen2().model_code(), "MG-30");
    }

    #[test]
    fn catalog_lookups_follow_generation() {
        // Damp reverb (0x0C) is GEN2-only.
        assert!(
            gen1()
                .model_by_name(EffectCategory::Reverb, "damp")
                .is_none()
        );
        let (id, model) = gen2()
            .model_by_name(EffectCategory::Reverb, "damp")
            .unwrap();
        assert_eq!(id, 0x0C);
        assert_eq!(model.id, 0x0C);
        assert!(gen1().model_by_id(EffectCategory::Reverb, 0x0C).is_none());
        assert!(gen2().model_by_id(EffectCategory::Reverb, 0x0C).is_some());
        // Models available on both generations resolve on both.
        assert!(gen1().model_by_id(EffectCategory::Reverb, 0x00).is_some());
    }

    #[test]
    fn knobs_and_bypass() {
        assert_eq!(gen2().knob(EffectCategory::Amp, "gain").unwrap().ctrl, 22);
        assert!(gen2().knob(EffectCategory::Amp, "nope").is_none());
        assert_eq!(gen2().bypass_cc(EffectCategory::Reverb), Some(8));
        assert_eq!(gen2().bypass_cc(EffectCategory::Vol), None);
    }

    // ── Requests ──────────────────────────────────────────────────────────────

    #[test]
    fn requests_are_generation_independent() {
        for driver in [gen1(), gen2()] {
            assert_eq!(
                driver.change_model(EffectCategory::Reverb, 0x04),
                vec![0x58, 0x09, 0x30, 0x08, 0x04]
            );
            let current = driver.current_preset_request();
            assert_eq!(current.payload, CURRENT_PRESET_BASIC.to_vec());
            assert_eq!(
                current.response_header,
                &CURRENT_PRESET_BASIC_RESPONSE_PATTERN
            );
            let detail = driver.preset_detail_request(0x7F).unwrap();
            assert_eq!(detail.payload[4], 0x7F);
            assert_eq!(
                detail.response_header,
                &CURRENT_PRESET_DETAIL_RESPONSE_PATTERN
            );
        }
    }

    #[test]
    fn preset_detail_request_rejects_missing_slot() {
        assert!(matches!(
            gen2().preset_detail_request(0x80),
            Err(NuxAudioError::InvalidPresetIndex(0x80))
        ));
    }

    #[test]
    fn decode_current_preset_reads_slot_and_scene() {
        let mut response = vec![0u8; 15];
        response[6] = 0x07;
        response[9] = 0x02;
        let preset = gen2().decode_current_preset(&response).unwrap();
        assert_eq!(preset.number, 7);
        assert_eq!(preset.active_scene_number, Some(2));
    }

    // ── Detail layouts ────────────────────────────────────────────────────────

    #[test]
    fn decode_detail_gen1_fields() {
        let mut resp = vec![0u8; 218];
        resp[6] = 0x0A;
        // Tempo: 120 BPM → ms/beat = 500 → 500 = 7*64 + 52
        resp[143] = 7;
        resp[144] = 52;
        resp[146] = 64;
        resp[208] = 0x01;
        resp[211] = 0x02;
        resp[214] = 0x03;
        // effect_order bytes 147..165 all zero → all Wah(0); name bytes all zero → ""

        let preset = gen1().decode_preset_detail(&resp).unwrap();
        assert_eq!(preset.number, 0x0A);
        assert_eq!(preset.tempo, 500);
        assert_eq!(preset.tempo_sync, None);
        assert_eq!(preset.parallel, 64);
        // Chain is decoded on GEN1 (exact slot count is unverified, see extract_effects_order).
        assert!(!preset.effect_order.is_empty());
        assert_eq!(preset.scenes.len(), 3);
        assert_eq!(preset.scenes[0].0[0], 0x01);
        assert_eq!(preset.scenes[1].0[0], 0x02);
        assert_eq!(preset.scenes[2].0[0], 0x03);
    }

    #[test]
    fn decode_detail_gen1_too_short() {
        assert!(matches!(
            decode_detail_gen1(&[0u8; 100]),
            Err(NuxAudioError::ResponseTooShort { expected: 217, .. })
        ));
    }

    /// 222-byte GEN2 detail response with BPM, sync and name set.
    fn gen2_detail(bpm_byte: u8, sync_byte: u8) -> Vec<u8> {
        let mut resp = vec![0u8; 222];
        resp[..6].copy_from_slice(&CURRENT_PRESET_DETAIL_RESPONSE_PATTERN);
        resp[6] = 0x05;
        resp[144] = sync_byte;
        resp[147] = bpm_byte;
        // "AA": 'A', flag 1, 0x02 → 0x02/2 + 0x40 = 'A'
        resp[169..172].copy_from_slice(&[0x41, 0x01, 0x02]);
        resp[221] = 0xF7;
        resp
    }

    #[test]
    fn decode_detail_gen2_confirmed_fields() {
        // Confirmed captures: BPM 120 = 0x70 (sync on), BPM 98 = 0x44 (sync off).
        let fast = gen2()
            .decode_preset_detail(&gen2_detail(0x70, 0x06))
            .unwrap();
        assert_eq!(fast.number, 5);
        assert_eq!(fast.tempo, 500); // 60_000 / 120
        assert_eq!(fast.tempo_sync, Some(true));
        assert_eq!(fast.name, "AA");
        // Not mapped for GEN2 yet → left empty instead of misread.
        assert!(fast.effect_order.is_empty());
        assert!(fast.scenes.is_empty());

        let slow = gen2()
            .decode_preset_detail(&gen2_detail(0x44, 0x00))
            .unwrap();
        assert_eq!(60_000 / slow.tempo, 98);
        assert_eq!(slow.tempo_sync, Some(false));
    }

    /// Regression: the GEN1 decoder reads GEN2's BPM byte (147) as the first
    /// effect-order slot, so a typical GEN2 preset failed to decode before drivers.
    #[test]
    fn gen2_response_is_not_decodable_with_gen1_layout() {
        let response = gen2_detail(0x44, 0x00);
        assert!(matches!(
            decode_detail_gen1(&response),
            Err(NuxAudioError::UnknownEffectCategory(0x44))
        ));
        assert!(gen2().decode_preset_detail(&response).is_ok());
    }

    #[test]
    fn decode_detail_gen2_too_short() {
        assert!(matches!(
            decode_detail_gen2(&[0u8; 180]),
            Err(NuxAudioError::ResponseTooShort { expected: 193, .. })
        ));
    }
}
