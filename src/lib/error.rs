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
        chunk: String,
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
}
