use crate::error::NuxAudioError;
use std::collections::HashMap;

// ── SysEx template system ─────────────────────────────────────────────────────
//
// SysEx payload layout (bytes sent to `MidiManager::send_sysex`):
//   [0x58, <opcode bytes…>, <data bytes…>]
//
// `send_sysex` automatically prepends 0xF0 0x43 and appends 0xF7, producing
// the full wire message:
//   0xF0  0x43  0x58  <opcode…>  <data…>  0xF7
//
// Templates and typed builders in this module all return payloads in this
// format so callers can pass the result directly to `send_sysex`.

/// A slot in a SysEx payload template — either a fixed byte or a named param.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateSlot {
    /// A fixed literal byte.
    Byte(u8),
    /// A named parameter filled at build time via [`SysExTemplate::build`].
    Param(&'static str),
}

/// A parameterized SysEx payload template.
///
/// Templates hold a mix of fixed bytes and named [`TemplateSlot::Param`] slots.
/// Call [`build`](SysExTemplate::build) with a `name → value` map to get the
/// concrete payload ready for [`MidiManager::send_sysex`].
///
/// # Example
/// ```
/// use nux_audio::nux_sys_ex_commands::{SysExTemplate, TemplateSlot};
/// use std::collections::HashMap;
///
/// let t = SysExTemplate {
///     description: "MG-30 preset change",
///     slots: vec![
///         TemplateSlot::Byte(0x58),
///         TemplateSlot::Byte(0x09),
///         TemplateSlot::Byte(0x20),
///         TemplateSlot::Param("preset"),
///         TemplateSlot::Param("scene"),
///     ],
/// };
/// let payload = t.build(&HashMap::from([("preset", 5u8), ("scene", 0u8)])).unwrap();
/// assert_eq!(payload, vec![0x58, 0x09, 0x20, 5, 0]);
/// ```
#[derive(Debug, Clone)]
pub struct SysExTemplate {
    pub description: &'static str,
    pub slots: Vec<TemplateSlot>,
}

impl SysExTemplate {
    /// Build the payload by substituting named parameters.
    ///
    /// Returns [`NuxAudioError::MissingTemplateParam`] if any `Param` slot has
    /// no matching entry in `params`.
    pub fn build(&self, params: &HashMap<&str, u8>) -> Result<Vec<u8>, NuxAudioError> {
        let mut out = Vec::with_capacity(self.slots.len());
        for slot in &self.slots {
            match slot {
                TemplateSlot::Byte(b) => out.push(*b),
                TemplateSlot::Param(name) => match params.get(name) {
                    Some(&v) => out.push(v),
                    None => return Err(NuxAudioError::MissingTemplateParam(name)),
                },
            }
        }
        Ok(out)
    }
}

// ── Fixed request payloads ────────────────────────────────────────────────────

/// Query device firmware version. Send with `send_sysex`.
pub const DEVICE_VERSION_REQUEST: [u8; 2] = [0x58, 0x00];

/// Request current-preset basic data. Send with `send_sysex`.
pub const CURRENT_PRESET_BASIC: [u8; 4] = [0x58, 0x70, 0x15, 0x00];

/// Request updated effect order. Send with `send_sysex`.
pub const REQUEST_UPDATED_EFFECT_ORDER_COMMAND: [u8; 4] = [0x58, 0x70, 0x0C, 0x00];

// ── Response patterns (full wire messages including F0 / F7) ─────────────────

