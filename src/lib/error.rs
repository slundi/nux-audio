//! Domain-level error type for the `nux_audio` library crate.

/// Domain-level errors for the `nux_audio` library crate.
#[derive(thiserror::Error, Debug)]
pub enum NuxAudioError {
    // ── Hex utility ──────────────────────────────────────────────────────────
    /// Produced when a hex string has an odd number of characters (cannot decode full bytes).
    #[error("hex string must have an even number of characters, got {0}")]
    OddLengthHex(usize),

    /// Produced when a two-character hex chunk contains a non-hex digit.
    #[error("invalid hex digit in '{chunk}': {source}")]
    InvalidHexDigit {
        /// The two-character hex chunk that failed to parse.
        chunk: String,
        /// Underlying integer-parse error from `u8::from_str_radix`.
        #[source]
        source: std::num::ParseIntError,
    },

    // ── SysEx framing ─────────────────────────────────────────────────────────
    /// Produced when an empty byte slice is passed where a SysEx message is expected.
    #[error("SysEx message is empty")]
    SysExEmpty,

    /// Produced when a message does not begin with `0xF0` or does not end with `0xF7`.
    #[error("not a SysEx message (must start with 0xF0 and end with 0xF7)")]
    SysExInvalidFraming,

    /// Produced when a SysEx message has valid framing but is too short to contain any payload.
    #[error("SysEx message too short")]
    SysExTooShort,

    // ── Scene / preset / device validation ───────────────────────────────────
    /// Produced when a scene number outside 1–3 is passed to `change_scene`.
    #[error("invalid scene number {0}")]
    InvalidSceneNumber(u8),

    /// Produced when a preset index exceeds the valid range for the target device.
    #[error("preset index {0} is out of range for this device")]
    InvalidPresetIndex(u8),

    /// Produced when a device ID byte is not in the expected set for the command being built.
    #[error("device ID 0x{0:02X} is not valid for this command")]
    InvalidDeviceId(u8),

    // ── SysEx template ────────────────────────────────────────────────────────
    /// Produced by `SysExTemplate::build` when a `Param` slot has no matching entry in the params map.
    #[error("missing SysEx template parameter '{0}'")]
    MissingTemplateParam(&'static str),

    // ── Device response parsing ───────────────────────────────────────────────
    /// Produced when a device response is shorter than the decoder requires.
    #[error("SysEx response too short: {actual} bytes (expected at least {expected})")]
    ResponseTooShort {
        /// Minimum length the decoder needs.
        expected: usize,
        /// Length actually received.
        actual: usize,
    },

    /// Produced when the version response does not contain a recognizable version string.
    #[error("cannot parse firmware version from '{0}'")]
    VersionParse(String),

    /// Produced when a text field of a device response is not valid UTF-8.
    #[error("invalid UTF-8 in device response: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// Produced when a preset name contains a byte outside printable ASCII.
    #[error("invalid byte 0x{0:02X} in preset name (expected printable ASCII)")]
    InvalidPresetName(u8),

    // ── Effect catalog lookup ─────────────────────────────────────────────────
    /// Produced when a response is too short to read effect slot data from.
    #[error(
        "SysEx response too short for effect parsing: {actual} bytes (need at least {expected})"
    )]
    EffectResponseTooShort {
        /// Minimum length the decoder needs.
        expected: usize,
        /// Length actually received.
        actual: usize,
    },

    /// Produced when a byte does not map to any [`EffectCategory`](crate::effects::EffectCategory).
    #[error("unknown effect category byte 0x{0:02X}")]
    UnknownEffectCategory(u8),

    /// Produced when a model ID is absent from the category's catalog for this firmware.
    #[error("unknown model 0x{model_id:02X} for category 0x{category_byte:02X}")]
    UnknownEffectModel {
        /// Raw category byte from the response.
        category_byte: u8,
        /// Raw model ID from the response.
        model_id: u8,
    },
}
