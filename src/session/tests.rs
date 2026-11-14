//! Unit tests for the session service, driven through `MockMidi` canned responses.

use super::*;
use crate::device::Manufacturer;
use crate::error::MidiError;
use crate::midi::MockMidi;
use nux_audio::effects::FirmwareGen;
use nux_audio::error::NuxAudioError;
use nux_audio::nux_sys_ex_commands::{
    CURRENT_PRESET_BASIC_RESPONSE_PATTERN as BASIC_HEADER,
    CURRENT_PRESET_DETAIL_RESPONSE_PATTERN as DETAIL_HEADER,
    DEVICE_VERSION_RESPONSE_PATTERN as VERSION_HEADER,
};
use std::collections::HashMap;

// ── Fixtures ──────────────────────────────────────────────────────────────────

fn device(model: &str, name: &str) -> Device {
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
fn version_response(version: &str) -> Vec<u8> {
    let mut response = VERSION_HEADER.to_vec();
    response.extend_from_slice(format!("{version} MG-30").as_bytes());
    response.push(0xF7);
    response
}

fn basic_response(number: u8, scene: u8) -> Vec<u8> {
    let mut response = vec![0u8; 10];
    response[..6].copy_from_slice(&BASIC_HEADER);
    response[6] = number;
    response[9] = scene;
    response
}

/// GEN2 (fw ≥ 5) detail response: 222 bytes, BPM 120 (byte 147 = 0x70), sync on.
fn detail_response(number: u8) -> Vec<u8> {
    let mut response = vec![0u8; 222];
    response[..6].copy_from_slice(&DETAIL_HEADER);
    response[6] = number;
    response[144] = 0x06;
    response[147] = 0x70;
    response
}

/// GEN1 (fw 4.0.3) detail response: tempo 500 ms/beat = 7*64 + 52 at bytes 143–144.
fn detail_response_gen1(number: u8) -> Vec<u8> {
    let mut response = vec![0u8; 218];
    response[..6].copy_from_slice(&DETAIL_HEADER);
    response[6] = number;
    response[143] = 7;
    response[144] = 52;
    response
}

/// Handshaken Gen2 session whose transport will answer with `responses`
/// after the version reply. `sent` is cleared so tests only see their own traffic.
async fn session_with(responses: Vec<Vec<u8>>) -> NuxSession<MockMidi> {
    session_on_firmware("v602", responses).await
}

/// Like [`session_with`], for an arbitrary firmware version (e.g. `"v402"` = GEN1).
async fn session_on_firmware(firmware: &str, responses: Vec<Vec<u8>>) -> NuxSession<MockMidi> {
    let mut queue = vec![version_response(firmware)];
    queue.extend(responses);
    let mut session = NuxSession::handshake(
        MockMidi::with_responses(queue),
        device("MG-30", "NUX MG-30"),
    )
    .await
    .unwrap();
    session.transport.sent.clear();
    session
}

// ── Device selection ──────────────────────────────────────────────────────────

#[test]
fn select_device_single_without_name() {
    let picked = select_device(vec![device("MG-30", "NUX MG-30")], None).unwrap();
    assert_eq!(picked.model, "MG-30");
}

#[test]
fn select_device_none_found() {
    assert!(matches!(
        select_device(vec![], None),
        Err(SessionError::NoDeviceFound)
    ));
}

#[test]
fn select_device_multiple_requires_name() {
    let devices = vec![
        device("MG-30", "NUX MG-30"),
        device("MP-3", "NUX Mighty Plug Pro"),
    ];
    match select_device(devices, None) {
        Err(SessionError::MultipleDevices(list)) => {
            assert!(list.contains("MG-30") && list.contains("MP-3"));
        }
        other => panic!("expected MultipleDevices, got {:?}", other.map(|d| d.model)),
    }
}

#[test]
fn select_device_by_model_or_name_case_insensitive() {
    let devices = || {
        vec![
            device("MG-30", "NUX MG-30"),
            device("MP-3", "NUX Mighty Plug Pro"),
        ]
    };
    assert_eq!(
        select_device(devices(), Some("mp-3")).unwrap().model,
        "MP-3"
    );
    assert_eq!(
        select_device(devices(), Some("mighty")).unwrap().model,
        "MP-3"
    );
    assert!(matches!(
        select_device(devices(), Some("helix")),
        Err(SessionError::DeviceNotFound(name)) if name == "helix"
    ));
}

#[test]
fn device_ports_reports_missing_direction() {
    assert_eq!(device_ports(&device("MG-30", "NUX MG-30")).unwrap(), (1, 2));

    let mut no_input = device("MG-30", "NUX MG-30");
    no_input.inputs.clear();
    assert!(matches!(
        device_ports(&no_input),
        Err(SessionError::MissingPort {
            direction: "input",
            ..
        })
    ));

    let mut no_output = device("MG-30", "NUX MG-30");
    no_output.outputs.clear();
    assert!(matches!(
        device_ports(&no_output),
        Err(SessionError::MissingPort {
            direction: "output",
            ..
        })
    ));
}

// ── Handshake ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn handshake_detects_firmware_generation() {
    let mock = MockMidi::with_responses([version_response("v602")]);
    let session = NuxSession::handshake(mock, device("MG-30", "NUX MG-30"))
        .await
        .unwrap();
    assert_eq!(session.driver().firmware(), "v602");
    assert_eq!(session.driver().firmware_gen(), Some(FirmwareGen::Gen2));
    assert_eq!(session.driver().model_code(), "MG-30");
    assert_eq!(session.device().model, "MG-30");
    // The version request was sent with the standard F0 43 framing.
    assert_eq!(
        session.transport.sent,
        vec![vec![0xF0, 0x43, 0x58, 0x00, 0xF7]]
    );

    let gen1 = NuxSession::handshake(
        MockMidi::with_responses([version_response("v402")]),
        device("MG-30", "NUX MG-30"),
    )
    .await
    .unwrap();
    assert_eq!(gen1.driver().firmware_gen(), Some(FirmwareGen::Gen1));
}

