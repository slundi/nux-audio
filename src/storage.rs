//! Local preset library and application configuration persistence.
//!
//! Two XDG-compliant files are managed:
//! - Config:         `~/.config/nux-audio/config.toml`
//! - Preset library: `~/.local/share/nux-audio/presets/<device-slug>.yaml`
//!   One file per device model (e.g. `mg-30.yaml`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::export::{EffectSlotFile, PRESET_FILE_VERSION};

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("XDG base directory error: {0}")]
    Xdg(String),

    #[error("cannot create directory '{path}': {detail}")]
    CreateDir { path: String, detail: String },

    #[error("cannot write '{path}': {detail}")]
    WriteFile { path: String, detail: String },

    #[error("cannot serialize config TOML: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("cannot serialize preset library YAML: {0}")]
    YamlSerialize(String),
}

// ── Config ────────────────────────────────────────────────────────────────────

/// Application configuration persisted to `~/.config/nux-audio/config.toml`.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Model code of the last used device (e.g. `"MG-30"`).
    pub last_device: Option<String>,
    /// Override the default XDG data directory for the preset library files.
    pub presets_dir: Option<PathBuf>,
}

// ── Preset library ────────────────────────────────────────────────────────────

/// One entry in a device's preset library YAML file.
///
/// `slot == None`   → preset exists only locally (not pulled from device).
/// `slot == Some(n)` → was pulled from device slot `n` (0-based).
/// When both a local and a device version of the same slot exist, the TUI
/// distinguishes them by the source icon (`📄` vs `📟`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LibraryEntry {
    /// Raw device slot index (0-based), or `None` for local-only presets.
    pub slot: Option<u8>,
    /// Firmware generation the preset was captured from, e.g. `"Gen2"`.
    pub firmware_gen: String,
    /// ISO-8601 timestamp of the last local modification.
    pub modified: DateTime<Utc>,
    pub name: String,
    /// Tempo in milliseconds per beat (`BPM = 60_000 / tempo_ms`).
    pub tempo_ms: u16,
    /// Parallel blend level (0–127).
    pub parallel: u8,
    /// Active scene index (0 = A, 1 = B, 2 = C).
    pub active_scene: u8,
    /// Signal-chain order by category name (e.g. `["Wah", "Comp", "Amp"]`).
    pub effect_order: Vec<String>,
    /// Per-scene bypass snapshots — 3 raw bytes each.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub scenes: Vec<[u8; 3]>,
    /// Active effect model for each populated slot.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub effects: Vec<EffectSlotFile>,
}

impl LibraryEntry {
    /// Build a local-only entry from a [`crate::export::PresetFile`].
    pub fn from_preset_file(preset: &crate::export::PresetFile, firmware_gen: &str) -> Self {
        Self {
            slot: None,
            firmware_gen: firmware_gen.to_owned(),
            modified: Utc::now(),
            name: preset.name.clone(),
            tempo_ms: preset.tempo_ms,
            parallel: preset.parallel,
            active_scene: preset.active_scene,
            effect_order: preset.effect_order.clone(),
            scenes: preset.scenes.clone(),
            effects: preset.effects.clone(),
        }
    }

    /// Returns `true` if this entry was pulled from a device slot.
    pub fn is_on_device(&self) -> bool {
        self.slot.is_some()
    }

    /// Returns `true` if this entry exists only locally (never synced to device).
    pub fn is_local_only(&self) -> bool {
        self.slot.is_none()
    }
}

/// Top-level structure for a device's preset library YAML file.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PresetLibrary {
    /// Format version — increment on breaking schema changes.
    pub version: u32,
    /// Device model code this library belongs to, e.g. `"MG-30"`.
    pub device: String,
    pub presets: Vec<LibraryEntry>,
}

impl PresetLibrary {
    pub fn new(device: &str) -> Self {
        Self {
            version: PRESET_FILE_VERSION,
            device: device.to_owned(),
            presets: Vec::new(),
        }
    }
}

// ── Storage ───────────────────────────────────────────────────────────────────