pub const DEVICE_VERSION_RESPONSE_PATTERN: [u8; 4] = [0xf0, 0x43, 0x58, 0x10];
pub const CURRENT_PRESET_BASIC_RESPONSE_PATTERN: [u8; 6] = [0xf0, 0x43, 0x58, 0x70, 0x15, 0x02];
pub const CURRENT_PRESET_DETAIL_RESPONSE_PATTERN: [u8; 6] = [0xf0, 0x43, 0x58, 0x70, 0x0b, 0x02];
pub const UPDATED_EFFECTS_ORDER_RESPONSE_PATTERN: [u8; 6] = [0xf0, 0x43, 0x58, 0x70, 0x0c, 0x02];
pub const PRESET_CHANGED_RESPONSE_PATTERN: [u8; 1] = [0xc0];
pub const SCENE_CHANGED_RESPONSE_PATTERN: [u8; 2] = [0xb0, 0x4f];
pub const EFFECT_CHANGED_RESPONSE_PATTERN: [u8; 1] = [0xb0];
pub const EFFECT_ORDER_CHANGED_RESPONSE_PATTERN: [u8; 7] =
    [0xf0, 0x43, 0x58, 0x70, 0x7e, 0x02, 0x0d];

// ── CC utilities ──────────────────────────────────────────────────────────────

/// Build a CC scene-change message for the MG-30.
///
/// `number` must be 1–3 (scenes A, B, C).  Returns a 3-byte raw MIDI message
/// `[0xB0, 0x4F, number]` that can be sent directly over MIDI (not via `send_sysex`).
pub fn change_scene(number: u8) -> Result<[u8; 3], NuxAudioError> {
    if !(1..=3).contains(&number) {
        return Err(NuxAudioError::InvalidSceneNumber(number));
    }
    Ok([0xB0, 0x4F, number])
}

// ── SysEx utility ─────────────────────────────────────────────────────────────

/// Wrap `message` bytes with `0xF0` / `0xF7` framing (no manufacturer byte).
pub fn build_sysex_message(message: &[u8]) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(message.len() + 2);
    result.push(0xf0);
    result.extend_from_slice(message);
    result.push(0xf7);
    result
}

/// Strip `0xF0` / manufacturer-ID / `0xF7` framing from a received SysEx message.
pub fn extract_sysex_message(message: &[u8]) -> Result<&[u8], NuxAudioError> {
    if message.is_empty() {
        return Err(NuxAudioError::SysExEmpty);
    }
    if message[0] != 0xf0 || message[message.len() - 1] != 0xf7 {
        return Err(NuxAudioError::SysExInvalidFraming);
    }
    if message.len() < 3 {
        return Err(NuxAudioError::SysExTooShort);
    }
    // let manufacturer_id = message[1];
    let data = &message[2..message.len() - 1]; // exclude F0, F7 and manufacturer ID
    Ok(data)
}

// ── Looper ────────────────────────────────────────────────────────────────────

/// MG-30 looper operation codes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LooperCommand {
    Stop = 0x00,
    Record = 0x01,
    Play = 0x02,
    Overdub = 0x03,
}

// ── MG-30 parameterized payload builders ──────────────────────────────────────

/// Request full preset data for slot `index` (0x00–0x7F).
///
/// Send with `manager.send_sysex(&mg30_request_preset(index))`.
pub fn mg30_request_preset(index: u8) -> [u8; 12] {
    [
        0x58, 0x70, 0x0B, 0x00, index, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]
}

/// Change to preset `index` (0x00–0x7F) and scene `scene` (0x00–0x03) on MG-30.
///
/// Scene 0 = A, 1 = B, 2 = C, 3 = D.
/// Send with `manager.send_sysex(&mg30_change_preset(index, scene)?)`.
pub fn mg30_change_preset(index: u8, scene: u8) -> Result<[u8; 5], NuxAudioError> {
    if index > 0x7F {
        return Err(NuxAudioError::InvalidPresetIndex(index));
    }
    if scene > 3 {
        return Err(NuxAudioError::InvalidSceneNumber(scene));
    }
    Ok([0x58, 0x09, 0x20, index, scene])
}

/// Save the device's current edit buffer to preset slot `index` (0x00–0x7F).
///
/// Send with `manager.send_sysex(&mg30_save_preset(index))`.
pub fn mg30_save_preset(index: u8) -> [u8; 12] {
    [
        0x58, 0x70, 0x7E, 0x02, 0x00, 0x00, index, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]
}

