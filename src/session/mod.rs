//! Device session service.
//!
//! [`NuxSession`] owns a connected [`MidiTransport`], the detected [`Device`] and the
//! [`DeviceDriver`] chosen for it, and exposes every device operation (bypass, knob,
//! model, preset fetch) as a typed async method. The CLI (and later the TUI) are thin
//! front-ends over it: they parse input, call the session, and render results.
//!
//! The session does the I/O and error mapping; everything device- or
//! firmware-specific (payloads, layouts, catalogs) is delegated to the driver.
//! Generic over the transport so all logic is unit-tested with `MockMidi`.

use crate::device::Device;
use crate::error::SessionError;
use crate::midi::{MidiConnection, MidiTransport};
use nux_audio::driver::{DeviceDriver, DriverKind, Request};
use nux_audio::effects::{CC_BYPASS_OFF, CC_BYPASS_ON, EffectCategory};
use nux_audio::preset::Preset;
use std::time::Duration;

#[cfg(test)]
mod tests;

/// How long to wait for any device response.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(3);

// ── Operation inputs / outputs ────────────────────────────────────────────────

/// Whether to activate or bypass an effect slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BypassState {
    On,
    Off,
}

/// A knob addressed either by raw CC number or by catalog name (e.g. `"gain"`).
#[derive(Debug, Clone)]
pub enum KnobIdentifier {
    Id(u8),
    Name(String),
}

/// One knob assignment: `knob=value`.
#[derive(Debug, Clone)]
pub struct EffectParameter {
    pub knob: KnobIdentifier,
    pub value: u8,
}

impl std::fmt::Display for EffectParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.knob {
            KnobIdentifier::Id(id) => write!(f, "{}={}", id, self.value),
            KnobIdentifier::Name(name) => write!(f, "{}={}", name, self.value),
        }
    }
}

/// An effect model addressed either by raw model ID or by catalog name (e.g. `"Hall"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelRef {
    Id(u8),
    Name(String),
}

/// Result of a model switch: the model ID sent, and its catalog title when known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelChange {
    pub id: u8,
    pub title: Option<String>,
}

// ── Device selection ──────────────────────────────────────────────────────────