#[tokio::test]
async fn handshake_refuses_device_without_driver() {
    // A Mighty Plug Pro is detected by the registry but has no driver yet: fail
    // before sending anything rather than speak MG-30 SysEx to it.
    let mock = MockMidi::with_responses([version_response("v602")]);
    match NuxSession::handshake(mock, device("MP-3", "NUX Mighty Plug Pro")).await {
        Err(SessionError::UnsupportedDevice(model)) => assert_eq!(model, "MP-3"),
        Err(other) => panic!("unexpected error: {other}"),
        Ok(_) => panic!("handshake should refuse an unsupported device"),
    }
}

#[tokio::test]
async fn handshake_timeout_names_the_operation() {
    let result = NuxSession::handshake(MockMidi::new(), device("MG-30", "NUX MG-30")).await;
    match result {
        Err(
            err @ SessionError::Midi {
                operation: "version query",
                source: MidiError::Timeout(_),
            },
        ) => assert!(err.to_string().starts_with("version query failed")),
        Err(other) => panic!("unexpected error: {other}"),
        Ok(_) => panic!("handshake should fail without a response"),
    }
}

#[tokio::test]
async fn handshake_rejects_unparsable_version() {
    let mut garbage = VERSION_HEADER.to_vec();
    garbage.extend_from_slice(b"???");
    garbage.push(0xF7);
    let result =
        NuxSession::handshake(MockMidi::with_responses([garbage]), device("MG-30", "x")).await;
    assert!(matches!(
        result,
        Err(SessionError::Parse {
            response: "firmware version",
            ..
        })
    ));
}