/// Manages XDG-compliant paths and I/O for [`Config`] and [`PresetLibrary`].
pub struct Storage {
    xdg: xdg::BaseDirectories,
    /// Optional override for the preset library directory (from `Config::presets_dir`).
    presets_dir: Option<PathBuf>,
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage {
    pub fn new() -> Self {
        Self {
            xdg: xdg::BaseDirectories::with_prefix("nux-audio"),
            presets_dir: None,
        }
    }

    /// Apply the relevant fields from a loaded [`Config`] to this storage instance.
    ///
    /// Call this after `load_config()` so that subsequent `load_library` /
    /// `save_library` calls honour the user's `presets_dir` override.
    pub fn apply_config(&mut self, config: &Config) {
        self.presets_dir = config.presets_dir.clone();
    }

    // ── Config ────────────────────────────────────────────────────────────────

    /// Load config from `~/.config/nux-audio/config.toml`.
    /// Returns [`Config::default`] if the file does not exist or cannot be parsed.
    pub fn load_config(&self) -> Config {
        let Some(path) = self.xdg.find_config_file("config.toml") else {
            return Config::default();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist `config` to `~/.config/nux-audio/config.toml`.
    pub fn save_config(&self, config: &Config) -> Result<(), StorageError> {
        let path = self
            .xdg
            .place_config_file("config.toml")
            .map_err(|e| StorageError::Xdg(e.to_string()))?;
        let content = toml::to_string(config)?;
        std::fs::write(&path, content).map_err(|e| StorageError::WriteFile {
            path: path.display().to_string(),
            detail: e.to_string(),
        })
    }

    // ── Preset library ────────────────────────────────────────────────────────

    /// Return the expected path of the library file for `device_model`.
    ///
    /// Default: `~/.local/share/nux-audio/presets/mg-30.yaml`.
    /// When `Config::presets_dir` is set, uses that directory instead:
    /// `<presets_dir>/mg-30.yaml` (no `presets/` sub-folder).
    /// The file may not exist yet.
    pub fn library_path(&self, device_model: &str) -> PathBuf {
        let slug = device_model.to_lowercase();
        match &self.presets_dir {
            Some(dir) => dir.join(format!("{slug}.yaml")),
            None => self
                .xdg
                .get_data_home()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(format!("presets/{slug}.yaml")),
        }
    }

    /// Load the preset library for `device_model`.
    /// Returns an empty [`PresetLibrary`] if the file does not exist or cannot be parsed.
    pub fn load_library(&self, device_model: &str) -> PresetLibrary {
        let path = self.library_path(device_model);
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_saphyr::from_str(&s).ok())
            .unwrap_or_else(|| PresetLibrary::new(device_model))
    }

    /// Persist `library` to its data path, creating parent directories as needed.
    ///
    /// Uses the `presets_dir` override when set (via [`apply_config`](Self::apply_config)),
    /// otherwise falls back to the XDG data home.
    pub fn save_library(&self, library: &PresetLibrary) -> Result<(), StorageError> {
        let path = self.library_path(&library.device);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StorageError::CreateDir {
                path: parent.display().to_string(),
                detail: e.to_string(),
            })?;
        }
        let content = serde_saphyr::to_string(library)
            .map_err(|e| StorageError::YamlSerialize(e.to_string()))?;
        std::fs::write(&path, content).map_err(|e| StorageError::WriteFile {
            path: path.display().to_string(),
            detail: e.to_string(),
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{EffectSlotFile, KnobFile};

    fn sample_entry() -> LibraryEntry {
        LibraryEntry {
            slot: Some(0),
            firmware_gen: "Gen2".to_owned(),
            modified: DateTime::from_timestamp(0, 0).unwrap(),
            name: "Crunch Lead".to_owned(),
            tempo_ms: 500,
            parallel: 64,
            active_scene: 0,
            effect_order: vec!["Amp".to_owned(), "Delay".to_owned()],
            scenes: vec![[0x01, 0x02, 0x03]],
            effects: vec![EffectSlotFile {
                category: "Amp".to_owned(),
                model_id: 3,
                model_name: "Modern".to_owned(),
                knobs: vec![KnobFile {
                    id: "gain".to_owned(),
                    title: "Gain".to_owned(),
                    ctrl: 22,
                    range_end: Some(127),
                }],
            }],
        }
    }

    #[test]
    fn test_library_entry_source_flags() {
        let mut entry = sample_entry();
        assert!(entry.is_on_device());
        assert!(!entry.is_local_only());

        entry.slot = None;
        assert!(!entry.is_on_device());
        assert!(entry.is_local_only());
    }

    #[test]
    fn test_library_yaml_roundtrip() {
        let library = PresetLibrary {
            version: PRESET_FILE_VERSION,
            device: "MG-30".to_owned(),
            presets: vec![sample_entry()],
        };

        let yaml = serde_saphyr::to_string(&library).unwrap();
        assert!(yaml.contains("MG-30"));
        assert!(yaml.contains("Crunch Lead"));
        assert!(yaml.contains("Gen2"));

        let restored: PresetLibrary = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(library, restored);
    }

    #[test]
    fn test_library_local_only_entry_no_slot() {
        let entry = LibraryEntry {
            slot: None,
            ..sample_entry()
        };
        let yaml = serde_saphyr::to_string(&entry).unwrap();
        // slot should serialise as null, not be missing
        assert!(yaml.contains("slot"));
        let restored: LibraryEntry = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(restored.slot, None);
    }

    #[test]
    fn test_preset_library_new_empty() {
        let lib = PresetLibrary::new("MG-30");
        assert_eq!(lib.device, "MG-30");
        assert_eq!(lib.version, PRESET_FILE_VERSION);
        assert!(lib.presets.is_empty());
    }

    #[test]
    fn test_config_yaml_roundtrip() {
        let cfg = Config {
            last_device: Some("MG-30".to_owned()),
            presets_dir: Some(PathBuf::from("/home/user/presets")),
        };
        let toml_str = toml::to_string(&cfg).unwrap();
        assert!(toml_str.contains("MG-30"));
        let restored: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg, restored);
    }

    #[test]
    fn test_storage_load_config_missing_returns_default() {
        // Point XDG at a temp dir that has no config file.
        let tmp = std::env::temp_dir().join("nux_test_no_config");
        let _ = std::fs::create_dir_all(&tmp);
        // We can't easily redirect XDG in a unit test without env var tricks,
        // so just verify the Storage::new() + load_config() path doesn't panic.
        let storage = Storage::new();
        let _cfg = storage.load_config(); // should not panic
    }

    #[test]
    fn test_library_path_slug() {
        let storage = Storage::new();
        let path = storage.library_path("MG-30");
        assert!(path.to_string_lossy().ends_with("mg-30.yaml"));

        let path2 = storage.library_path("Mighty Plug");
        assert!(path2.to_string_lossy().ends_with("mighty plug.yaml"));
    }

    #[test]
    fn test_library_path_override() {
        let mut storage = Storage::new();
        storage.apply_config(&Config {
            last_device: None,
            presets_dir: Some(PathBuf::from("/custom/presets")),
        });
        let path = storage.library_path("MG-30");
        assert_eq!(path, PathBuf::from("/custom/presets/mg-30.yaml"));
    }

    #[test]
    fn test_apply_config_none_presets_dir() {
        let mut storage = Storage::new();
        storage.apply_config(&Config {
            last_device: Some("MG-30".to_owned()),
            presets_dir: None,
        });
        // No override → XDG path, just verify slug still correct
        let path = storage.library_path("MG-30");
        assert!(path.to_string_lossy().ends_with("mg-30.yaml"));
    }

    #[test]
    fn test_from_preset_file() {
        use crate::export::PresetFile;
        let pf = PresetFile {
            version: PRESET_FILE_VERSION,
            device: "MG-30".to_owned(),
            slot: "01A".to_owned(),
            name: "Blues Clean".to_owned(),
            tempo_ms: 600,
            parallel: 0,
            active_scene: 1,
            effect_order: vec!["Amp".to_owned()],
            scenes: vec![[0x00, 0x00, 0x00]],
            effects: vec![],
        };
        let entry = LibraryEntry::from_preset_file(&pf, "Gen2");
        assert_eq!(entry.slot, None);
        assert_eq!(entry.firmware_gen, "Gen2");
        assert_eq!(entry.name, "Blues Clean");
        assert_eq!(entry.tempo_ms, 600);
        assert!(entry.is_local_only());
    }
}
