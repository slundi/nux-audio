//! Device drivers: everything that differs between device families or firmware
//! generations, behind one trait.
//!
//! A front-end (the binary's session service) never hard-codes SysEx payloads,
//! response layouts or catalog generations. It:
//!
//! 1. picks a [`DriverKind`] from the registry model code ([`DriverKind::for_model`]),
//! 2. sends [`DriverKind::version_request`] and builds the driver from the reply
//!    ([`DriverKind::from_version_response`]), which fixes the firmware generation,
//! 3. performs every further operation through the returned [`DeviceDriver`].
//!
//! Drivers are pure: they build requests and decode responses, the caller does the I/O.

pub mod mg30;

use crate::effects::{EffectCategory, EffectOption, FirmwareGen, KnobDef};
use crate::error::NuxAudioError;
use crate::preset::Preset;

/// A SysEx request paired with the header of the response that answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Payload to send, without `F0 43` / `F7` framing (the transport adds it).
    pub payload: Vec<u8>,
    /// Leading bytes (framing included) identifying the matching response.
    pub response_header: &'static [u8],
}

/// Protocol of one device family at one firmware generation.
///
/// Obtained from [`DriverKind::from_version_response`] once the firmware is known.
pub trait DeviceDriver: Send + Sync {
    /// Registry model code this driver handles (e.g. `"MG-30"`).
    fn model_code(&self) -> &'static str;

    /// Firmware version string reported by the device (e.g. `"v602"`).
    fn firmware(&self) -> &str;

    /// Hardware generation, for families that have several (`None` otherwise).
    fn firmware_gen(&self) -> Option<FirmwareGen>;

    /// CC number toggling the bypass of `category`, if the slot has one.
    fn bypass_cc(&self, category: EffectCategory) -> Option<u8>;

    /// Knob definition for `knob_id` (e.g. `"gain"`) in `category`.
    fn knob(&self, category: EffectCategory, knob_id: &str) -> Option<&'static KnobDef>;

    /// Model `model_id` of `category`, if available on this firmware.
    fn model_by_id(&self, category: EffectCategory, model_id: u8) -> Option<&'static EffectOption>;

    /// Model of `category` whose title matches `name` (case-insensitive), if available
    /// on this firmware; returns its ID with it.
    fn model_by_name(
        &self,
        category: EffectCategory,
        name: &str,
    ) -> Option<(u8, &'static EffectOption)>;

    /// Payload switching `category` to model `model_id` (no response expected).
    fn change_model(&self, category: EffectCategory, model_id: u8) -> Vec<u8>;

    /// Request for the currently loaded preset (slot + active scene).
    fn current_preset_request(&self) -> Request;

    /// Decode the reply to [`current_preset_request`](Self::current_preset_request):
    /// only `number` and `active_scene_number` are filled.
    fn decode_current_preset(&self, response: &[u8]) -> Result<Preset, NuxAudioError>;

    /// Request for the full data of preset `slot`.
    ///
    /// Fails with [`NuxAudioError::InvalidPresetIndex`] when `slot` does not exist.
    fn preset_detail_request(&self, slot: u8) -> Result<Request, NuxAudioError>;

    /// Decode the reply to [`preset_detail_request`](Self::preset_detail_request).
    ///
    /// Fields whose position is not known for this firmware are left at their default.
    fn decode_preset_detail(&self, response: &[u8]) -> Result<Preset, NuxAudioError>;
}

/// Device families with a driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverKind {
    /// NUX MG-30 (all firmware generations).
    Mg30,
}

impl DriverKind {
    /// Driver family for a registry model code, or `None` if no driver exists yet.
    pub fn for_model(model_code: &str) -> Option<Self> {
        match model_code {
            mg30::MODEL_CODE => Some(Self::Mg30),
            _ => None,
        }
    }

    /// Request identifying the device and its firmware; send it first.
    pub fn version_request(self) -> Request {
        match self {
            Self::Mg30 => mg30::version_request(),
        }
    }

    /// Build the driver from the reply to [`version_request`](Self::version_request).
    pub fn from_version_response(
        self,
        response: &[u8],
    ) -> Result<Box<dyn DeviceDriver>, NuxAudioError> {
        match self {
            Self::Mg30 => Ok(Box::new(mg30::Mg30Driver::new(mg30::extract_version(
                response,
            )?))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version_reply(text: &[u8]) -> Vec<u8> {
        let mut reply = vec![0xF0, 0x43, 0x58, 0x10];
        reply.extend_from_slice(text);
        reply.push(0xF7);
        reply
    }

    #[test]
    fn for_model_knows_only_supported_families() {
        assert_eq!(DriverKind::for_model("MG-30"), Some(DriverKind::Mg30));
        assert_eq!(DriverKind::for_model("MP-3"), None);
        assert_eq!(DriverKind::for_model("mg-30"), None);
    }

    #[test]
    fn mg30_handshake_builds_generation_aware_driver() {
        let kind = DriverKind::for_model("MG-30").unwrap();
        let request = kind.version_request();
        assert_eq!(request.payload, vec![0x58, 0x00]);
        assert_eq!(request.response_header, &[0xF0, 0x43, 0x58, 0x10]);

        let driver = kind
            .from_version_response(&version_reply(b"v602 MG-30"))
            .unwrap();
        assert_eq!(driver.model_code(), "MG-30");
        assert_eq!(driver.firmware(), "v602");
        assert_eq!(driver.firmware_gen(), Some(FirmwareGen::Gen2));

        let gen1 = kind
            .from_version_response(&version_reply(b"v402 MG-30"))
            .unwrap();
        assert_eq!(gen1.firmware_gen(), Some(FirmwareGen::Gen1));
    }

    #[test]
    fn from_version_response_rejects_garbage() {
        assert!(matches!(
            DriverKind::Mg30.from_version_response(&version_reply(b"???")),
            Err(NuxAudioError::VersionParse(_))
        ));
    }
}