// ── Bypass ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn set_bypass_amp_on() {
    let mut session = session_with(vec![]).await;
    let (cc, value) = session
        .set_bypass(EffectCategory::Amp, BypassState::On)
        .await
        .unwrap();
    assert_eq!(cc, 3); // Amp bypass CC = discriminant 3
    assert_eq!(value, CC_BYPASS_ON);
    assert_eq!(session.transport.sent, vec![vec![0xB0, 3, CC_BYPASS_ON]]);
}

#[tokio::test]
async fn set_bypass_reverb_off() {
    let mut session = session_with(vec![]).await;
    let (cc, value) = session
        .set_bypass(EffectCategory::Reverb, BypassState::Off)
        .await
        .unwrap();
    assert_eq!(cc, 8); // Reverb bypass CC = discriminant 8
    assert_eq!(value, CC_BYPASS_OFF);
    assert_eq!(session.transport.sent, vec![vec![0xB0, 8, CC_BYPASS_OFF]]);
}

#[tokio::test]
async fn set_bypass_vol_has_no_cc() {
    let mut session = session_with(vec![]).await;
    let err = session
        .set_bypass(EffectCategory::Vol, BypassState::On)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("no bypass toggle"));
    assert!(session.transport.sent.is_empty());
}

#[tokio::test]
async fn set_bypass_propagates_midi_error() {
    let mut session = session_with(vec![]).await;
    session.transport.should_fail = true;
    let result = session
        .set_bypass(EffectCategory::Amp, BypassState::On)
        .await;
    assert!(matches!(
        result,
        Err(SessionError::Midi {
            operation: "bypass CC",
            source: MidiError::Send(_)
        })
    ));
}

// ── Knobs ─────────────────────────────────────────────────────────────────────

fn named(knob: &str, value: u8) -> EffectParameter {
    EffectParameter {
        knob: KnobIdentifier::Name(knob.to_owned()),
        value,
    }
}

fn numeric(cc: u8, value: u8) -> EffectParameter {
    EffectParameter {
        knob: KnobIdentifier::Id(cc),
        value,
    }
}

#[tokio::test]
async fn set_knob_named_resolves_cc() {
    let mut session = session_with(vec![]).await;
    let (cc, value) = session
        .set_knob(EffectCategory::Amp, &named("gain", 80))
        .await
        .unwrap();
    // Amp gain is CC22 per MIDI_CC_assignments.md
    assert_eq!((cc, value), (22, 80));
    assert_eq!(session.transport.sent, vec![vec![0xB0, 22, 80]]);
}

#[tokio::test]
async fn set_knob_numeric_cc_direct() {
    let mut session = session_with(vec![]).await;
    let (cc, value) = session
        .set_knob(EffectCategory::Delay, &numeric(52, 64))
        .await
        .unwrap();
    assert_eq!((cc, value), (52, 64));
    assert_eq!(session.transport.sent, vec![vec![0xB0, 52, 64]]);
}

#[tokio::test]
async fn set_knob_unknown_name_errors() {
    let mut session = session_with(vec![]).await;
    let err = session
        .set_knob(EffectCategory::Amp, &named("nonexistent", 50))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not found"));
    assert!(session.transport.sent.is_empty());
}

#[tokio::test]
async fn set_knob_propagates_midi_error() {
    let mut session = session_with(vec![]).await;
    session.transport.should_fail = true;
    let result = session
        .set_knob(EffectCategory::Amp, &numeric(22, 80))
        .await;
    assert!(matches!(
        result,
        Err(SessionError::Midi {
            operation: "knob CC",
            ..
        })
    ));
}

// Range-validation for named knobs — the "raw CC by numeric id" path
// intentionally bypasses this to remain an escape hatch.

#[tokio::test]
async fn set_knob_rejects_value_above_knob_range() {
    // IR "mic" is CC64 with range 0..=7 (DYN421/S57/…).
    let mut session = session_with(vec![]).await;
    let err = session
        .set_knob(EffectCategory::Ir, &named("mic", 100))
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("out of range"), "unexpected error: {err}");
    assert!(err.contains("0..=7"), "unexpected error: {err}");
    assert!(
        session.transport.sent.is_empty(),
        "invalid value must not be transmitted"
    );
}