/// Send a looper command to the MG-30.
///
/// Send with `manager.send_sysex(&mg30_looper_control(cmd))`.
pub fn mg30_looper_control(cmd: LooperCommand) -> [u8; 4] {
    [0x58, 0x09, 0x70, cmd as u8]
}

/// Set the MG-30 effect chain order.
///
/// `slot_ids` contains one byte per effect slot in the desired signal-chain order.
/// Send with `manager.send_sysex(&mg30_set_effect_order(&ids))`.
pub fn mg30_set_effect_order(slot_ids: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(5 + slot_ids.len());
    payload.extend_from_slice(&[0x58, 0x70, 0x0D, 0x01]);
    payload.extend_from_slice(slot_ids);
    payload.push(0x00);
    payload
}

/// Switch the active model in an MG-30 effect slot.
///
/// `category_id`: the `EffectCategory` discriminant (`EffectCategory as u8`):
///   Wah=0, Comp=1, Efx=2, Amp=3, Eq=4, Gate=5, Mod=6, Delay=7, Reverb=8, Ir=9, Sr=10.
/// `model_id`: the model byte from the static catalog (e.g. `0x45` for a specific amp).
///
/// # Wire format (provisional — not yet verified against hardware captures)
///
/// Send with `manager.send_sysex(&mg30_change_effect_model(cat, id))`.
/// The full wire message will be: `F0 43 58 09 30 <category_id> <model_id> F7`
pub fn mg30_change_effect_model(category_id: u8, model_id: u8) -> [u8; 5] {
    [0x58, 0x09, 0x30, category_id, model_id]
}

// ── Mighty series parameterized payload builders ──────────────────────────────

/// Change preset on a Mighty series device over MIDI.
///
/// `device_id`: 0x01 (Plug MP-2), 0x02 (Air MA-1), 0x03 (Plug Pro MP-3), 0x04 (Space MS-1).
/// `index`: 0x00–0x06 (7 preset slots).
///
/// Send with `manager.send_sysex(&mighty_change_preset(device_id, index)?)`.
pub fn mighty_change_preset(device_id: u8, index: u8) -> Result<[u8; 4], NuxAudioError> {
    if !(0x01..=0x04).contains(&device_id) {
        return Err(NuxAudioError::InvalidDeviceId(device_id));
    }
    if index > 0x06 {
        return Err(NuxAudioError::InvalidPresetIndex(index));
    }
    Ok([0x58, device_id, 0x00, index])
}

// ── Predefined templates ──────────────────────────────────────────────────────

/// Ready-made [`SysExTemplate`] constructors for common operations.
///
/// These templates produce the same payloads as the typed builder functions
/// above, but are useful when the command needs to be stored or composed
/// dynamically (e.g. loaded from a YAML file).
pub mod templates {
    use super::{SysExTemplate, TemplateSlot};

    /// MG-30: change to a specific preset and scene.
    ///
    /// Parameters: `"preset"` (0x00–0x7F), `"scene"` (0x00–0x03).
    pub fn mg30_preset_change() -> SysExTemplate {
        SysExTemplate {
            description: "MG-30 preset change (preset 0x00–0x7F, scene 0x00–0x03)",
            slots: vec![
                TemplateSlot::Byte(0x58),
                TemplateSlot::Byte(0x09),
                TemplateSlot::Byte(0x20),
                TemplateSlot::Param("preset"),
                TemplateSlot::Param("scene"),
            ],
        }
    }

    /// MG-30: looper control.
    ///
    /// Parameter: `"command"` — use [`LooperCommand`](super::LooperCommand) as `u8`.
    pub fn mg30_looper() -> SysExTemplate {
        SysExTemplate {
            description: "MG-30 looper control (0=stop, 1=record, 2=play, 3=overdub)",
            slots: vec![
                TemplateSlot::Byte(0x58),
                TemplateSlot::Byte(0x09),
                TemplateSlot::Byte(0x70),
                TemplateSlot::Param("command"),
            ],
        }
    }

