//! Decoding of device SysEx responses that are not tied to a preset or an effect.
//!
//! Preset responses are decoded by [`Preset`](crate::preset::Preset); effect slot data
//! by [`effects`](crate::effects). Request builders and response header patterns live
//! in [`nux_sys_ex_commands`](crate::nux_sys_ex_commands).

use crate::error::NuxAudioError;
use crate::string::clean_string;

/// Extract the firmware version (e.g. `"v602"`) from a device version response.
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

    if let Some(value) = version.split_once("MG-30") {
        return Ok(clean_string(value.0));
    }
    Err(NuxAudioError::VersionParse(version))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(text: &[u8]) -> Vec<u8> {
        let mut message = vec![0xF0, 0x43, 0x58, 0x10];
        message.extend_from_slice(text);
        message.push(0xF7);
        message
    }

    #[test]
    fn extract_version_mg30() {
        assert_eq!(extract_version(&response(b"v602 MG-30")).unwrap(), "v602");
    }

    #[test]
    fn extract_version_trims_nul_padding() {
        assert_eq!(
            extract_version(&response(b"v402MG-30\0\0\0")).unwrap(),
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
            extract_version(&response(b"v100 XYZ")),
            Err(NuxAudioError::VersionParse(text)) if text == "v100 XYZ"
        ));
    }

    #[test]
    fn extract_version_invalid_utf8() {
        assert!(matches!(
            extract_version(&response(&[0xC3, 0x28, b'x'])),
            Err(NuxAudioError::Utf8(_))
        ));
    }
}