#[tokio::test]
async fn set_knob_accepts_value_at_range_end() {
    let mut session = session_with(vec![]).await;
    let (cc, value) = session
        .set_knob(EffectCategory::Ir, &named("mic", 7))
        .await
        .unwrap();
    assert_eq!((cc, value), (64, 7));
}

#[tokio::test]
async fn set_knob_rejects_one_over_range_end() {
    // Boundary: gate mode range is 0..=6, so 7 must be rejected.
    let mut session = session_with(vec![]).await;
    let err = session
        .set_knob(EffectCategory::Gate, &named("mode", 7))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("0..=6"), "unexpected error: {err}");
    assert!(session.transport.sent.is_empty());
}

#[tokio::test]
async fn set_knob_numeric_id_skips_range_check() {
    // IR mic (CC64) has range 0..=7 — with a raw numeric id the catalog is
    // not consulted, so value 100 (still ≤ 127 MIDI limit) succeeds.
    let mut session = session_with(vec![]).await;
    let (cc, value) = session
        .set_knob(EffectCategory::Ir, &numeric(64, 100))
        .await
        .unwrap();
    assert_eq!((cc, value), (64, 100));
}

#[test]
fn effect_parameter_display() {
    assert_eq!(named("gain", 80).to_string(), "gain=80");
    assert_eq!(numeric(22, 5).to_string(), "22=5");
}

// ── Models ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn set_model_by_id_sends_sysex() {
    let mut session = session_with(vec![]).await;
    let change = session
        .set_model(EffectCategory::Amp, &ModelRef::Id(0x45))
        .unwrap();
    assert_eq!(
        change,
        ModelChange {
            id: 0x45,
            title: None
        }
    );
    let category = EffectCategory::Amp as u8;
    assert_eq!(
        session.transport.sent,
        vec![vec![0xF0, 0x43, 0x58, 0x09, 0x30, category, 0x45, 0xF7]]
    );
}

#[tokio::test]
async fn set_model_by_known_id_reports_title() {
    // Damp reverb (0x0C) exists on GEN2 only: titled there, raw-but-allowed on GEN1.
    let mut gen2 = session_with(vec![]).await;
    let change = gen2
        .set_model(EffectCategory::Reverb, &ModelRef::Id(0x0C))
        .unwrap();
    assert!(change.title.is_some());

    let mut gen1 = session_on_firmware("v402", vec![]).await;
    let change = gen1
        .set_model(EffectCategory::Reverb, &ModelRef::Id(0x0C))
        .unwrap();
    assert_eq!(change.title, None);
    assert_eq!(gen1.transport.sent.len(), 1);
}

#[tokio::test]
async fn set_model_by_name_follows_generation() {
    let mut gen1 = session_on_firmware("v402", vec![]).await;
    assert!(matches!(
        gen1.set_model(EffectCategory::Reverb, &ModelRef::Name("damp".to_owned())),
        Err(SessionError::UnknownModel { .. })
    ));
    assert!(gen1.transport.sent.is_empty());
}

#[tokio::test]
async fn set_model_by_name_resolves_title() {
    let mut session = session_with(vec![]).await;
    let change = session
        .set_model(EffectCategory::Reverb, &ModelRef::Name("hall".to_owned()))
        .unwrap();
    assert!(
        change
            .title
            .as_deref()
            .is_some_and(|title| title.eq_ignore_ascii_case("hall"))
    );
    assert_eq!(session.transport.sent.len(), 1);
    assert_eq!(session.transport.sent[0][6], change.id);
}

