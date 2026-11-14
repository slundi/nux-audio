// Preset management: fetch from device and send preset files.
//
// Device I/O goes through `NuxSession`; this file handles paths and output.

use super::error::CliError;
use crate::export::{OutputFormat, PresetFile};
use crate::session::NuxSession;
use nux_audio::preset::Preset;
use std::path::PathBuf;

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

pub async fn run(
    device_name: Option<&str>,
    action: PresetAction,
    format: OutputFormat,
) -> Result<(), CliError> {
    match action {
        PresetAction::Show => show(device_name, format).await,
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

async fn get(
    device_name: Option<&str>,
    args: GetPresetArgs,
    format: OutputFormat,
) -> Result<(), CliError> {
    let mut session = NuxSession::connect(device_name).await?;

    // Explicit `--slot` wins; otherwise the session reads whichever preset is
    // currently loaded (which also gives us the active scene).
    let preset = session.fetch_preset(args.slot).await?;

    let file = PresetFile::from_preset(&preset, &session.device().model);
    let path = default_output_path(args.filename, &preset, format);
    file.write_to_file(&path, format)?;

    println!(
        "Saved preset {} (\"{}\") to {}",
        preset.get_preset_code(),
        preset.name,
        path.display()
    );
    Ok(())
}

/// Print `bytes` as a 16-per-line hex dump with decimal offsets.
fn print_hex_dump(label: &str, bytes: &[u8]) {
    println!("Raw {label} response ({} bytes):", bytes.len());
    for (i, chunk) in bytes.chunks(16).enumerate() {
        print!("  [{:03}] ", i * 16);
        for b in chunk {
            print!("{b:02X} ");
        }
        println!();
    }
}

async fn show(device_name: Option<&str>, format: OutputFormat) -> Result<(), CliError> {
    let mut session = NuxSession::connect(device_name).await?;

    let driver = session.driver();
    match driver.firmware_gen() {
        Some(fw_gen) => println!("Firmware:  {} ({fw_gen})", driver.firmware()),
        None => println!("Firmware:  {}", driver.firmware()),
    }

    // Step 1: basic response → preset number + active scene
    let basic_resp = session.current_preset_raw().await?;
    print_hex_dump("basic", &basic_resp);
    let basic = session.driver().decode_current_preset(&basic_resp)?;

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

    // Step 2: detail response → name, tempo, effect order, scenes (layout per firmware)
    let detail_resp = session.preset_detail_raw(basic.number).await?;
    print_hex_dump("detail", &detail_resp);

    let mut preset = session.driver().decode_preset_detail(&detail_resp)?;
    preset.active_scene_number = basic.active_scene_number;
    println!("Name:    {}", preset.name);
    let bpm = 60_000u32.checked_div(preset.tempo as u32).unwrap_or(0);
    let sync = match preset.tempo_sync {
        Some(true) => ", sync on",
        Some(false) => ", sync off",
        None => "",
    };
    println!("Tempo:   {bpm} BPM (raw: {} ms/beat{sync})", preset.tempo);
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
    // Honor --json/--yaml: emit a machine-readable dump of the parsed preset.
    let label = match format {
        OutputFormat::Yaml => "yaml",
        OutputFormat::Json => "json",
    };
    println!("--- serialized ({label}) ---");
    let file = PresetFile::from_preset(&preset, &session.device().model);
    println!("{}", file.to_string_fmt(format)?);
    Ok(())
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
