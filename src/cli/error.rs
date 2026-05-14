use crate::error::NuxError;

/// CLI-level errors for the `nux-cli` binary.
#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error("no supported Nux device detected — is the device connected?")]
    NoDeviceFound,

    #[error("multiple devices found — specify one with -d <model>: {0}")]
    MultipleDevices(String),

    #[error("device '{0}' not found — use `nux devices` to list ports")]
    DeviceNotFound(String),

    #[error("no MIDI input port for {0}")]
    NoMidiInput(String),

    #[error("no MIDI output port for {0}")]
    NoMidiOutput(String),

    #[error("cannot create MIDI manager: {0}")]
    MidiSetup(String),

    #[error("cannot connect to {device}: {detail}")]
    Connection { device: String, detail: String },

    #[error("failed to send message: {0}")]
    SendError(String),

    #[error("timeout: no response from device within {0} seconds")]
    Timeout(u64),

    #[error(transparent)]
    Parse(#[from] NuxError),

    #[error("internal task error: {0}")]
    TaskJoin(String),
}