#[tokio::test]
async fn set_model_unknown_name_sends_nothing() {
    let mut session = session_with(vec![]).await;
    let result = session.set_model(EffectCategory::Reverb, &ModelRef::Name("nope".to_owned()));
    assert!(matches!(
        result,
        Err(SessionError::UnknownModel { model, .. }) if model == "nope"
    ));
    assert!(session.transport.sent.is_empty());
}

#[tokio::test]
async fn set_model_propagates_midi_error() {
    let mut session = session_with(vec![]).await;
    session.transport.should_fail = true;
    assert!(matches!(
        session.set_model(EffectCategory::Amp, &ModelRef::Id(1)),
        Err(SessionError::Midi {
            operation: "model change",
            ..
        })
    ));
}

// ── Presets ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn fetch_current_preset_reads_basic_then_detail() {
    let mut session = session_with(vec![basic_response(10, 2), detail_response(10)]).await;
    let preset = session.fetch_preset(None).await.unwrap();
    assert_eq!(preset.number, 10);
    assert_eq!(preset.tempo, 500);
    assert_eq!(preset.tempo_sync, Some(true));
    assert_eq!(preset.active_scene_number, Some(2));

    let sent = &session.transport.sent;
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0], vec![0xF0, 0x43, 0x58, 0x70, 0x15, 0x00, 0xF7]);
    // Detail request carries the slot read from the basic response.
    assert_eq!(&sent[1][2..5], &[0x58, 0x70, 0x0B]);
    assert_eq!(sent[1][6], 10);
}

#[tokio::test]
async fn fetch_explicit_slot_skips_basic_query() {
    let mut session = session_with(vec![detail_response(5)]).await;
    let preset = session.fetch_preset(Some(5)).await.unwrap();
    assert_eq!(preset.number, 5);
    assert_eq!(preset.active_scene_number, None);
    assert_eq!(session.transport.sent.len(), 1);
}

#[tokio::test]
async fn fetch_decodes_with_the_firmware_layout() {
    // GEN1 firmware: tempo at bytes 143–144, no sync flag.
    let mut gen1 = session_on_firmware("v402", vec![detail_response_gen1(3)]).await;
    let preset = gen1.fetch_preset(Some(3)).await.unwrap();
    assert_eq!(preset.tempo, 500);
    assert_eq!(preset.tempo_sync, None);

    // The same GEN1 bytes on a GEN2 session are read with the GEN2 layout.
    let mut gen2 = session_with(vec![detail_response_gen1(3)]).await;
    let preset = gen2.fetch_preset(Some(3)).await.unwrap();
    assert_eq!(preset.tempo_sync, Some(true)); // byte 144 = 52 ≠ 0
}

#[tokio::test]
async fn fetch_rejects_slot_out_of_range_without_sending() {
    let mut session = session_with(vec![]).await;
    assert!(matches!(
        session.fetch_preset(Some(0x80)).await,
        Err(SessionError::Rejected {
            operation: "preset detail query",
            source: NuxAudioError::InvalidPresetIndex(0x80)
        })
    ));
    assert!(session.transport.sent.is_empty());
}

#[tokio::test]
async fn fetch_reports_truncated_detail_as_parse_error() {
    let mut truncated = detail_response(1);
    truncated.truncate(50);
    let mut session = session_with(vec![truncated]).await;
    assert!(matches!(
        session.fetch_preset(Some(1)).await,
        Err(SessionError::Parse {
            response: "preset detail",
            ..
        })
    ));
}

#[tokio::test]
async fn current_preset_parse_and_timeout_errors() {
    let mut session = session_with(vec![BASIC_HEADER.to_vec()]).await;
    assert!(matches!(
        session.current_preset().await,
        Err(SessionError::Parse {
            response: "current preset",
            ..
        })
    ));
    // Queue now empty → the next request times out.
    assert!(matches!(
        session.current_preset_raw().await,
        Err(SessionError::Midi {
            operation: "current preset query",
            source: MidiError::Timeout(_)
        })
    ));
}
