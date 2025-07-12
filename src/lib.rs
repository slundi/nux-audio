const SYSEX_START: u8 = 0xF0;
const SYSEX_END: u8 = 0xF7;

/// Possible error when encoding/decoding SysEx messages
#[derive(Debug, PartialEq, Eq)]
pub enum SysExError {
    /// Does not start with 0xF0.
    InvalidStartByte,
    /// Does not end with 0xF7.
    InvalidEndByte,
    /// Too short to contain a manufacturer ID and its data
    MessageTooShort,
    /// The message contains status bytes (MSB is 1, except F0 and F7) inside the data.
    /// SysEx messages must contain data bytes (MSB is 0) between F0 and F7
    ContainsStatusByte,
    /// Internal error/other problem
    Other(String),
}

/// Represent a MIDI System Exclusive Message (SysEx) decoded.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SysExMessage {
    /// Manufacturer ID (1 or 3 bytes).
    pub manufacturer_id: Vec<u8>,
    /// Manufacturer specific bytes
    pub data_bytes: Vec<u8>,
}

impl SysExMessage {
    /// Creates a new SysEx message.
    ///
    /// # Arguments
    /// * `manufacturer_id` - A vector of bytes representing the manufacturer ID.
    ///                       Must be 1 or 3 bytes long.
    /// * `data_bytes` - A vector of bytes representing the specific data.
    ///                  All bytes must have the MSB set to 0 (value < 0x80).
    pub fn new(manufacturer_id: Vec<u8>, data_bytes: Vec<u8>) -> Result<Self, SysExError> {
        if manufacturer_id.is_empty() || (manufacturer_id.len() != 1 && manufacturer_id.len() != 3)
        {
            return Err(SysExError::Other(
                "Manufacturer ID must be 1 or 3 bytes long.".to_string(),
            ));
        }
        if data_bytes.iter().any(|&b| b >= 0x80) {
            return Err(SysExError::ContainsStatusByte);
        }
        Ok(SysExMessage {
            manufacturer_id,
            data_bytes,
        })
    }

    /// Encodes the `SysExMessage` into a raw MIDI byte vector.
    ///
    /// The resulting vector will start with `0xF0` and end with `0xF7`.
    pub fn encode(&self) -> Vec<u8> {
        let mut encoded_message = Vec::new();
        encoded_message.push(SYSEX_START);
        encoded_message.extend_from_slice(&self.manufacturer_id);
        encoded_message.extend_from_slice(&self.data_bytes);
        encoded_message.push(SYSEX_END);
        encoded_message
    }

    /// Decodes a raw MIDI byte slice into a `SysExMessage`.
    ///
    /// # Arguments
    /// * `bytes` - The raw MIDI byte slice to decode.
    ///
    /// # Errors
    /// Returns a `SysExError` if the message format is invalid.
    pub fn decode(bytes: &[u8]) -> Result<Self, SysExError> {
        // First, check for minimum length
        // A SysEx message must at least contain F0, Manufacturer ID (1 byte), F7
        // So, a minimum of 3 bytes.
        if bytes.len() < 3 {
            return Err(SysExError::MessageTooShort);
        }

        // Now check start and end bytes
        if bytes[0] != SYSEX_START {
            return Err(SysExError::InvalidStartByte);
        }
        if *bytes.last().unwrap() != SYSEX_END {
            return Err(SysExError::InvalidEndByte);
        }

        let mut current_idx = 1; // Start after SYSEX_START

        // Determine manufacturer ID length.
        // 1-byte IDs are typical. 3-byte IDs for extended or universal IDs (00 00 01, etc.).
        // A common pattern for 3-byte universal IDs is 0x00 0x00 0xXX or 0x00 0x7E 0xXX for universal non-realtime.
        // For simplicity, we check for 0x00 as the second byte to imply a 3-byte ID.
        // More robust parsing might be needed for edge cases or specific extended ID schemes.
        let manufacturer_id_len = if bytes.len() > 1 && bytes[1] == 0x00 && bytes.len() > 3 {
            // Re-check length for 3-byte ID: must be at least F0 + 3 ID bytes + F7 = 5 bytes
            if bytes.len() < 5 {
                return Err(SysExError::MessageTooShort);
            }
            3
        } else {
            1
        };

        if bytes.len() < current_idx + manufacturer_id_len {
            // This check should technically be redundant due to initial bytes.len() < 3 check for 1-byte ID
            // and the bytes.len() < 5 check for 3-byte ID, but keeping it as an extra safeguard.
            return Err(SysExError::MessageTooShort);
        }

        let manufacturer_id = bytes[current_idx..current_idx + manufacturer_id_len].to_vec();
        current_idx += manufacturer_id_len;

        // Data bytes are between the manufacturer ID and SYSEX_END
        let data_end_idx = bytes.len() - 1; // Position of SYSEX_END

        if current_idx > data_end_idx {
            // This condition should now be covered by initial length checks,
            // but is fine to remain for clarity if a message is like F0 41 F7 (no data).
            // For F0 41 F7, current_idx would be 2, data_end_idx would be 2.
            // current_idx > data_end_idx would be false.
            // data_bytes would be bytes[2..2] which is empty, which is correct.
            // So no explicit return Err(SysExError::MessageTooShort) needed here.
        }

        let data_bytes = bytes[current_idx..data_end_idx].to_vec();

        // Check that all data bytes have MSB set to 0 (no status bytes)
        if data_bytes.iter().any(|&b| b >= 0x80) {
            return Err(SysExError::ContainsStatusByte);
        }

        Ok(SysExMessage {
            manufacturer_id,
            data_bytes,
        })
    }
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    // Test basic encoding/decoding with a 1-byte ID
    #[test]
    fn test_encode_decode_single_byte_id() {
        let original_data = vec![0x01, 0x02, 0x7F, 0x03];
        let original_manufacturer_id = vec![0x41]; // Roland ID

        let message = SysExMessage::new(original_manufacturer_id.clone(), original_data.clone())
            .expect("Failed to create SysExMessage");
        let encoded = message.encode();

        // Encoded message should look like: F0 41 01 02 7F 03 F7
        assert_eq!(encoded, vec![0xF0, 0x41, 0x01, 0x02, 0x7F, 0x03, 0xF7]);

        let decoded_message =
            SysExMessage::decode(&encoded).expect("Failed to decode SysEx message");

        assert_eq!(decoded_message.manufacturer_id, original_manufacturer_id);
        assert_eq!(decoded_message.data_bytes, original_data);
    }

