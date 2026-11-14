//! Domain-level error types for the nux-cli binary.
//!
//! - [`NuxError`]: SysEx response parsing, effect catalog lookups, byte utilities.
//! - [`MidiError`]: MIDI backend setup, port connection, sending and response waiting.
//! - [`ExportError`]: preset file (de)serialization, schema versioning and file I/O.
//! - [`SessionError`]: device selection and device operations (`crate::session`).
//!
//! CLI-specific errors live in `crate::cli::error` and wrap these via `#[from]`.

use std::path::PathBuf;
use std::time::Duration;

/// Domain-level errors for the nux-cli binary.
///
/// Covers parsing of SysEx responses from the device, effect catalog lookups,
/// and raw byte utilities.
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
    #[error("SysEx response too short: {actual} bytes (expected at least {expected})")]
    ResponseTooShort { expected: usize, actual: usize },

    #[error("cannot parse firmware version from '{0}'")]
    VersionParse(String),

    #[error("invalid UTF-8 in device response: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("invalid byte 0x{0:02X} in preset name (expected printable ASCII)")]
    InvalidPresetName(u8),

    // ── Effect catalog lookup ─────────────────────────────────────────────────
    #[error(
        "SysEx response too short for effect parsing: {actual} bytes (need at least {expected})"
    )]
    EffectResponseTooShort { expected: usize, actual: usize },

    #[error("unknown effect category byte 0x{0:02X}")]
    UnknownEffectCategory(u8),

    #[error("unknown model 0x{model_id:02X} for category 0x{category_byte:02X}")]
    UnknownEffectModel { category_byte: u8, model_id: u8 },
}

/// Errors raised by the MIDI transport layer (`crate::midi`).
#[derive(thiserror::Error, Debug)]
pub enum MidiError {
    /// The MIDI backend (ALSA/JACK/CoreMIDI/WinMM) could not be initialized.
    #[error(transparent)]
    Init(#[from] midir::InitError),

    /// A port index does not match any currently available port.
    #[error("MIDI {direction} port index {index} out of range ({available} available)")]
    PortOutOfRange {
        /// `"input"` or `"output"`.
        direction: &'static str,
        index: usize,
        available: usize,
    },

    /// The backend could not report information (e.g. the name) of a port.
    #[error("cannot query MIDI port: {0}")]
    PortInfo(#[from] midir::PortInfoError),

    /// Opening a connection to a port failed.
    #[error("cannot connect to MIDI port '{port}': {kind}")]
    Connect {
        port: String,
        kind: midir::ConnectErrorKind,
    },

    /// A send was attempted before any output port was connected.
    #[error("no MIDI output connection established")]
    NotConnected,

    /// The backend rejected an outgoing message.
    #[error("MIDI send failed: {0}")]
    Send(#[from] midir::SendError),

    /// A CC value above the 7-bit MIDI limit was refused before sending.
    #[error("CC value {0} exceeds MIDI maximum of 127")]
    CcValueOutOfRange(u8),

    /// No message matching the expected response header arrived in time.
    #[error("no response from device within {0:?}")]
    Timeout(Duration),

    /// The incoming-message channel was closed (input connection dropped).
    #[error("MIDI dispatcher channel closed unexpectedly")]
    ChannelClosed,
}

/// Errors raised by the device session service (`crate::session`).
#[derive(thiserror::Error, Debug)]
pub enum SessionError {
    // ── Device selection ──────────────────────────────────────────────────────
    #[error("no supported Nux device detected — is the device connected?")]
    NoDeviceFound,

    #[error("multiple devices found — specify one with -d <model>: {0}")]
    MultipleDevices(String),

    #[error("device '{0}' not found — use `nux devices` to list ports")]
    DeviceNotFound(String),

    #[error("no MIDI {direction} port for {model}")]
    MissingPort {
        /// `"input"` or `"output"`.
        direction: &'static str,
        model: String,
    },

    // ── Transport ─────────────────────────────────────────────────────────────
    #[error("cannot connect to {model}: {source}")]
    Connect {
        model: String,
        #[source]
        source: MidiError,
    },

    /// A device operation failed at the MIDI level; `operation` says which one.
    #[error("{operation} failed: {source}")]
    Midi {
        operation: &'static str,
        #[source]
        source: MidiError,
    },

    /// A device response could not be decoded; `response` says which one.
    #[error("cannot parse {response}: {source}")]
    Parse {
        response: &'static str,
        #[source]
        source: NuxError,
    },

    // ── Catalog / argument validation ─────────────────────────────────────────
    #[error(
        "knob '{knob}' not found in {category} catalog — use a CC number directly (e.g. 22=80)"
    )]
    UnknownKnob {
        category: &'static str,
        knob: String,
    },

    #[error("value {value} out of range for {category} knob '{knob}' (0..={max})")]
    KnobValueOutOfRange {
        category: &'static str,
        knob: String,
        value: u8,
        max: u8,
    },

    #[error("the '{0}' slot has no bypass toggle")]
    NoBypassCc(&'static str),

    #[error("model '{model}' not found in {category} catalog")]
    UnknownModel {
        category: &'static str,
        model: String,
    },

    #[error("preset slot {0} out of range (0–127)")]
    SlotOutOfRange(u8),
}

/// Errors raised while exporting or importing preset files (`crate::export`).
#[derive(thiserror::Error, Debug)]
pub enum ExportError {
    #[error("cannot serialize preset: {0}")]
    Serialize(String),

    #[error("cannot deserialize preset: {0}")]
    Deserialize(String),

    #[error("preset file version {found} is no longer supported (minimum: {minimum})")]
    VersionTooOld { found: u32, minimum: u32 },

    #[error(
        "preset file version {found} requires a newer nux-cli (this build supports up to v{supported})"
    )]
    VersionTooNew { found: u32, supported: u32 },

