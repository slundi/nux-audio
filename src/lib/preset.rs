//! The [`Preset`] model and the layout-independent decoders (slot code, basic
//! response, name encoding).
//!
//! The full detail response layout differs per device and firmware generation, so
//! it is decoded by the [`driver`](crate::driver) for that device.

use std::collections::HashMap;

use crate::effects::{EffectCategory, EffectOption};
use crate::error::NuxAudioError;

const PRESET_LETTERS: [char; 4] = ['A', 'B', 'C', 'D'];

/// On/off state for one scene slot — 3 raw bytes from the detail SysEx response.
///
/// Each byte encodes the bypass state of several effect slots. Exact bit layout
/// must be verified against hardware captures.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct SceneSnapshot(pub [u8; 3]);

/// A device preset, as decoded from SysEx responses.
///
/// Which fields are filled depends on the decoder: the basic response only gives
/// `number` and `active_scene_number`; the detail response gives the rest, as far
/// as the device's layout is known (see
/// [`DeviceDriver::decode_preset_detail`](crate::driver::DeviceDriver::decode_preset_detail)).
#[derive(Default, Clone, Debug)]
pub struct Preset {
    /// Slot index (0x00–0x7F); see [`Preset::format_code`] for the display form.
    pub number: u8,
    /// Tempo in milliseconds per beat; convert to BPM with `60_000 / tempo`.
    pub tempo: u16,
    /// Whether tempo sync is enabled, when the response layout exposes it.
    pub tempo_sync: Option<bool>,
    /// Parallel blend level (0–127).
    pub parallel: u8,
    /// Effect slots in signal-chain order.
    pub effect_order: Vec<EffectCategory>,
    /// Preset name (printable ASCII, cleaned).
    pub name: String,
    /// Scene index used by front-ends for editing (0 = A).
    pub scene: u8,
    /// Scene currently active on the device (0 = A), when known.
    pub active_scene_number: Option<u8>,
    /// Per-scene bypass snapshots (3 scenes on MG-30).
    pub scenes: Vec<SceneSnapshot>,
    /// Active model per populated slot.
    pub effects: HashMap<EffectCategory, EffectOption>,
}

impl Preset {
    /// Display code of this preset's slot (e.g. `"02B"`); see [`Preset::format_code`].
    pub fn get_preset_code(&self) -> String {
        Self::format_code(self.number)
    }

    /// Format a slot index as the device shows it: bank number + letter A–D.
    ///
    /// Four presets per bank, zero-padded below bank 10: `0 → "01A"`, `5 → "02B"`,
    /// `127 → "32D"`.
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
    pub fn extract_current_preset_basic_data(response: &[u8]) -> Result<Self, NuxAudioError> {
        const MIN_LEN: usize = 10;
        if response.len() < MIN_LEN {
            return Err(NuxAudioError::ResponseTooShort {
                expected: MIN_LEN,
                actual: response.len(),
            });
        }
        Ok(Self {
            number: response[6],
            active_scene_number: Some(response[9]),
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
    pub fn extract_name(response: &[u8]) -> Result<String, NuxAudioError> {
        let mut name = String::with_capacity(16);
        let mut counter = 0usize;
        while counter < response.len() {
            let b = response[counter];
            if b == 0 {
                break;
            }
            // Bytes > 127 are not valid ASCII; the MG-30 only uses printable ASCII
            // in preset names. Non-printable ASCII control chars (< 0x20, 0x7F)
            // are treated as end-of-name rather than errors.
            if b > 127 {
                return Err(NuxAudioError::InvalidPresetName(b));
            }
            if b < b' ' || b == 127 {
                break;
            }
            name.push(b as char);

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
        Ok(crate::string::clean_string(&name))
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
            Err(NuxAudioError::ResponseTooShort { .. })
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
    fn test_extract_name_non_ascii_errors() {
        let bytes = [0xC0u8, 0x00, 0x00]; // 0xC0 > 127
        assert!(matches!(
            Preset::extract_name(&bytes),
            Err(NuxAudioError::InvalidPresetName(0xC0))
        ));
    }

    #[test]
    fn test_extract_name_control_char_stops() {
        // ASCII control chars (< 0x20) silently terminate the name
        let bytes = [b'A', 0x01, 0x00]; // 'A', then control char
        let name = Preset::extract_name(&bytes).unwrap();
        assert_eq!(name, "A");
    }
}
