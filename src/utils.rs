use crate::error::NuxError;

pub fn hex_to_bytes(hex_string: &str) -> Result<Vec<u8>, NuxError> {
    if !hex_string.len().is_multiple_of(2) {
        return Err(NuxError::OddLengthHex(hex_string.len()));
    }

    (0..hex_string.len())
        .step_by(2)
        .map(|i| {
            let chunk = &hex_string[i..i + 2];
            u8::from_str_radix(chunk, 16).map_err(|source| NuxError::InvalidHexDigit {
                chunk: chunk.to_string(),
                source,
            })
        })
        .collect()
}

pub fn unit_hex_to_bytes(response: &[u8]) -> Vec<String> {
    response
        .iter() // Iterate over each byte in the slice
        .map(|&byte| {
            // For each byte...
            // Convert the byte to its hexadecimal string representation.
            // {:02X} is a formatting specifier:
            //   - `:` starts the format options.
            //   - `0` means pad with leading zeros.
            //   - `2` means total width of 2 characters.
            //   - `X` means uppercase hexadecimal.
            format!("{:02X}", byte)
        })
        .collect() // Collect all the resulting String representations into a Vec<String>
}

/// Remove non-alphanumeric characters (except space and dash) from a `&str`.
/// Also trim the string and trim any trailing null characters (byte value 0)
pub fn clean_string(s: &str) -> String {
    let result: String = s
        .chars() // Iterate over each character in the string
        .filter(|c| {
            // Keep characters that satisfy the condition
            c.is_ascii_alphanumeric() // Check if it's a-z, A-Z, or 0-9
                || *c == ' ' // Keep spaces
                || *c == '-' // Keep dashes
        })
        .collect(); // Collect the filtered characters into a new String
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_bytes() {
        assert!(hex_to_bytes("123").is_err());
        assert_eq!(hex_to_bytes("00aaff").unwrap(), vec![0, 170, 255]);
    }

    #[test]
    fn test_clean_string() {
        assert_eq!(clean_string("  azerty  "), "azerty");
        assert_eq!(clean_string("Some french: garçon"), "Some french garon");
        assert_eq!(clean_string(" with null \0 char "), "with null  char");
    }
}
