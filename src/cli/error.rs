//! CLI-level error type for the `nux-cli` binary; wraps domain errors via `#[from]`.

use crate::error::{ExportError, MidiError, NuxError, SessionError};

/// CLI-level errors for the `nux-cli` binary.
///
/// Every command returns `Result<(), CliError>`; `main` prints the error once
/// and exits with a non-zero status.
#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error(
        "unknown effect category '{0}' — try: wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir, vol"
    )]
    UnknownCategory(String),

    #[error("invalid hex model ID '{0}'")]
    InvalidModelId(String),

    /// Some of a multi-parameter command failed (each failure was already printed).
    #[error("{failed} of {total} parameter(s) failed")]
    PartialFailure { failed: usize, total: usize },

    #[error(transparent)]
    Session(#[from] SessionError),

    #[error(transparent)]
    Parse(#[from] NuxError),

    #[error(transparent)]
    Midi(#[from] MidiError),

    #[error(transparent)]
    Export(#[from] ExportError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_errors_convert_transparently() {
        let midi: CliError = MidiError::NotConnected.into();
        assert!(matches!(midi, CliError::Midi(_)));
        assert_eq!(midi.to_string(), MidiError::NotConnected.to_string());

        let export: CliError = ExportError::Serialize("bad".to_owned()).into();
        assert!(matches!(export, CliError::Export(_)));
        assert_eq!(export.to_string(), "cannot serialize preset: bad");

        let session: CliError = SessionError::NoDeviceFound.into();
        assert!(matches!(session, CliError::Session(_)));
        assert_eq!(session.to_string(), SessionError::NoDeviceFound.to_string());
    }

    #[test]
    fn test_partial_failure_message() {
        let err = CliError::PartialFailure {
            failed: 1,
            total: 3,
        };
        assert_eq!(err.to_string(), "1 of 3 parameter(s) failed");
    }
}
