// Preset management: fetch from device and send preset files.

use crate::export::{OutputFormat, PresetFile};
use crate::preset::Preset;
use nux_audio::nux_sys_ex_commands::{CURRENT_PRESET_BASIC, mg30_request_preset};
use std::path::PathBuf;
use std::time::Duration;

const BASIC_HEADER: [u8; 6] = [0xF0, 0x43, 0x58, 0x70, 0x15, 0x02];
const DETAIL_HEADER: [u8; 6] = [0xF0, 0x43, 0x58, 0x70, 0x0B, 0x02];
const TIMEOUT: Duration = Duration::from_secs(3);

/// Exit code used when `preset send` is invoked. The write-preset SysEx opcode
/// has not been reverse-engineered yet, so the command cannot complete.
const EXIT_NOT_IMPLEMENTED: i32 = 2;

#[derive(Clone)]
pub enum PresetAction {
    Show,
    Get(GetPresetArgs),
    Send(SendPresetArgs),
}

#[derive(Clone)]
pub struct GetPresetArgs {
    pub slot: Option<u8>,
    pub filename: Option<PathBuf>,
}

#[derive(Clone)]
pub struct SendPresetArgs {
    pub slot: Option<u8>,
    pub filename: PathBuf,
}

pub async fn run(device_name: &Option<String>, action: PresetAction, format: OutputFormat) {
    match action {
        PresetAction::Show => show(device_name).await,
        PresetAction::Get(args) => get(device_name, args, format).await,
        PresetAction::Send(_) => {
            eprintln!(
                "error: `preset send` is not yet implemented — the write-preset SysEx opcode \
                 has not been reverse-engineered. See docs/Roadmap.md (Phase 3)."
            );
            std::process::exit(EXIT_NOT_IMPLEMENTED);
        }
    }
}

/// Filesystem-safe basename derived from the slot code and preset name.
///
/// Example: slot 5 with name "My Tone" → `"02B-my-tone"`.
/// Empty or all-non-alphanumeric names collapse to just the slot code.
fn sanitize_basename(slot_code: &str, name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    // Collapse runs of dashes so "foo   bar" doesn't become "foo---bar".
    let mut collapsed = String::with_capacity(trimmed.len());
    let mut prev_dash = false;
    for c in trimmed.chars() {
        let is_dash = c == '-';
        if !(is_dash && prev_dash) {
            collapsed.push(c);
        }
        prev_dash = is_dash;
    }
    if collapsed.is_empty() {
        slot_code.to_owned()
    } else {
        format!("{slot_code}-{collapsed}")
    }
}

/// Choose a destination path for `preset get`. `explicit` (from `--filename`) wins;
/// otherwise derive `{slot_code}-{name}.{yaml|json}` from the fetched preset.
fn default_output_path(
    explicit: Option<PathBuf>,
    preset: &Preset,
    format: OutputFormat,
) -> PathBuf {
    if let Some(path) = explicit {
        return path;
    }
    let ext = match format {
        OutputFormat::Yaml => "yaml",
        OutputFormat::Json => "json",
    };
    let basename = sanitize_basename(&preset.get_preset_code(), &preset.name);
    PathBuf::from(format!("{basename}.{ext}"))
}

async fn get(device_name: &Option<String>, args: GetPresetArgs, format: OutputFormat) {
    let mut session = match super::connect_and_detect(device_name).await {
        Some(s) => s,
        None => return,
    };

    // Resolve target slot: explicit `--slot` wins; otherwise query the device for
    // whichever preset is currently loaded (also gives us the active scene).
    let (target_slot, active_scene) = match args.slot {
        Some(slot) => {
            if slot > 0x7F {
                eprintln!("error: slot {slot} out of range (0–127)");
                return;
            }
            (slot, None)
        }
        None => match query_current_preset_basic(&mut session).await {
            Some((slot, scene)) => (slot, Some(scene)),
            None => return,
        },
    };

    if let Err(e) = session
        .manager
        .send_sysex(&mg30_request_preset(target_slot))
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

    let mut preset = match Preset::extract_current_preset_detail_data(&detail_resp) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse detail data: {e}");
            return;
        }
    };
    preset.active_scene_number = active_scene;

    let file = PresetFile::from_preset(&preset, &session.device.model);
    let path = default_output_path(args.filename, &preset, format);

    if let Err(e) = file.write_to_file(&path, format) {
        eprintln!("Failed to write preset file: {e}");
        return;
    }

    println!(
        "Saved preset {} (\"{}\") to {}",
        preset.get_preset_code(),
        preset.name,
        path.display()
    );
}

/// Query the device for the currently active preset (slot number + active scene).
///
/// Returns `None` on any failure (error already printed to stderr).
async fn query_current_preset_basic(session: &mut super::DeviceSession) -> Option<(u8, u8)> {
    if let Err(e) = session.manager.send_sysex(&CURRENT_PRESET_BASIC) {
        eprintln!("Failed to send preset basic request: {e}");
        return None;
    }
    let basic_resp = match session
        .dispatcher
        .await_response(&BASIC_HEADER, TIMEOUT)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("No basic preset response: {e}");
            return None;
        }
    };
    let basic = match Preset::extract_current_preset_basic_data(&basic_resp) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to parse basic preset data: {e}");
            return None;
        }
    };
    Some((basic.number, basic.active_scene_number.unwrap_or(0)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_basename_plain_name() {
        assert_eq!(sanitize_basename("02B", "My Tone"), "02B-my-tone");
    }

    #[test]
    fn sanitize_basename_punctuation_collapsed() {
        assert_eq!(
            sanitize_basename("01A", "Metal!!! Rig #3"),
            "01A-metal-rig-3"
        );
    }

    #[test]
    fn sanitize_basename_leading_trailing_punct_trimmed() {
        assert_eq!(sanitize_basename("10C", "  ...clean..."), "10C-clean");
    }

    #[test]
    fn sanitize_basename_all_symbols_falls_back_to_slot() {
        // No alphanumeric survives sanitization → basename is just the slot code.
        assert_eq!(sanitize_basename("32D", "!!!"), "32D");
    }

    #[test]
    fn sanitize_basename_empty_name() {
        assert_eq!(sanitize_basename("01A", ""), "01A");
    }

    #[test]
    fn sanitize_basename_unicode_dropped() {
        // Non-ASCII chars become '-', then get collapsed/trimmed.
        assert_eq!(sanitize_basename("05A", "Café"), "05A-caf");
    }

    fn make_preset(number: u8, name: &str) -> Preset {
        Preset {
            number,
            name: name.to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn default_output_path_explicit_wins() {
        let preset = make_preset(0, "ignored");
        let explicit = PathBuf::from("/tmp/custom.yaml");
        let path = default_output_path(Some(explicit.clone()), &preset, OutputFormat::Yaml);
        assert_eq!(path, explicit);
    }

    #[test]
    fn default_output_path_yaml_extension() {
        let preset = make_preset(0, "Clean");
        let path = default_output_path(None, &preset, OutputFormat::Yaml);
        assert_eq!(path, PathBuf::from("01A-clean.yaml"));
    }

    #[test]
    fn default_output_path_json_extension() {
        let preset = make_preset(4, "Lead");
        let path = default_output_path(None, &preset, OutputFormat::Json);
        assert_eq!(path, PathBuf::from("02A-lead.json"));
    }

    #[test]
    fn default_output_path_no_name_uses_slot_only() {
        let preset = make_preset(127, "");
        let path = default_output_path(None, &preset, OutputFormat::Yaml);
        assert_eq!(path, PathBuf::from("32D.yaml"));
    }
}
