//! String utilities

use crate::error::NuxAudioError;

/// Convert a string containing hex chars into a `Vec<u8>`.
/// Input string examples: `0bf043` or `0b f0 43`
pub fn hex_string_to_vec_u8(s: &str) -> Result<Vec<u8>, NuxAudioError> {
    let s = s.replace(" ", "");
    if s.len() % 2 == 1 {
        return Err(NuxAudioError::OddLengthHex(s.len()));
    }
    let mut bytes = Vec::new();
    for i in (0..s.len()).step_by(2) {
        let chunk = &s[i..i + 2];
        let byte =
            u8::from_str_radix(chunk, 16).map_err(|source| NuxAudioError::InvalidHexDigit {
                chunk: chunk.to_string(),
                source,
            })?;
        bytes.push(byte);
    }
    Ok(bytes)
}

/// Keep only ASCII alphanumerics, spaces and dashes, then trim surrounding whitespace.
///
/// Used to normalize text decoded from device responses (preset names, firmware
/// version), which may carry NUL padding or stray control bytes.
pub fn clean_string(s: &str) -> String {
    let result: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-')
        .collect();
    result.trim().to_string()
}

/// Convert a string containing u8 as decimal into a `Vec<u8>`.
/// Input string examples: `11 255 48 0` ou `11,255,48,0`
pub fn decimal_string_to_vec_u8(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    s.split([' ', ','])
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().parse::<u8>())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_string_to_vec_u8() {
        assert_eq!(
            hex_string_to_vec_u8("0bf043").unwrap(),
            vec![0x0b, 0xf0, 0x43]
        );
        assert_eq!(
            hex_string_to_vec_u8("0b f0 43").unwrap(),
            vec![0x0b, 0xf0, 0x43]
        );
        assert_eq!(
            hex_string_to_vec_u8("0b  f0  43").unwrap(),
            vec![0x0b, 0xf0, 0x43]
        );

        assert!(hex_string_to_vec_u8("0bf043a").is_err());
        assert!(hex_string_to_vec_u8("0bf043zr").is_err());
        assert!(hex_string_to_vec_u8("0b f0 43 a").is_err());
        assert!(hex_string_to_vec_u8("0b f0 43 zr").is_err());
    }

    #[test]
    fn test_clean_string() {
        assert_eq!(clean_string("  azerty  "), "azerty");
        assert_eq!(clean_string("Some french: garçon"), "Some french garon");
        assert_eq!(clean_string(" with null \0 char "), "with null  char");
    }

    #[test]
    fn test_decimal_string_to_vec_u8() {
        assert_eq!(
            decimal_string_to_vec_u8("11 255 48 0").unwrap(),
            vec![11, 255, 48, 0]
        );
        assert_eq!(
            decimal_string_to_vec_u8("11,255,48,0").unwrap(),
            vec![11, 255, 48, 0]
        );
        assert_eq!(
            decimal_string_to_vec_u8("11 255,48, 0  16").unwrap(),
            vec![11, 255, 48, 0, 16]
        );

        assert!(decimal_string_to_vec_u8("11 255 48 0 512").is_err());
        assert!(decimal_string_to_vec_u8("11 255 -48 0 512").is_err());
    }
}