    // Test encoding/decoding with a 3-byte ID (e.g., Universal Non-Realtime)
    #[test]
    fn test_encode_decode_three_byte_id() {
        let original_data = vec![0x01, 0x02, 0x03, 0x04];
        let original_manufacturer_id = vec![0x00, 0x00, 0x7F]; // Universal Non-Realtime

        let message = SysExMessage::new(original_manufacturer_id.clone(), original_data.clone())
            .expect("Failed to create SysExMessage");
        let encoded = message.encode();

        // Encoded message should look like: F0 00 00 7F 01 02 03 04 F7
        assert_eq!(
            encoded,
            vec![0xF0, 0x00, 0x00, 0x7F, 0x01, 0x02, 0x03, 0x04, 0xF7]
        );

        let decoded_message =
            SysExMessage::decode(&encoded).expect("Failed to decode SysEx message");

        assert_eq!(decoded_message.manufacturer_id, original_manufacturer_id);
        assert_eq!(decoded_message.data_bytes, original_data);
    }

    // Test for an empty data payload
    #[test]
    fn test_empty_data() {
        let manufacturer_id = vec![0x41];
        let message = SysExMessage::new(manufacturer_id.clone(), vec![])
            .expect("Failed to create SysExMessage");
        let encoded = message.encode();
        assert_eq!(encoded, vec![0xF0, 0x41, 0xF7]);
        let decoded = SysExMessage::decode(&encoded).unwrap();
        assert_eq!(decoded.manufacturer_id, manufacturer_id);
        assert!(decoded.data_bytes.is_empty());
    }

    // --- Error Tests ---
    #[test]
    fn test_decode_invalid_start_byte() {
        let bytes = vec![0x00, 0x41, 0x01, 0xF7];
        assert_eq!(
            SysExMessage::decode(&bytes),
            Err(SysExError::InvalidStartByte)
        );
    }

    #[test]
    fn test_decode_invalid_end_byte() {
        let bytes = vec![0xF0, 0x41, 0x01, 0x00];
        assert_eq!(
            SysExMessage::decode(&bytes),
            Err(SysExError::InvalidEndByte)
        );
    }

    #[test]
    fn test_decode_message_too_short() {
        let bytes_no_man_id = vec![0xF0, 0xF7]; // F0 + F7, no manufacturer ID
        assert_eq!(
            SysExMessage::decode(&bytes_no_man_id),
            Err(SysExError::MessageTooShort)
        );

        let bytes_only_f0 = vec![0xF0];
        assert_eq!(
            SysExMessage::decode(&bytes_only_f0),
            Err(SysExError::MessageTooShort)
        );

        let bytes_empty = vec![];
        assert_eq!(
            SysExMessage::decode(&bytes_empty),
            Err(SysExError::MessageTooShort)
        );
    }

    #[test]
    fn test_decode_contains_status_byte_in_data() {
        // F0 41 90 F7 (90 is a Status Byte)
        let bytes = vec![0xF0, 0x41, 0x90, 0xF7];
        assert_eq!(
            SysExMessage::decode(&bytes),
            Err(SysExError::ContainsStatusByte)
        );
    }

    #[test]
    fn test_new_invalid_manufacturer_id_length() {
        let result = SysExMessage::new(vec![0x41, 0x42], vec![0x01]);
        assert_eq!(
            result,
            Err(SysExError::Other(
                "Manufacturer ID must be 1 or 3 bytes long.".to_string()
            ))
        );
    }

    #[test]
    fn test_new_data_bytes_contains_status_byte() {
        let result = SysExMessage::new(vec![0x41], vec![0x01, 0x90]);
        assert_eq!(result, Err(SysExError::ContainsStatusByte));
    }
}