/// Pick the target device among `devices`.
///
/// With `name`, match case-insensitively on the port-derived name (substring) or
/// the exact model code. Without it, exactly one device must be present.
pub fn select_device(mut devices: Vec<Device>, name: Option<&str>) -> Result<Device, SessionError> {
    if let Some(name) = name {
        let wanted = name.to_lowercase();
        return match devices.iter().position(|device| {
            device.name.to_lowercase().contains(&wanted) || device.model.to_lowercase() == wanted
        }) {
            Some(index) => Ok(devices.swap_remove(index)),
            None => Err(SessionError::DeviceNotFound(name.to_owned())),
        };
    }
    match devices.len() {
        0 => Err(SessionError::NoDeviceFound),
        1 => Ok(devices.swap_remove(0)),
        _ => Err(SessionError::MultipleDevices(
            devices
                .iter()
                .map(|device| device.model.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )),
    }
}

/// First `(input, output)` port indices of `device`.
pub fn device_ports(device: &Device) -> Result<(usize, usize), SessionError> {
    let missing = |direction| SessionError::MissingPort {
        direction,
        model: device.model.clone(),
    };
    let port_in = device
        .inputs
        .values()
        .copied()
        .next()
        .ok_or_else(|| missing("input"))?;
    let port_out = device
        .outputs
        .values()
        .copied()
        .next()
        .ok_or_else(|| missing("output"))?;
    Ok((port_in, port_out))
}

/// Detect connected devices, select one (see [`select_device`]) and return it
/// with its `(input, output)` port indices. Does not open any connection.
pub fn resolve_device(name: Option<&str>) -> Result<(Device, usize, usize), SessionError> {
    let device = select_device(Device::detect(), name)?;
    let (port_in, port_out) = device_ports(&device)?;
    Ok((device, port_in, port_out))
}

// ── Session ───────────────────────────────────────────────────────────────────

/// A connected, handshaken device. All device operations go through here.
pub struct NuxSession<T> {
    transport: T,
    device: Device,
    driver: Box<dyn DeviceDriver>,
}

impl NuxSession<MidiConnection> {
    /// Detect and select a device, open its MIDI ports, and run the handshake.
    pub async fn connect(device_name: Option<&str>) -> Result<Self, SessionError> {
        let (device, port_in, port_out) = resolve_device(device_name)?;
        let transport =
            MidiConnection::open(port_in, port_out).map_err(|source| SessionError::Connect {
                model: device.model.clone(),
                source,
            })?;
        Self::handshake(transport, device).await
    }
}

impl<T: MidiTransport> NuxSession<T> {
    /// Pick the driver for `device`, query its firmware version over `transport`, and
    /// build the generation-aware driver from the reply.
    ///
    /// Must complete before any other operation so that version-dependent catalog
    /// lookups and response layouts (e.g. GEN2 detail offsets) are the right ones.
    pub async fn handshake(mut transport: T, device: Device) -> Result<Self, SessionError> {
        let kind = DriverKind::for_model(&device.model)
            .ok_or_else(|| SessionError::UnsupportedDevice(device.model.clone()))?;
        let response = request(&mut transport, "version query", &kind.version_request()).await?;
        let driver =
            kind.from_version_response(&response)
                .map_err(|source| SessionError::Parse {
                    response: "firmware version",
                    source,
                })?;
        Ok(Self {
            transport,
            device,
            driver,
        })
    }

    /// The connected device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// The protocol driver for the connected device (firmware, generation, decoders).
    pub fn driver(&self) -> &dyn DeviceDriver {
        self.driver.as_ref()
    }

    /// Activate or bypass an effect slot. Returns the `(cc, value)` sent.
    pub async fn set_bypass(
        &mut self,
        category: EffectCategory,
        state: BypassState,
    ) -> Result<(u8, u8), SessionError> {
        let cc = self
            .driver
            .bypass_cc(category)
            .ok_or(SessionError::NoBypassCc(category.as_str()))?;
        let value = match state {
            BypassState::On => CC_BYPASS_ON,
            BypassState::Off => CC_BYPASS_OFF,
        };
        self.send_cc("bypass CC", cc, value).await?;
        Ok((cc, value))
    }

    /// Set one knob via CC. Returns the `(cc, value)` sent.
    ///
    /// Named knobs are resolved through the effect catalog and range-checked;
    /// numeric ids are sent as raw CC numbers without catalog checks (escape hatch).
    pub async fn set_knob(
        &mut self,
        category: EffectCategory,
        param: &EffectParameter,
    ) -> Result<(u8, u8), SessionError> {
        let cc = match &param.knob {
            KnobIdentifier::Id(id) => *id,
            KnobIdentifier::Name(name) => {
                let knob =
                    self.driver
                        .knob(category, name)
                        .ok_or_else(|| SessionError::UnknownKnob {
                            category: category.as_str(),
                            knob: name.clone(),
                        })?;
                if let Some(max) = knob.range_end
                    && param.value > max
                {
                    return Err(SessionError::KnobValueOutOfRange {
                        category: category.as_str(),
                        knob: name.clone(),
                        value: param.value,
                        max,
                    });
                }
                knob.ctrl
            }
        };
        self.send_cc("knob CC", cc, param.value).await?;
        Ok((cc, param.value))
    }

    /// Resolve `model` against this device's catalog (honouring the firmware generation).
    ///
    /// Raw IDs are accepted even when absent from the catalog (escape hatch); their
    /// title is filled in when the catalog knows them.
    pub fn resolve_model(
        &self,
        category: EffectCategory,
        model: &ModelRef,
    ) -> Result<ModelChange, SessionError> {
        match model {
            ModelRef::Id(id) => Ok(ModelChange {
                id: *id,
                title: self
                    .driver
                    .model_by_id(category, *id)
                    .map(|option| option.title.clone()),
            }),
            ModelRef::Name(name) => self
                .driver
                .model_by_name(category, name)
                .map(|(id, option)| ModelChange {
                    id,
                    title: Some(option.title.clone()),
                })
                .ok_or_else(|| SessionError::UnknownModel {
                    category: category.as_str(),
                    model: name.clone(),
                }),
        }
    }

    /// Switch the active model of an effect slot (provisional SysEx).
    pub fn set_model(
        &mut self,
        category: EffectCategory,
        model: &ModelRef,
    ) -> Result<ModelChange, SessionError> {
        let change = self.resolve_model(category, model)?;
        self.transport
            .send_sysex(&self.driver.change_model(category, change.id))
            .map_err(|source| SessionError::Midi {
                operation: "model change",
                source,
            })?;
        Ok(change)
    }

    /// Raw current-preset basic response (preset number + active scene).
    pub async fn current_preset_raw(&mut self) -> Result<Vec<u8>, SessionError> {
        let current = self.driver.current_preset_request();
        request(&mut self.transport, "current preset query", &current).await
    }

    /// Raw preset detail response for `slot`.
    pub async fn preset_detail_raw(&mut self, slot: u8) -> Result<Vec<u8>, SessionError> {
        let detail =
            self.driver
                .preset_detail_request(slot)
                .map_err(|source| SessionError::Rejected {
                    operation: "preset detail query",
                    source,
                })?;
        request(&mut self.transport, "preset detail query", &detail).await
    }

    /// Currently loaded preset: only `number` and `active_scene_number` are filled.
    pub async fn current_preset(&mut self) -> Result<Preset, SessionError> {
        let response = self.current_preset_raw().await?;
        self.driver
            .decode_current_preset(&response)
            .map_err(|source| SessionError::Parse {
                response: "current preset",
                source,
            })
    }

    /// Fetch and parse a full preset.
    ///
    /// With `slot`, that preset is read (active scene unknown → `None`). Without it,
    /// the currently loaded preset is read and its active scene filled in.
    pub async fn fetch_preset(&mut self, slot: Option<u8>) -> Result<Preset, SessionError> {
        let (slot, active_scene) = match slot {
            Some(slot) => (slot, None),
            None => {
                let current = self.current_preset().await?;
                (
                    current.number,
                    Some(current.active_scene_number.unwrap_or(0)),
                )
            }
        };
        let response = self.preset_detail_raw(slot).await?;
        let mut preset = self
            .driver
            .decode_preset_detail(&response)
            .map_err(|source| SessionError::Parse {
                response: "preset detail",
                source,
            })?;
        preset.active_scene_number = active_scene;
        Ok(preset)
    }

    async fn send_cc(
        &mut self,
        operation: &'static str,
        cc: u8,
        value: u8,
    ) -> Result<(), SessionError> {
        self.transport
            .send_cc(cc, value)
            .await
            .map_err(|source| SessionError::Midi { operation, source })
    }
}

/// Send a driver [`Request`] and wait for its response. A free function (not a
/// method) so the handshake can use it before the session exists.
async fn request<T: MidiTransport>(
    transport: &mut T,
    operation: &'static str,
    request: &Request,
) -> Result<Vec<u8>, SessionError> {
    transport
        .request_sysex(&request.payload, request.response_header, RESPONSE_TIMEOUT)
        .await
        .map_err(|source| SessionError::Midi { operation, source })
}
