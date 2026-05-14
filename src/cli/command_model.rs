// Effect model switch via SysEx.
//
// Accepts a model by hex ID (0x45), decimal ID (69), or case-insensitive name.
// Wire format is provisional — not yet verified against hardware captures.

use crate::effects::{EffectCategory, find_model_by_name};
use crate::midi::MidiManager;
use nux_audio::nux_sys_ex_commands::mg30_change_effect_model;

/// Switch the active model within an effect slot.
///
/// `model_str` accepts:
/// - `"0x45"` or `"0X45"` — parsed as a hex model ID
/// - `"69"` — parsed as a decimal model ID
/// - `"Hall"` — matched case-insensitively against the catalog `title` field
pub async fn run(device_name: &Option<String>, category_str: &str, model_str: &str) {
    let category = match category_str.parse::<EffectCategory>() {
        Ok(c) => c,
        Err(()) => {
            eprintln!(
                "Unknown effect category '{category_str}'. \
                 Try: wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir"
            );
            return;
        }
    };

    // Resolve model: hex (0x…), decimal, or name search
    let (model_id, model_title) = if let Some(hex) = model_str
        .strip_prefix("0x")
        .or_else(|| model_str.strip_prefix("0X"))
    {
        match u8::from_str_radix(hex, 16) {
            Ok(id) => (id, None),
            Err(_) => {
                eprintln!("Invalid hex model ID '{model_str}'");
                return;
            }
        }
    } else if let Ok(id) = model_str.parse::<u8>() {
        (id, None)
    } else {
        match find_model_by_name(category, model_str) {
            Some((id, opt)) => (id, Some(opt.title)),
            None => {
                eprintln!(
                    "Model '{model_str}' not found in {} catalog",
                    category.as_str()
                );
                return;
            }
        }
    };

    let (device, _port_in, port_out) = match super::resolve_ports(device_name) {
        Some(t) => t,
        None => return,
    };

    let mut manager = match MidiManager::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Cannot create MIDI manager: {e}");
            return;
        }
    };

    if let Err(e) = manager.connect_output(port_out) {
        eprintln!("Cannot connect to {}: {e}", device.model);
        return;
    }

    let payload = mg30_change_effect_model(category as u8, model_id);
    match manager.send_sysex(&payload) {
        Ok(()) => {
            let name_part = model_title.map(|t| format!(" ({t})")).unwrap_or_default();
            println!(
                "{} model → 0x{model_id:02X}{name_part} [provisional SysEx] ({})",
                category.as_str(),
                device.model
            );
        }
        Err(e) => eprintln!("Failed to send model change SysEx: {e}"),
    }
}