    /// MG-30: request full preset data for a slot.
    ///
    /// Parameter: `"index"` (0x00–0x7F).
    pub fn mg30_request_preset() -> SysExTemplate {
        SysExTemplate {
            description: "MG-30 request preset data by slot index",
            slots: vec![
                TemplateSlot::Byte(0x58),
                TemplateSlot::Byte(0x70),
                TemplateSlot::Byte(0x0B),
                TemplateSlot::Byte(0x00),
                TemplateSlot::Param("index"),
                TemplateSlot::Byte(0x00),
                TemplateSlot::Byte(0x00),
                TemplateSlot::Byte(0x00),
                TemplateSlot::Byte(0x00),
                TemplateSlot::Byte(0x00),
                TemplateSlot::Byte(0x00),
                TemplateSlot::Byte(0x00),
            ],
        }
    }

    /// MG-30: switch the active model within an effect slot [provisional].
    ///
    /// Parameters: `"category_id"` (`EffectCategory as u8`), `"model_id"` (catalog byte).
    pub fn mg30_effect_model_change() -> SysExTemplate {
        SysExTemplate {
            description: "MG-30 effect model switch [provisional — unverified against hardware]",
            slots: vec![
                TemplateSlot::Byte(0x58),
                TemplateSlot::Byte(0x09),
                TemplateSlot::Byte(0x30),
                TemplateSlot::Param("category_id"),
                TemplateSlot::Param("model_id"),
            ],
        }
    }

