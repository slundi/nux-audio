use std::collections::HashMap;

use crate::effects::{EffectCategory, EffectOption};
use crate::error::NuxError;

const PRESET_LETTERS: [char; 4] = ['A', 'B', 'C', 'D'];

/// On/off state for one scene slot — 3 raw bytes from the detail SysEx response.
///
/// Each byte encodes the bypass state of several effect slots. Exact bit layout
/// must be verified against hardware captures.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct SceneSnapshot(pub [u8; 3]);

#[derive(Default, Clone, Debug)]
pub struct Preset<'a> {
    pub number: u8,
    /// Raw tempo value encoded as `response[143] * 64 + response[144]`.
    /// This is milliseconds-per-beat; convert to BPM with `60_000 / tempo`.
    pub tempo: u16,
    /// Parallel blend level (0–127).
    pub parallel: u8,
    pub effect_order: Vec<EffectCategory>,
    pub name: String,
    pub scene: u8,
    pub active_scene_number: Option<u8>,
    /// Per-scene bypass snapshots (3 scenes on MG-30).
    pub scenes: Vec<SceneSnapshot>,
    pub effects: HashMap<EffectCategory, EffectOption<'a>>,
}

impl Preset<'_> {
    pub fn get_preset_code(&self) -> String {
        Self::format_code(self.number)
    }

    pub fn format_code(index: u8) -> String {
        let mut preset_code = String::with_capacity(3);
        let group_number = (index / 4) + 1;
        let letter_index = index % 4;
        if index < 36 {
            preset_code.push('0');
        }
        preset_code.push_str(&group_number.to_string());
        preset_code.push(PRESET_LETTERS[letter_index as usize]);
        preset_code
    }

    /// Parse preset number and active scene from a `CURRENT_PRESET_BASIC` SysEx response.
    ///
    /// Response header: `[0xF0, 0x43, 0x58, 0x70, 0x15, 0x02, ...]`
    ///   - Byte 6 = preset number (0x00–0x7F)
    ///   - Byte 9 = active scene (0=A, 1=B, 2=C)
    pub fn extract_current_preset_basic_data(response: &[u8]) -> Result<Self, NuxError> {
        const MIN_LEN: usize = 10;
        if response.len() < MIN_LEN {
            return Err(NuxError::ResponseTooShort(response.len()));
        }
        Ok(Self {
            number: response[6],
            active_scene_number: Some(response[9]),
            ..Default::default()
        })
    }

    /// Parse all preset fields from a `CURRENT_PRESET_DETAIL` SysEx response.
    ///
    /// Response header: `[0xF0, 0x43, 0x58, 0x70, 0x0B, 0x02, ...]`
    ///
    /// Confirmed byte offsets (reverse-engineered from hardware captures):
    /// | Bytes     | Field                                   |
    /// |-----------|----------------------------------------|
    /// | 6         | Preset number                           |
    /// | 143–144   | Tempo (ms/beat = `[143]*64 + [144]`)   |
    /// | 146       | Parallel blend (0–127)                 |
    /// | 147–164   | Effect signal-chain order (18 bytes)   |
    /// | 165–188   | Preset name (24 bytes, 3-byte pairs)   |
    /// | 208–210   | Scene 1 on/off bytes                   |
    /// | 211–213   | Scene 2 on/off bytes                   |
    /// | 214–216   | Scene 3 on/off bytes                   |
    pub fn extract_current_preset_detail_data(response: &[u8]) -> Result<Self, NuxError> {
        const MIN_LEN: usize = 217;
        if response.len() < MIN_LEN {
            return Err(NuxError::ResponseTooShort(response.len()));
        }

        let number = response[6];
        let tempo = (response[143] as u16) * 64 + response[144] as u16;
        let parallel = response[146];
        let effect_order = crate::effects::extract_effects_order(response)?;
        let name = Self::extract_name(&response[165..189])?;
        let scenes = vec![
            SceneSnapshot([response[208], response[209], response[210]]),
            SceneSnapshot([response[211], response[212], response[213]]),
            SceneSnapshot([response[214], response[215], response[216]]),
        ];

        Ok(Self {
            number,
            name,
            tempo,
            parallel,
            effect_order,
            scenes,
            ..Default::default()
        })
    }

    /// Decode a preset name from the 24-byte name section of a SysEx response.
    ///
    /// The MG-30 encodes each character pair into 3 bytes:
    ///   - `bytes[n]`   → first character (raw ASCII)
    ///   - `bytes[n+1]` → flag: 1 means add 0x40 to the second character
    ///   - `bytes[n+2]` → second character = `bytes[n+2] / 2 (+ 0x40 if flag set)`
    ///
    /// Parsing stops at a zero byte or end of input.
    pub fn extract_name(response: &[u8]) -> Result<String, NuxError> {
        let mut name = String::with_capacity(16);
        let mut counter = 0usize;
        while counter < response.len() {
            if response[counter] == 0 {
                break;
            }
            name.push(response[counter] as char);

            // Need two more bytes for the pair encoding; stop early if truncated.
            if counter + 2 >= response.len() {
                break;
            }

            let flag = response[counter + 1];
            let mut second_char = response[counter + 2] / 2;
            if flag == 1 {
                second_char += 0x40;
            }

            if second_char != 0 {
                name.push(second_char as char);
            } else {
                break;
            }

            counter += 3;
        }
        Ok(crate::utils::clean_string(&name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_preset_code() {
        let mut preset = Preset::default();
        assert_eq!(preset.get_preset_code(), "01A");

        preset.number = 35;
        assert_eq!(preset.get_preset_code(), "09D");

        preset.number = 36;
        assert_eq!(preset.get_preset_code(), "10A");

        preset.number = 127;
        assert_eq!(preset.get_preset_code(), "32D");
    }

    #[test]
    fn test_extract_current_preset_basic_data_too_short() {
        let short = vec![0xF0, 0x43, 0x58, 0x70, 0x15, 0x02, 0x05];
        assert!(matches!(
            Preset::extract_current_preset_basic_data(&short),
            Err(NuxError::ResponseTooShort(_))
        ));
    }

    #[test]
    fn test_extract_current_preset_basic_data_ok() {
        let mut resp = vec![0u8; 10];
        resp[6] = 0x07; // preset 7
        resp[9] = 0x02; // scene C
        let preset = Preset::extract_current_preset_basic_data(&resp).unwrap();
        assert_eq!(preset.number, 7);
        assert_eq!(preset.active_scene_number, Some(2));
    }

    #[test]
    fn test_extract_name_empty() {
        let bytes = [0u8; 24];
        assert_eq!(Preset::extract_name(&bytes).unwrap(), "");
    }

    #[test]
    fn test_extract_name_truncated() {
        // Should not panic when response ends before counter + 2
        let bytes = [0x41u8]; // just 'A', no pair bytes
        let name = Preset::extract_name(&bytes).unwrap();
        assert_eq!(name, "A");
    }

    #[test]
    fn test_extract_current_preset_detail_data_too_short() {
        let short = vec![0u8; 100];
        assert!(matches!(
            Preset::extract_current_preset_detail_data(&short),
            Err(NuxError::ResponseTooShort(_))
        ));
    }

    #[test]
    fn test_extract_current_preset_detail_data_fields() {
        let mut resp = vec![0u8; 217];
        // Preset number
        resp[6] = 0x0A;
        // Tempo: 120 BPM → ms/beat = 500 → 500 = 7*64 + 52
        resp[143] = 7;
        resp[144] = 52;
        // Parallel
        resp[146] = 64;
        // Scenes
        resp[208] = 0x01;
        resp[211] = 0x02;
        resp[214] = 0x03;
        // effect_order bytes 147..165 all zero → all Wah(0) — just check no panic
        // name bytes 165..189 all zero → empty name

        let preset = Preset::extract_current_preset_detail_data(&resp).unwrap();
        assert_eq!(preset.number, 0x0A);
        assert_eq!(preset.tempo, 500); // 7*64 + 52
        assert_eq!(preset.parallel, 64);
        assert_eq!(preset.scenes.len(), 3);
        assert_eq!(preset.scenes[0].0[0], 0x01);
        assert_eq!(preset.scenes[1].0[0], 0x02);
        assert_eq!(preset.scenes[2].0[0], 0x03);
    }
}
