use anyhow::{anyhow, Result};

// const PRESET_DATA_COMMAND = (index: number) => {
//   return `58 70 0B 00 ${hexIndex(index)} 00 00 00 00 00 00 00`;
// };

// const SAVE_CURRENT_PRESET_DATA_COMMAND = (index: number) => {
//   // TODO: Save is abit tricky...we need to form whole 218 bytes with correct effect states and etc ...then send that to device
//   return `58 70 7E 02 00 00 ${hexIndex(index)} 00 00 00 00 00`;
// };

// const CURRENT_PRESET_EFFECT_ORDER_COMMAND = (options: Nux.EffectOption[]) => {
//   let command = [240, 67, 88, 112, 13, 1];
//   options.forEach((option) => {
//     if (option.index !== undefined && option.index !== null) {
//       command.push(option.index);
//     }
//   });
//   command.push(0);
//   command.push(247);
//   return command;
// };

pub fn change_scene(number: u8) -> Result<[u8; 3]> {
    if !(1..=3).contains(&number) {
        return Err(anyhow!("Invalid scene number '{number}'. It must be between 1 and 3"));
    }
    Ok([176, 79, number])
}

const DEVICE_VERSION_REQUEST: [u8; 2] = [0x58, 0x00];
const CURRENT_PRESET_BASIC: [u8; 4] = [0x58, 0x70, 0x15, 0x00];
const REQUEST_UPDATED_EFFECT_ORDER_COMMAND: [u8; 4] = [0x58, 0x70, 0x0C, 0x00];

// Response patterns
const DEVICE_VERSION_RESPONSE_PATTERN: [u8; 4] = [0xf0, 0x43, 0x58, 0x10];
const CURRENT_PRESET_BASIC_RESPONSE_PATTERN: [u8; 6] = [0xf0, 0x43, 0x58, 0x70, 0x15, 0x02];
const CURRENT_PRESET_DETAIL_RESPONSE_PATTERN: [u8; 6] = [0xf0, 0x43, 0x58, 0x70, 0x0b, 0x02];
const UPDATED_EFFECTS_ORDER_RESPONSE_PATTERN: [u8; 6] = [0xf0, 0x43, 0x58, 0x70, 0x0c, 0x02];
const PRESET_CHANGED_RESPONSE_PATTERN: [u8; 1] = [0xc0];
const SCENE_CHANGED_RESPONSE_PATTERN: [u8; 2] = [0xb0, 0x4f];
const EFFECT_CHANGED_RESPONSE_PATTERN: [u8; 1] = [0xb0];
const EFFECT_ORDER_CHANGED_RESPONSE_PATTERN: [u8; 7] = [0xf0, 0x43, 0x58, 0x70, 0x7e, 0x02, 0x0d];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_scene() {
        assert!(change_scene(0).is_err());
        assert!(change_scene(4).is_err());
        assert_eq!(change_scene(1).unwrap(), [176, 79, 1]);
        assert_eq!(change_scene(2).unwrap(), [176, 79, 2]);
        assert_eq!(change_scene(3).unwrap(), [176, 79, 3]);
    }
}

