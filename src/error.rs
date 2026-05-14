/// Domain-level errors for the nux-cli binary.
///
/// Covers parsing of SysEx responses from the device, effect catalog lookups,
/// and raw byte utilities.  CLI-specific errors live in `crate::cli::error`.
#[derive(thiserror::Error, Debug)]
pub enum NuxError {
    // ── Hex utility ──────────────────────────────────────────────────────────
    #[error("hex string must have an even number of characters, got {0}")]
    OddLengthHex(usize),

    #[error("invalid hex digit in '{chunk}': {source}")]
    InvalidHexDigit {
        chunk: String,
        #[source]
        source: std::num::ParseIntError,
    },

    // ── Device response parsing ───────────────────────────────────────────────
    #[error("SysEx response too short: {0} bytes (expected at least 6)")]
    ResponseTooShort(usize),

    #[error("cannot parse firmware version from '{0}'")]
    VersionParse(String),

    #[error("invalid UTF-8 in device response: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    // ── Effect catalog lookup ─────────────────────────────────────────────────
    #[error(
        "SysEx response too short for effect parsing: {actual} bytes (need at least {expected})"
    )]
    EffectResponseTooShort { expected: usize, actual: usize },

    #[error("unknown effect category byte 0x{0:02X}")]
    UnknownEffectCategory(u8),

    #[error("unknown model 0x{model_id:02X} for category 0x{category_byte:02X}")]
    UnknownEffectModel { category_byte: u8, model_id: u8 },

    // ── MIDI CC ───────────────────────────────────────────────────────────────
    #[error("CC value {0} exceeds MIDI maximum of 127")]
    CcValueOutOfRange(u8),

    #[error("effect slot {0:?} has no bypass CC (Vol slot has no bypass toggle)")]
    NoBypassCc(String),
}
