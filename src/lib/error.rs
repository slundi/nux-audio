/// Domain-level errors for the `nux_audio` library crate.
#[derive(thiserror::Error, Debug)]
pub enum NuxAudioError {
    // ── Hex utility ──────────────────────────────────────────────────────────
    #[error("hex string must have an even number of characters, got {0}")]
    OddLengthHex(usize),

    #[error("invalid hex digit in '{chunk}': {source}")]
    InvalidHexDigit {
        chunk: String,
        #[source]
        source: std::num::ParseIntError,
    },

    // ── SysEx framing ─────────────────────────────────────────────────────────
    #[error("SysEx message is empty")]
    SysExEmpty,

    #[error("not a SysEx message (must start with 0xF0 and end with 0xF7)")]
    SysExInvalidFraming,

    #[error("SysEx message too short")]
    SysExTooShort,

    // ── Scene control ─────────────────────────────────────────────────────────
    #[error("invalid scene number {0}: must be between 1 and 3")]
    InvalidSceneNumber(u8),
}