    /// Mighty series (Plug / Air / Plug Pro / Space): change preset.
    ///
    /// Parameters: `"device_id"` (0x01–0x04), `"preset"` (0x00–0x06).
    pub fn mighty_preset_change() -> SysExTemplate {
        SysExTemplate {
            description: "Mighty series preset change (device_id 0x01–0x04, preset 0x00–0x06)",
            slots: vec![
                TemplateSlot::Byte(0x58),
                TemplateSlot::Param("device_id"),
                TemplateSlot::Byte(0x00),
                TemplateSlot::Param("preset"),
            ],
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_scene() {
        assert!(change_scene(0).is_err());
        assert!(change_scene(4).is_err());
        assert_eq!(change_scene(1).unwrap(), [0xB0, 0x4F, 1]);
        assert_eq!(change_scene(2).unwrap(), [0xB0, 0x4F, 2]);
        assert_eq!(change_scene(3).unwrap(), [0xB0, 0x4F, 3]);
    }

    #[test]
    fn test_mg30_request_preset() {
        let msg = mg30_request_preset(0x05);
        assert_eq!(msg[0], 0x58);
        assert_eq!(&msg[1..4], &[0x70, 0x0B, 0x00]);
        assert_eq!(msg[4], 0x05);
        assert_eq!(&msg[5..], &[0u8; 7]);
    }

    #[test]
    fn test_mg30_change_preset() {
        assert!(mg30_change_preset(0x80, 0).is_err()); // index out of range
        assert!(mg30_change_preset(0, 4).is_err()); // scene out of range
        assert_eq!(mg30_change_preset(3, 1).unwrap(), [0x58, 0x09, 0x20, 3, 1]);
        assert_eq!(
            mg30_change_preset(0x7F, 3).unwrap(),
            [0x58, 0x09, 0x20, 0x7F, 3]
        );
    }

    #[test]
    fn test_mg30_save_preset() {
        let msg = mg30_save_preset(10);
        assert_eq!(&msg[..6], &[0x58, 0x70, 0x7E, 0x02, 0x00, 0x00]);
        assert_eq!(msg[6], 10);
    }

    #[test]
    fn test_mg30_looper_control() {
        assert_eq!(
            mg30_looper_control(LooperCommand::Stop),
            [0x58, 0x09, 0x70, 0x00]
        );
        assert_eq!(
            mg30_looper_control(LooperCommand::Record),
            [0x58, 0x09, 0x70, 0x01]
        );
        assert_eq!(
            mg30_looper_control(LooperCommand::Play),
            [0x58, 0x09, 0x70, 0x02]
        );
        assert_eq!(
            mg30_looper_control(LooperCommand::Overdub),
            [0x58, 0x09, 0x70, 0x03]
        );
    }

    #[test]
    fn test_mg30_set_effect_order() {
        let order = mg30_set_effect_order(&[0, 1, 2, 3]);
        assert_eq!(&order[..4], &[0x58, 0x70, 0x0D, 0x01]);
        assert_eq!(&order[4..8], &[0, 1, 2, 3]);
        assert_eq!(*order.last().unwrap(), 0x00);
    }

    #[test]
    fn test_mighty_change_preset() {
        assert!(mighty_change_preset(0x00, 0).is_err()); // device_id too low
        assert!(mighty_change_preset(0x05, 0).is_err()); // device_id too high
        assert!(mighty_change_preset(0x01, 7).is_err()); // preset out of range
        assert_eq!(
            mighty_change_preset(0x02, 3).unwrap(),
            [0x58, 0x02, 0x00, 3]
        );
        assert_eq!(
            mighty_change_preset(0x03, 0x06).unwrap(),
            [0x58, 0x03, 0x00, 0x06]
        );
    }

    #[test]
    fn test_template_build() {
        let t = templates::mg30_preset_change();
        let msg = t
            .build(&HashMap::from([("preset", 7u8), ("scene", 2u8)]))
            .unwrap();
        assert_eq!(msg, vec![0x58, 0x09, 0x20, 7, 2]);
    }

    #[test]
    fn test_template_missing_param() {
        let t = templates::mg30_preset_change();
        let err = t.build(&HashMap::from([("preset", 7u8)])); // "scene" missing
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("scene"));
    }

    #[test]
    fn test_mg30_change_effect_model() {
        // Provisional format: [0x58, 0x09, 0x30, category_id, model_id]
        assert_eq!(
            mg30_change_effect_model(3, 0x45),
            [0x58, 0x09, 0x30, 3, 0x45]
        );
        // Reverb slot (8) with model 0x21
        assert_eq!(
            mg30_change_effect_model(8, 0x21),
            [0x58, 0x09, 0x30, 8, 0x21]
        );
    }

    #[test]
    fn test_template_matches_typed_builder() {
        // Template and typed builder must agree on the wire bytes.
        let t = templates::mg30_preset_change();
        let from_template = t
            .build(&HashMap::from([("preset", 5u8), ("scene", 1u8)]))
            .unwrap();
        let from_builder = mg30_change_preset(5, 1).unwrap().to_vec();
        assert_eq!(from_template, from_builder);

        let t2 = templates::mg30_looper();
        let from_template2 = t2
            .build(&HashMap::from([("command", LooperCommand::Play as u8)]))
            .unwrap();
        let from_builder2 = mg30_looper_control(LooperCommand::Play).to_vec();
        assert_eq!(from_template2, from_builder2);

        let t3 = templates::mg30_request_preset();
        let from_template3 = t3.build(&HashMap::from([("index", 4u8)])).unwrap();
        let from_builder3 = mg30_request_preset(4).to_vec();
        assert_eq!(from_template3, from_builder3);

        let t4 = templates::mighty_preset_change();
        let from_template4 = t4
            .build(&HashMap::from([("device_id", 0x01u8), ("preset", 3u8)]))
            .unwrap();
        let from_builder4 = mighty_change_preset(0x01, 3).unwrap().to_vec();
        assert_eq!(from_template4, from_builder4);

        let t5 = templates::mg30_effect_model_change();
        let from_template5 = t5
            .build(&HashMap::from([("category_id", 3u8), ("model_id", 0x45u8)]))
            .unwrap();
        let from_builder5 = mg30_change_effect_model(3, 0x45).to_vec();
        assert_eq!(from_template5, from_builder5);
    }
}
