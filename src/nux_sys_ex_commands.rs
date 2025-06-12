use anyhow::{anyhow, Result};

// SysEx messages are starting with:
// * 0xf0 start sysex
// * 0x?? manufacturer ID
// * command type
// * data 1
// * data ...
// * 0xf7 end sysex

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
        return Err(anyhow!(
            "Invalid scene number '{number}'. It must be between 1 and 3"
        ));
    }
    Ok([176, 79, number])
}

pub const DEVICE_VERSION_REQUEST: [u8; 2] = [0x58, 0x00];
pub const CURRENT_PRESET_BASIC: [u8; 4] = [0x58, 0x70, 0x15, 0x00];
pub const REQUEST_UPDATED_EFFECT_ORDER_COMMAND: [u8; 4] = [0x58, 0x70, 0x0C, 0x00];

// Response patterns
pub const DEVICE_VERSION_RESPONSE_PATTERN: [u8; 4] = [0xf0, 0x43, 0x58, 0x10];
pub const CURRENT_PRESET_BASIC_RESPONSE_PATTERN: [u8; 6] = [0xf0, 0x43, 0x58, 0x70, 0x15, 0x02];
pub const CURRENT_PRESET_DETAIL_RESPONSE_PATTERN: [u8; 6] = [0xf0, 0x43, 0x58, 0x70, 0x0b, 0x02];
pub const UPDATED_EFFECTS_ORDER_RESPONSE_PATTERN: [u8; 6] = [0xf0, 0x43, 0x58, 0x70, 0x0c, 0x02];
pub const PRESET_CHANGED_RESPONSE_PATTERN: [u8; 1] = [0xc0];
pub const SCENE_CHANGED_RESPONSE_PATTERN: [u8; 2] = [0xb0, 0x4f];
pub const EFFECT_CHANGED_RESPONSE_PATTERN: [u8; 1] = [0xb0];
pub const EFFECT_ORDER_CHANGED_RESPONSE_PATTERN: [u8; 7] =
    [0xf0, 0x43, 0x58, 0x70, 0x7e, 0x02, 0x0d];

pub fn build_sysex_message(message: &[u8]) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(message.len() + 2);
    result.push(0xf0);
    for byte in message {
        result.push(*byte);
    }
    result.push(0xf7);
    result
}

pub fn extract_sysex_message(message: &[u8]) -> Result<&[u8]> {
    if message.is_empty() {
        return Err(anyhow!("Empty message"));
    }

    if message[0] != 0xf0 || message[message.len() - 1] != 0xf7 {
        return Err(anyhow!("Not a SysEx message"));
    }

    // SysEx messages are starting with 0xf0 and ends with 0xf7
    // println!(" Raw MIDI SysEx message ({} bytes): {:?}", message.len(), message);

    if message.len() < 3 {
        return Err(anyhow!("SysEx message too short"));
    }

    // let manufacturer_id = message[1];
    let data = &message[2..message.len() - 1]; // exclude f0, f7 & manufacturer ID
    Ok(data)
    // println!("  Manufacturer ID (simplified) : 0x{:X}", manufacturer_id);
    // println!("  Useful data (simplified) : {:?}", data);
}

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

    fn test_sysex_message() {
        // test build
        assert_eq!(
            build_sysex_message(&DEVICE_VERSION_REQUEST),
            [0xf0, 0x43, 0x58, 0x00, 0xf7]
        );

        // test extract data
        assert_eq!(
            extract_sysex_message(&[0xf0, 0x43, 0x58, 0x00, 0xf7]).unwrap(),
            DEVICE_VERSION_REQUEST
        );
        assert!(extract_sysex_message(&[]).is_err());
        assert!(extract_sysex_message(&[0x00, 0x43, 0x58, 0x00, 0xf7]).is_err());
        assert!(extract_sysex_message(&[0xf0, 0x43, 0x58, 0x00, 0x00]).is_err());
        assert!(extract_sysex_message(&[0xf0, 0xf7]).is_err());
    }
}
