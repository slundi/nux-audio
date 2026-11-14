//! Device session service.
//!
//! [`NuxSession`] owns a connected [`MidiTransport`], the detected [`Device`] and its
//! firmware generation, and exposes every device operation (bypass, knob, model,
//! preset fetch) as a typed async method. The CLI (and later the TUI) are thin
//! front-ends over it: they parse input, call the session, and render results.
//!
//! Generic over the transport so all logic is unit-tested with `MockMidi`.

use crate::device::{Device, extract_version};
use crate::effects::{
    CC_BYPASS_OFF, CC_BYPASS_ON, EffectCategory, FirmwareGen, find_knob_def, find_model_by_name,
};
use crate::error::SessionError;
use crate::midi::{MidiConnection, MidiTransport};
use crate::preset::Preset;
use nux_audio::nux_sys_ex_commands::{
    CURRENT_PRESET_BASIC, DEVICE_VERSION_REQUEST, mg30_change_effect_model, mg30_request_preset,
};
use std::time::Duration;

#[cfg(test)]
mod tests;

/// Header of the device version response (`F0 43 58 10 …`).
const VERSION_HEADER: [u8; 4] = [0xF0, 0x43, 0x58, 0x10];
/// Header of the current-preset basic response (preset number + active scene).
const BASIC_HEADER: [u8; 6] = [0xF0, 0x43, 0x58, 0x70, 0x15, 0x02];
/// Header of the preset detail response (name, tempo, chain, scenes).
const DETAIL_HEADER: [u8; 6] = [0xF0, 0x43, 0x58, 0x70, 0x0B, 0x02];
/// How long to wait for any device response.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(3);
/// Highest addressable preset slot (7-bit SysEx data byte).
const MAX_PRESET_SLOT: u8 = 0x7F;

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
    firmware: String,
    fw_gen: FirmwareGen,
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
    /// Query the firmware version over `transport` and detect the hardware generation.
    ///
    /// Must complete before any other operation so that version-dependent catalog
    /// lookups (e.g. Gen2-only effect models) use the right table.
    pub async fn handshake(mut transport: T, device: Device) -> Result<Self, SessionError> {
        let response = transport
            .request_sysex(&DEVICE_VERSION_REQUEST, &VERSION_HEADER, RESPONSE_TIMEOUT)
            .await
            .map_err(|source| SessionError::Midi {
                operation: "version query",
                source,
            })?;
        let firmware = extract_version(&response).map_err(|source| SessionError::Parse {
            response: "firmware version",
            source,
        })?;
        let fw_gen = FirmwareGen::from_version_str(&firmware);
        Ok(Self {
            transport,
            device,
            firmware,
            fw_gen,
        })
    }

    /// The connected device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Firmware version string reported by the device (e.g. `"v602"`).
    pub fn firmware(&self) -> &str {
        &self.firmware
    }

    /// Hardware generation derived from [`firmware`](Self::firmware).
    pub fn fw_gen(&self) -> FirmwareGen {
        self.fw_gen
    }

    /// Activate or bypass an effect slot. Returns the `(cc, value)` sent.
    pub async fn set_bypass(
        &mut self,
        category: EffectCategory,
        state: BypassState,
    ) -> Result<(u8, u8), SessionError> {
        let cc = category
            .bypass_cc()
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
                    find_knob_def(category, name).ok_or_else(|| SessionError::UnknownKnob {
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
    pub fn resolve_model(
        &self,
        category: EffectCategory,
        model: &ModelRef,
    ) -> Result<ModelChange, SessionError> {
        match model {
            ModelRef::Id(id) => Ok(ModelChange {
                id: *id,
                title: None,
            }),
            ModelRef::Name(name) => find_model_by_name(category, name, self.fw_gen)
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
            .send_sysex(&mg30_change_effect_model(category as u8, change.id))
            .map_err(|source| SessionError::Midi {
                operation: "model change",
                source,
            })?;
        Ok(change)
    }

    /// Raw current-preset basic response (preset number + active scene).
    pub async fn current_preset_raw(&mut self) -> Result<Vec<u8>, SessionError> {
        self.request("current preset query", &CURRENT_PRESET_BASIC, &BASIC_HEADER)
            .await
    }

    /// Raw preset detail response for `slot` (0–127).
    pub async fn preset_detail_raw(&mut self, slot: u8) -> Result<Vec<u8>, SessionError> {
        if slot > MAX_PRESET_SLOT {
            return Err(SessionError::SlotOutOfRange(slot));
        }
        self.request(
            "preset detail query",
            &mg30_request_preset(slot),
            &DETAIL_HEADER,
        )
        .await
    }

    /// Currently loaded preset: only `number` and `active_scene_number` are filled.
    pub async fn current_preset(&mut self) -> Result<Preset, SessionError> {
        let response = self.current_preset_raw().await?;
        Preset::extract_current_preset_basic_data(&response).map_err(|source| SessionError::Parse {
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
        let mut preset =
            Preset::extract_current_preset_detail_data(&response).map_err(|source| {
                SessionError::Parse {
                    response: "preset detail",
                    source,
                }
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

    async fn request(
        &mut self,
        operation: &'static str,
        request: &[u8],
        response_header: &[u8],
    ) -> Result<Vec<u8>, SessionError> {
        self.transport
            .request_sysex(request, response_header, RESPONSE_TIMEOUT)
            .await
            .map_err(|source| SessionError::Midi { operation, source })
    }
}