    #[error("cannot write to '{}': {source}", path.display())]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot read from '{}': {source}", path.display())]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_error_port_out_of_range_message() {
        let err = MidiError::PortOutOfRange {
            direction: "output",
            index: 7,
            available: 2,
        };
        assert_eq!(
            err.to_string(),
            "MIDI output port index 7 out of range (2 available)"
        );
    }

    #[test]
    fn test_midi_error_from_midir_errors() {
        let init: MidiError = midir::InitError.into();
        assert!(matches!(init, MidiError::Init(_)));
        assert_eq!(init.to_string(), "MIDI support could not be initialized");

        let send: MidiError = midir::SendError::Other("boom").into();
        assert!(matches!(send, MidiError::Send(_)));
        assert_eq!(send.to_string(), "MIDI send failed: boom");

        let info: MidiError = midir::PortInfoError::CannotRetrievePortName.into();
        assert!(matches!(info, MidiError::PortInfo(_)));
    }

    #[test]
    fn test_midi_error_connect_message() {
        let err = MidiError::Connect {
            port: "NUX MG-30".to_owned(),
            kind: midir::ConnectErrorKind::InvalidPort,
        };
        assert_eq!(
            err.to_string(),
            "cannot connect to MIDI port 'NUX MG-30': invalid port"
        );
    }

    #[test]
    fn test_midi_error_timeout_message_includes_duration() {
        let err = MidiError::Timeout(Duration::from_secs(3));
        assert_eq!(err.to_string(), "no response from device within 3s");
    }

    #[test]
    fn test_session_error_keeps_operation_and_source() {
        use std::error::Error;

        let err = SessionError::Midi {
            operation: "version query",
            source: MidiError::Timeout(Duration::from_secs(3)),
        };
        assert_eq!(
            err.to_string(),
            "version query failed: no response from device within 3s"
        );
        assert!(err.source().is_some());

        let missing = SessionError::MissingPort {
            direction: "input",
            model: "MG-30".to_owned(),
        };
        assert_eq!(missing.to_string(), "no MIDI input port for MG-30");
    }

    #[test]
    fn test_export_error_version_messages() {
        let too_old = ExportError::VersionTooOld {
            found: 0,
            minimum: 1,
        };
        assert!(too_old.to_string().contains("no longer supported"));

        let too_new = ExportError::VersionTooNew {
            found: 9,
            supported: 1,
        };
        assert!(too_new.to_string().contains("newer nux-cli"));
        assert!(too_new.to_string().contains("up to v1"));
    }

    #[test]
    fn test_export_error_io_keeps_source() {
        use std::error::Error;

        let err = ExportError::ReadFile {
            path: PathBuf::from("/nope/preset.yaml"),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        };
        assert!(
            err.to_string()
                .starts_with("cannot read from '/nope/preset.yaml'")
        );
        assert!(err.source().is_some());
    }
}
