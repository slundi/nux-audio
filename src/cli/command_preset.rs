// Preset management: fetch from device and send preset files.

use crate::export::OutputFormat;
use crate::preset::Preset;
use clap::{Args, Subcommand};
use nux_audio::nux_sys_ex_commands::{CURRENT_PRESET_BASIC, mg30_request_preset};
use std::path::PathBuf;
use std::time::Duration;

const BASIC_HEADER: [u8; 6] = [0xF0, 0x43, 0x58, 0x70, 0x15, 0x02];
const DETAIL_HEADER: [u8; 6] = [0xF0, 0x43, 0x58, 0x70, 0x0B, 0x02];
const TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Subcommand)]
pub enum PresetAction {
    /// Display current preset details
    Show,

    /// Get preset from device
    Get(GetPresetArgs),

    /// Send preset to device
    Send(SendPresetArgs),
}

#[derive(Args)]
pub struct GetPresetArgs {
    /// Preset slot/location to retrieve from
    #[arg(short = 's', long = "slot")]
    pub slot: Option<u8>,

    /// Output file (optional, otherwise display to console)
    pub filename: Option<PathBuf>,
}

#[derive(Args)]
pub struct SendPresetArgs {
    /// Destination preset slot/location
    #[arg(short = 's', long = "slot")]
    pub slot: Option<u8>,

    /// Preset file to send
    pub filename: PathBuf,
}

pub async fn run(device_name: &Option<String>, action: PresetAction, _format: OutputFormat) {
    match action {
        PresetAction::Show => show(device_name).await,
        PresetAction::Get(args) => {
            println!(
                "Getting preset from slot: {:?}, saving to: {:?}",
                args.slot, args.filename
            );
        }
        PresetAction::Send(args) => {
            println!(
                "Sending preset from: {:?} to slot: {:?}",
                args.filename, args.slot
            );
        }
    }
}

async fn show(device_name: &Option<String>) {
    let mut session = match super::connect_and_detect(device_name).await {
        Some(s) => s,
        None => return,
    };

    println!("Firmware:  {} ({:?})", session.version_str, session.fw_gen);

    // Step 1: basic response → preset number + active scene
    if let Err(e) = session.manager.send_sysex(&CURRENT_PRESET_BASIC) {
        eprintln!("Failed to send preset basic request: {e}");
        return;
    }
    let basic_resp = match session
        .dispatcher
        .await_response(&BASIC_HEADER, TIMEOUT)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("No basic preset response: {e}");
            return;
        }
    };
    println!("Raw basic response ({} bytes):", basic_resp.len());
    for (i, chunk) in basic_resp.chunks(16).enumerate() {
        print!("  [{:03}] ", i * 16);
        for b in chunk {
            print!("{b:02X} ");
        }
        println!();
    }

    let basic = match Preset::extract_current_preset_basic_data(&basic_resp) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse basic preset data: {e}");
            return;
        }
    };

    let scene_letter = match basic.active_scene_number {
        Some(0) => 'A',
        Some(1) => 'B',
        Some(2) => 'C',
        Some(3) => 'D',
        _ => '?',
    };
    println!(
        "Preset:  {} (slot {})",
        basic.get_preset_code(),
        basic.number
    );
    println!("Scene:   S{scene_letter}");

    // Step 2: detail response → name, tempo, effect order, scenes
    if let Err(e) = session
        .manager
        .send_sysex(&mg30_request_preset(basic.number))
    {
        eprintln!("Failed to send preset detail request: {e}");
        return;
    }
    let detail_resp = match session
        .dispatcher
        .await_response(&DETAIL_HEADER, TIMEOUT)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("No detail preset response: {e}");
            return;
        }
    };

    // GEN2: BPM at bytes 144 (sync state) and 147 (value)
    // Formula: BPM = (byte_147 + 128) / 2
    // Confirmed at BPM 98 (0x44) and BPM 120 (0x70)
    if detail_resp.len() > 147 {
        let sync_on = detail_resp[144] != 0x00;
        let bpm = (detail_resp[147] as u16 + 128) / 2;
        if sync_on {
            println!("BPM:     {bpm} (sync on)");
        } else {
            println!("BPM:     {bpm} (sync off)");
        }
    }

    println!("Raw detail response ({} bytes):", detail_resp.len());
    for (i, chunk) in detail_resp.chunks(16).enumerate() {
        print!("  [{:03}] ", i * 16);
        for b in chunk {
            print!("{b:02X} ");
        }
        println!();
    }

    match Preset::extract_current_preset_detail_data(&detail_resp) {
        Ok(preset) => {
            println!("Name:    {}", preset.name);
            let bpm = 60_000u32.checked_div(preset.tempo as u32).unwrap_or(0);
            println!("Tempo:   {bpm} BPM (raw: {})", preset.tempo);
            println!("Parallel:{}", preset.parallel);
            let order: Vec<&str> = preset.effect_order.iter().map(|e| e.as_str()).collect();
            println!("Chain:   {}", order.join(" → "));
            for (i, scene) in preset.scenes.iter().enumerate() {
                println!(
                    "Scene {}:  {:02X} {:02X} {:02X}",
                    i + 1,
                    scene.0[0],
                    scene.0[1],
                    scene.0[2]
                );
            }
        }
        Err(e) => eprintln!("Failed to parse detail data: {e}"),
    }
}
