//! Test fixtures shared by the session and worker tests: a fake [`Device`] and
//! canned MG-30 SysEx responses (test builds only).

use crate::device::{Device, Manufacturer};
use nux_audio::nux_sys_ex_commands::{
    CURRENT_PRESET_BASIC_RESPONSE_PATTERN as BASIC_HEADER,
    CURRENT_PRESET_DETAIL_RESPONSE_PATTERN as DETAIL_HEADER,
    DEVICE_VERSION_RESPONSE_PATTERN as VERSION_HEADER,
};
use std::collections::HashMap;

/// A detected device with one input (index 1) and one output (index 2) port.
pub fn device(model: &str, name: &str) -> Device {
    Device {
        manufacturer: Manufacturer::Nux,
        model: model.to_owned(),
        name: name.to_owned(),
        device_id: 0x09,
        inputs: HashMap::from([(format!("{name} in"), 1)]),
        outputs: HashMap::from([(format!("{name} out"), 2)]),
    }
}

/// `F0 43 58 10 "<version> MG-30" F7` — what an MG-30 answers to the version request.
pub fn version_response(version: &str) -> Vec<u8> {
    let mut response = VERSION_HEADER.to_vec();
    response.extend_from_slice(format!("{version} MG-30").as_bytes());
    response.push(0xF7);
    response
}

/// Current-preset basic response for slot `number` and active scene `scene`.
pub fn basic_response(number: u8, scene: u8) -> Vec<u8> {
    let mut response = vec![0u8; 10];
    response[..6].copy_from_slice(&BASIC_HEADER);
    response[6] = number;
    response[9] = scene;
    response
}

/// GEN2 (fw ≥ 5) detail response: 222 bytes, BPM 120 (byte 147 = 0x70), sync on.
pub fn detail_response(number: u8) -> Vec<u8> {
    let mut response = vec![0u8; 222];
    response[..6].copy_from_slice(&DETAIL_HEADER);
    response[6] = number;
    response[144] = 0x06;
    response[147] = 0x70;
    response
}

/// GEN1 (fw 4.0.3) detail response: tempo 500 ms/beat = 7*64 + 52 at bytes 143–144.
pub fn detail_response_gen1(number: u8) -> Vec<u8> {
    let mut response = vec![0u8; 218];
    response[..6].copy_from_slice(&DETAIL_HEADER);
    response[6] = number;
    response[143] = 7;
    response[144] = 52;
    response
}
