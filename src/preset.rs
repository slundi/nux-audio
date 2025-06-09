use std::collections::HashMap;

use crate::effects::{EffectCategory, EffectOption};

const PRESET_LETTERS: [char; 4] = ['A', 'B', 'C', 'D'];

#[derive(Default, Clone, Debug)]
pub struct Preset<'a> {
    pub number: u8,
    pub tempo: f32,
    pub parallel: f32, // ?
    pub effect_order: Vec<EffectCategory>,
    pub name: String,
    pub scene: u8,
    pub active_scene_number: Option<u8>,
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

    pub fn extract_current_preset_basic_data(response: &[u8]) -> Self {
        let number = response[6];
        let scene = response[9];
        Self {
            number,
            active_scene_number: Some(scene),
            ..Default::default()
        }
    }

    // pub fn extract_current_preset_detail_data(response: &[u8]) -> Preset {
    //     // let tempo = response[143] * 64 + response[144]; // TODO: check
    //     Self {
    //         number: response[6],
    //         // tempo,
    //         // parallel: response[146],
    //         ..Default::default()
    //     }
    // }

    pub fn extract_name(response: &[u8]) -> String {
        let mut name = String::with_capacity(24);
        let mut counter = 0usize;
        while counter < response.len() {
            // If the first byte is zero, break the loop (equivalent to Dart check)
            if response[counter] == 0 {
                break;
            }

            name.push(response[counter] as char); // Add the first character

            // Calculate the second character from the third byte
            let mut second_char = response[counter + 2] / 2;

            // Apply the rule for second character based on the second byte
            if response[counter + 1] == 1 {
                second_char += 0x40;
            }

            // If the second character is non-zero, add it to the name
            if second_char != 0 {
                name.push(second_char as char);
            } else {
                break;
            }

            counter += 3; // Move to the next 3-byte segment
        }
        crate::utils::clean_string(&name)
    }
}

// function extractCurrentPresetBasicData(
//   response: Uint8Array,
// ): Partial<Nux.Preset> {
//   const presetNumber = response[6];
//   const presetScene = response[9];
//   const preset = {
//     presetNumber: presetNumber,
//     formattedPresetNumber: formatPresetNumber(presetNumber),
//     activeSceneNumber: presetScene,
//   };

//   return preset;
// }

// function extractCurrentPresetDetailData(
//   response: Uint8Array,
// ): Nux.SysExResponseData {
//   const data: Nux.SysExResponseData = {
//     presetNumber: response[6],
//     formattedPresetNumber: formatPresetNumber(response[6]),
//     tempo: response[143] * 64 + response[144],
//     parallel: response[146],
//     effectsOrder: extractEffectsOrder(response),
//     name: Parser.Presets.extractPresetName(response.slice(165, 189)),
//     scene: {
//       scene1: response.slice(208, 211),
//       scene2: response.slice(211, 214),
//       scene3: response.slice(214, 217),
//     },
//     effects: Parser.Effects.extractEffects(response),
//   };
//   return data;
// }

// function extractPresetName(nameBytes: Uint8Array) {
//   let name = "";
//   let counter = 0;

//   while (counter < nameBytes.length) {
//     // If the first byte is zero, break the loop (equivalent to Dart check)
//     if (nameBytes[counter] === 0) {
//       break;
//     }

//     // Add the first character
//     const char = String.fromCharCode(nameBytes[counter]);
//     name += char;

//     // Calculate the second character from the third byte
//     let secondChar = nameBytes[counter + 2] / 2;

//     // Apply the rule for second character based on the second byte
//     if (nameBytes[counter + 1] === 1) {
//       secondChar += 0x40;
//     }

//     // If the second character is non-zero, add it to the name
//     if (secondChar !== 0) {
//       name += String.fromCharCode(secondChar);
//     } else {
//       break;
//     }

//     // Move to the next 3-byte segment
//     counter += 3;
//   }

//   // Trim any trailing null characters (byte value 0)
//   name = name.replace(/\0+$/, "").trim();
//   // Remove any non-alphanumeric characters (except space and dash)
//   name = name.replace(/[^a-zA-Z0-9\s\-]/g, "");

//   return name;
// }

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
}
