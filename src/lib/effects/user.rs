//! Runtime user-defined effect model extensions loaded from XDG config.
//!
//! Place YAML files under `~/.config/nux-audio/effects/` to add custom models
//! without recompiling.  See `docs/effects_extension_schema.yaml` for the format.
//!
//! Loader diagnostics: any I/O, parse, or validation problem emits an
//! `eprintln!` warning naming the offending file so users can spot bad YAML
//! without hunting through silent failures.

use std::{collections::HashMap, path::Path, sync::OnceLock};

use crate::effects::{EffectCategory, EffectOption, FirmwareGen};

/// MIDI Control-Change and value limit — MIDI is 7-bit, so `ctrl` and knob
/// endpoints must be in `0..=127`.
const MIDI_MAX: u8 = 127;

/// All user-defined models, keyed by category then by model ID (as `'static` refs via Box::leak).
/// Loaded once on first access; empty if no YAML files exist.
pub static USER_MODELS: OnceLock<HashMap<EffectCategory, HashMap<u8, &'static EffectOption>>> =
    OnceLock::new();

/// Load user extension YAMLs from `~/.config/nux-audio/effects/*.yaml`.
/// Called exactly once, via `USER_MODELS.get_or_init(load)`.
pub fn load() -> HashMap<EffectCategory, HashMap<u8, &'static EffectOption>> {
    let Some(config_dir) = dirs_next_config_dir() else {
        return HashMap::new();
    };
    let effects_dir = config_dir.join("nux-audio").join("effects");
    if !effects_dir.exists() {
        return HashMap::new();
    }

    let mut result: HashMap<EffectCategory, HashMap<u8, &'static EffectOption>> = HashMap::new();

    let entries = match std::fs::read_dir(&effects_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "warning: cannot read user-effects directory {}: {e}",
                effects_dir.display()
            );
            return result;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        load_one_file(&path, &mut result);
    }

    result
}

fn load_one_file(
    path: &Path,
    result: &mut HashMap<EffectCategory, HashMap<u8, &'static EffectOption>>,
) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("warning: cannot read {}: {e}", path.display());
            return;
        }
    };
    let ext: ExtensionFile = match serde_saphyr::from_str(&content) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("warning: cannot parse {}: {e}", path.display());
            return;
        }
    };

    let category = match ext.category.parse::<EffectCategory>() {
        Ok(c) => c,
        Err(()) => {
            eprintln!(
                "warning: unknown category '{}' in {}",
                ext.category,
                path.display()
            );
            return;
        }
    };

    let slot = result.entry(category).or_default();
    for model in ext.models {
        if let Err(msg) = validate_model(&model) {
            eprintln!(
                "warning: skipping model 0x{:02X} '{}' in {}: {msg}",
                model.id,
                model.title,
                path.display()
            );
            continue;
        }
        let leaked: &'static EffectOption = Box::leak(Box::new(model));
        slot.insert(leaked.id, leaked);
    }
}

/// Reject models that would produce out-of-spec MIDI messages when used.
///
/// MIDI is a 7-bit protocol: CC numbers, values, and SysEx bytes are all
/// bounded to `0..=127`. Since these fields are `u8`, deserialization would
/// otherwise happily accept 128..=255 and the failure would only surface
/// later inside `MidiManager::send_cc`.
fn validate_model(model: &EffectOption) -> Result<(), String> {
    if model.id > MIDI_MAX {
        return Err(format!("model id 0x{:02X} > 0x7F", model.id));
    }
    for knob in &model.knobs {
        if knob.ctrl > MIDI_MAX {
            return Err(format!("knob '{}' has ctrl {} > 127", knob.id, knob.ctrl));
        }
        if let Some(end) = knob.range_end
            && end > MIDI_MAX
        {
            return Err(format!("knob '{}' has range_end {end} > 127", knob.id));
        }
        if let (Some(start), Some(end)) = (knob.range_start, knob.range_end)
            && start > end
        {
            return Err(format!(
                "knob '{}' has range_start {start} > range_end {end}",
                knob.id
            ));
        }
    }
    Ok(())
}

fn dirs_next_config_dir() -> Option<std::path::PathBuf> {
    // Use the XDG config home or fallback
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
}

#[derive(serde::Deserialize)]
struct ExtensionFile {
    /// Effect slot name, e.g. `"amp"` or `"reverb"`.
    category: String,
    models: Vec<EffectOption>,
}

/// Look up a user-defined model.  Returns `None` if no user extension covers this slot/ID.
pub fn get(category: EffectCategory, model_id: u8) -> Option<&'static EffectOption> {
    USER_MODELS
        .get_or_init(load)
        .get(&category)?
        .get(&model_id)
        .copied()
}

/// Find a user-defined model by name (case-insensitive).
pub fn find_by_name(
    category: EffectCategory,
    name: &str,
    fw_gen: FirmwareGen,
) -> Option<(u8, &'static EffectOption)> {
    USER_MODELS
        .get_or_init(load)
        .get(&category)?
        .iter()
        .filter(|(_, opt)| opt.min_gen <= fw_gen)
        .find(|(_, opt)| opt.title.eq_ignore_ascii_case(name))
        .map(|(id, opt)| (*id, *opt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::KnobDef;

    fn model_with_ctrl(ctrl: u8) -> EffectOption {
        EffectOption {
            id: 0x00,
            title: "Test".into(),
            knobs: vec![KnobDef {
                id: "k".into(),
                title: "K".into(),
                range_start: Some(0),
                range_end: Some(100),
                ctrl,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn validate_accepts_well_formed_model() {
        assert!(validate_model(&model_with_ctrl(22)).is_ok());
    }

    #[test]
    fn validate_rejects_model_id_over_127() {
        let mut m = model_with_ctrl(22);
        m.id = 128;
        let err = validate_model(&m).unwrap_err();
        assert!(err.contains("model id"), "unexpected error: {err}");
    }

    #[test]
    fn validate_rejects_cc_over_127() {
        let err = validate_model(&model_with_ctrl(200)).unwrap_err();
        assert!(err.contains("ctrl 200"), "unexpected error: {err}");
    }

    #[test]
    fn validate_rejects_cc_at_boundary_128() {
        // Boundary: 127 is the MIDI max, 128 is the first invalid value.
        assert!(validate_model(&model_with_ctrl(127)).is_ok());
        assert!(validate_model(&model_with_ctrl(128)).is_err());
    }

    #[test]
    fn validate_rejects_range_end_over_127() {
        let mut m = model_with_ctrl(22);
        m.knobs[0].range_end = Some(200);
        let err = validate_model(&m).unwrap_err();
        assert!(err.contains("range_end 200"), "unexpected error: {err}");
    }

    #[test]
    fn validate_rejects_inverted_range() {
        let mut m = model_with_ctrl(22);
        m.knobs[0].range_start = Some(80);
        m.knobs[0].range_end = Some(50);
        let err = validate_model(&m).unwrap_err();
        assert!(
            err.contains("range_start 80 > range_end 50"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validate_reports_second_knob_when_first_ok() {
        let mut m = model_with_ctrl(22);
        m.knobs.push(KnobDef {
            id: "bad".into(),
            title: "Bad".into(),
            range_start: Some(0),
            range_end: Some(100),
            ctrl: 200,
        });
        let err = validate_model(&m).unwrap_err();
        assert!(err.contains("'bad'"), "unexpected error: {err}");
    }
}
