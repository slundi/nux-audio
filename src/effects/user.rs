//! Runtime user-defined effect model extensions loaded from XDG config.
//!
//! Place YAML files under `~/.config/nux-audio/effects/` to add custom models
//! without recompiling.  See `docs/effects_extension_schema.yaml` for the format.

use std::{collections::HashMap, sync::OnceLock};

use crate::effects::{EffectCategory, EffectOption, FirmwareGen};

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

    let Ok(entries) = std::fs::read_dir(&effects_dir) else {
        return result;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(ext) = serde_saphyr::from_str::<ExtensionFile>(&content) else {
            continue;
        };

        let Ok(category) = ext.category.parse::<EffectCategory>() else {
            continue;
        };
        let slot = result.entry(category).or_default();
        for model in ext.models {
            let leaked: &'static EffectOption = Box::leak(Box::new(model));
            slot.insert(leaked.id, leaked);
        }
    }

    result
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
