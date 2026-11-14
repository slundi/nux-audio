//! Versioned preset file format (YAML/JSON): export types, schema migration and file I/O.

use serde::{Deserialize, Serialize};

use crate::error::ExportError;
use crate::preset::Preset;

// ── Schema migration ──────────────────────────────────────────────────────────

/// Minimum recognized schema version (files older than this are rejected).
const MIN_SUPPORTED_VERSION: u32 = 1;

/// Deserialize a preset file string, applying any necessary schema migrations.
///
/// This is the preferred entry-point for loading preset files rather than calling
/// [`PresetFile::from_str_fmt`] directly, because it handles version upgrades
/// transparently:
///
/// - Version 1 (current): no transformation needed — returned as-is.
/// - Versions below [`MIN_SUPPORTED_VERSION`]: rejected with a descriptive error.
/// - Future versions above [`PRESET_FILE_VERSION`]: rejected with a descriptive
///   error so older tool builds fail loudly rather than silently dropping fields.
///
/// # Adding a migration
///
/// When bumping `PRESET_FILE_VERSION` to N, add a match arm here that converts
/// version N-1 data to N:
///
/// ```ignore
/// 1 => {
///     // parse as V1 (may be missing new_field)
///     let v1: PresetFileV1 = parse_versioned(raw, format)?;
///     PresetFile { new_field: default_value(), ..v1.into() }
/// }
/// ```
pub fn migrate_preset(raw: &str, format: OutputFormat) -> Result<PresetFile, ExportError> {
    // Quick-parse only the `version` field so we can branch before full deserialization.
    #[derive(Deserialize)]
    struct VersionProbe {
        version: u32,
    }

    let probe: VersionProbe = match format {
        OutputFormat::Yaml => {
            serde_saphyr::from_str(raw).map_err(|e| ExportError::Deserialize(e.to_string()))?
        }
        OutputFormat::Json => {
            serde_json::from_str(raw).map_err(|e| ExportError::Deserialize(e.to_string()))?
        }
    };

    if probe.version < MIN_SUPPORTED_VERSION {
        return Err(ExportError::VersionTooOld {
            found: probe.version,
            minimum: MIN_SUPPORTED_VERSION,
        });
    }
    if probe.version > PRESET_FILE_VERSION {
        return Err(ExportError::VersionTooNew {
            found: probe.version,
            supported: PRESET_FILE_VERSION,
        });
    }

    // Version 1 is current — no transformation needed.
    PresetFile::from_str_fmt(raw, format)
}

/// Schema version stamped into every exported preset file.
/// Increment when the format changes in a backwards-incompatible way.
pub const PRESET_FILE_VERSION: u32 = 1;

/// Output format selection (CLI flags `--yaml` / `--json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Yaml,
    Json,
}

// ── Owned, serde-compatible export types ─────────────────────────────────────
//
// The runtime types (`Preset`, `EffectOption`, `KnobDef`) borrow `&'static str`
// slices from the compiled-in effect catalogs.  Those lifetimes cannot be
// expressed in a Deserialize impl, so we use separate owned types for the file
// format and convert with `PresetFile::from_preset`.

/// Serializable knob definition stored in a preset file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnobFile {
    pub id: String,
    pub title: String,
    /// MIDI CC number that controls this knob in real time.
    pub ctrl: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_end: Option<u8>,
}

/// Serializable effect slot stored in a preset file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectSlotFile {
    /// Category name, e.g. `"Amp"`.
    pub category: String,
    /// Model ID as transmitted in SysEx messages.
    pub model_id: u8,
    /// Human-readable model name, e.g. `"Modern"`.
    pub model_name: String,
    pub knobs: Vec<KnobFile>,
}

/// Versioned, device-tagged preset file — the canonical export/import format.
///
/// Serializes to YAML (default) or JSON depending on the `--yaml` / `--json`
/// CLI flag.  The `version` field guards against format drift when the schema
/// changes in future releases.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PresetFile {
    /// Format schema version. Currently `1`.
    pub version: u32,
    /// Device model code this preset was captured from, e.g. `"MG-30"`.
    pub device: String,
    /// Formatted slot identifier, e.g. `"01A"`.
    pub slot: String,
    pub name: String,
    /// Tempo in milliseconds per beat (`BPM = 60_000 / tempo_ms`).
    pub tempo_ms: u16,
    /// Parallel blend level (0–127).
    pub parallel: u8,
    /// Active scene index (0 = A, 1 = B, 2 = C).
    pub active_scene: u8,
    /// Signal-chain order by category name (e.g. `["Wah", "Comp", "Amp", …]`).
    pub effect_order: Vec<String>,
    /// Per-scene bypass snapshots — 3 raw bytes each.
    /// Bit layout is provisional pending hardware verification.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub scenes: Vec<[u8; 3]>,
    /// Active effect model for each populated slot.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub effects: Vec<EffectSlotFile>,
}

impl PresetFile {
    /// Convert a parsed [`Preset`] into an exportable [`PresetFile`].
    ///
    /// `device` is the model code (e.g. `"MG-30"`) from the device registry.
    pub fn from_preset(preset: &Preset, device: &str) -> Self {
        let slot = Preset::format_code(preset.number);

        let effect_order = preset
            .effect_order
            .iter()
            .map(|c| c.as_str().to_owned())
            .collect();

        let scenes = preset.scenes.iter().map(|s| s.0).collect();

        let effects = preset
            .effects
            .iter()
            .map(|(cat, opt)| EffectSlotFile {
                category: cat.as_str().to_owned(),
                model_id: opt.id,
                model_name: opt.title.clone(),
                knobs: opt
                    .knobs
                    .iter()
                    .map(|k| KnobFile {
                        id: k.id.clone(),
                        title: k.title.clone(),
                        ctrl: k.ctrl,
                        range_end: k.range_end,
                    })
                    .collect(),
            })
            .collect();

        Self {
            version: PRESET_FILE_VERSION,
            device: device.to_owned(),
            slot,
            name: preset.name.clone(),
            tempo_ms: preset.tempo,
            parallel: preset.parallel,
            active_scene: preset.active_scene_number.unwrap_or(0),
            effect_order,
            scenes,
            effects,
        }
    }

    /// Serialize this preset to a UTF-8 string in the requested format.
    pub fn to_string_fmt(&self, format: OutputFormat) -> Result<String, ExportError> {
        match format {
            OutputFormat::Yaml => {
                serde_saphyr::to_string(self).map_err(|e| ExportError::Serialize(e.to_string()))
            }
            OutputFormat::Json => serde_json::to_string_pretty(self)
                .map_err(|e| ExportError::Serialize(e.to_string())),
        }
    }

    /// Deserialize a preset from a UTF-8 string in the given format.
    pub fn from_str_fmt(content: &str, format: OutputFormat) -> Result<Self, ExportError> {
        match format {
            OutputFormat::Yaml => {
                serde_saphyr::from_str(content).map_err(|e| ExportError::Deserialize(e.to_string()))
            }
            OutputFormat::Json => {
                serde_json::from_str(content).map_err(|e| ExportError::Deserialize(e.to_string()))
            }
        }
    }

    /// Write to a file path, creating or truncating it.
    pub fn write_to_file(
        &self,
        path: &std::path::Path,
        format: OutputFormat,
    ) -> Result<(), ExportError> {
        let content = self.to_string_fmt(format)?;
        std::fs::write(path, content).map_err(|source| ExportError::WriteFile {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Read and deserialize from a file path, inferring format from extension.
    ///
    /// `.json` files are parsed as JSON; everything else is treated as YAML.
    pub fn read_from_file(path: &std::path::Path) -> Result<Self, ExportError> {
        let content = std::fs::read_to_string(path).map_err(|source| ExportError::ReadFile {
            path: path.to_path_buf(),
            source,
        })?;
        let format = match path.extension().and_then(|e| e.to_str()) {
            Some("json") => OutputFormat::Json,
            _ => OutputFormat::Yaml,
        };
        Self::from_str_fmt(&content, format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{EffectCategory, EffectOption, KnobDef};
    use crate::preset::{Preset, SceneSnapshot};
    use std::collections::HashMap;

    fn sample_preset() -> Preset {
        let mut effects = HashMap::new();
        effects.insert(
            EffectCategory::Amp,
            EffectOption {
                id: 3,
                title: "Modern".into(),
                on_byte: 0x01,
                off_byte: 0x00,
                category_color: 0xFF5500,
                dominant_color: None,
                knobs: vec![KnobDef {
                    id: "gain".into(),
                    title: "Gain".into(),
                    ctrl: 22,
                    range_start: Some(0),
                    range_end: Some(127),
                }],
                ..Default::default()
            },
        );
        Preset {
            number: 5,
            name: "My Tone".to_owned(),
            tempo: 500,
            parallel: 64,
            active_scene_number: Some(1),
            scene: 1,
            effect_order: vec![EffectCategory::Amp, EffectCategory::Reverb],
            scenes: vec![
                SceneSnapshot([0x01, 0x02, 0x03]),
                SceneSnapshot([0x04, 0x05, 0x06]),
                SceneSnapshot([0x07, 0x08, 0x09]),
            ],
            effects,
        }
    }

    #[test]
    fn test_from_preset_fields() {
        let preset = sample_preset();
        let file = PresetFile::from_preset(&preset, "MG-30");

        assert_eq!(file.version, PRESET_FILE_VERSION);
        assert_eq!(file.device, "MG-30");
        assert_eq!(file.slot, "02B");
        assert_eq!(file.name, "My Tone");
        assert_eq!(file.tempo_ms, 500);
        assert_eq!(file.parallel, 64);
        assert_eq!(file.active_scene, 1);
        assert_eq!(file.effect_order, vec!["Amp", "Reverb"]);
        assert_eq!(file.scenes.len(), 3);
        assert_eq!(file.scenes[0], [0x01, 0x02, 0x03]);
        assert_eq!(file.effects.len(), 1);
        assert_eq!(file.effects[0].category, "Amp");
        assert_eq!(file.effects[0].model_id, 3);
        assert_eq!(file.effects[0].model_name, "Modern");
        assert_eq!(file.effects[0].knobs[0].ctrl, 22);
    }

    #[test]
    fn test_yaml_roundtrip() {
        let preset = sample_preset();
        let file = PresetFile::from_preset(&preset, "MG-30");

        let yaml = file.to_string_fmt(OutputFormat::Yaml).unwrap();
        assert!(yaml.contains("version: 1"));
        assert!(yaml.contains("MG-30"));
        assert!(yaml.contains("My Tone"));

        let restored = PresetFile::from_str_fmt(&yaml, OutputFormat::Yaml).unwrap();
        assert_eq!(file, restored);
    }

    #[test]
    fn test_json_roundtrip() {
        let preset = sample_preset();
        let file = PresetFile::from_preset(&preset, "MG-30");

        let serialized = file.to_string_fmt(OutputFormat::Json).unwrap();
        assert!(serialized.contains("\"version\": 1"));
        assert!(serialized.contains("MG-30"));

        let restored = PresetFile::from_str_fmt(&serialized, OutputFormat::Json).unwrap();
        assert_eq!(file, restored);
    }

    #[test]
    fn test_format_mismatch_error() {
        let preset = sample_preset();
        let _file = PresetFile::from_preset(&preset, "MG-30");

        // Valid JSON is also valid YAML (superset), but an invalid JSON string
        // should fail JSON parsing.
        assert!(PresetFile::from_str_fmt("not: valid: json: {{", OutputFormat::Json).is_err());
    }

    #[test]
    fn test_infer_format_from_extension() {
        // Write and read back via temp file with .json extension
        let preset = sample_preset();
        let file = PresetFile::from_preset(&preset, "MG-30");

        let tmp = std::env::temp_dir().join("nux_test_preset.json");
        file.write_to_file(&tmp, OutputFormat::Json).unwrap();
        let restored = PresetFile::read_from_file(&tmp).unwrap();
        assert_eq!(file, restored);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_infer_yaml_from_extension() {
        let preset = sample_preset();
        let file = PresetFile::from_preset(&preset, "MG-30");

        let tmp = std::env::temp_dir().join("nux_test_preset.yaml");
        file.write_to_file(&tmp, OutputFormat::Yaml).unwrap();
        let restored = PresetFile::read_from_file(&tmp).unwrap();
        assert_eq!(file, restored);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_migrate_preset_current_version_roundtrip() {
        let preset = sample_preset();
        let file = PresetFile::from_preset(&preset, "MG-30");
        let yaml = serde_saphyr::to_string(&file).unwrap();
        let restored = migrate_preset(&yaml, OutputFormat::Yaml).unwrap();
        assert_eq!(file, restored);
    }

    #[test]
    fn test_migrate_preset_rejects_future_version() {
        let raw = "version: 9999\ndevice: MG-30\nslot: 01A\nname: Test\ntempo_ms: 500\nparallel: 0\nactive_scene: 0\neffect_order: []\n";
        let result = migrate_preset(raw, OutputFormat::Yaml);
        assert!(matches!(
            result,
            Err(ExportError::VersionTooNew {
                found: 9999,
                supported: PRESET_FILE_VERSION
            })
        ));
    }

    #[test]
    fn test_migrate_preset_rejects_version_zero() {
        let raw = "version: 0\ndevice: MG-30\nslot: 01A\nname: Test\ntempo_ms: 500\nparallel: 0\nactive_scene: 0\neffect_order: []\n";
        let result = migrate_preset(raw, OutputFormat::Yaml);
        assert!(matches!(
            result,
            Err(ExportError::VersionTooOld {
                found: 0,
                minimum: MIN_SUPPORTED_VERSION
            })
        ));
    }

    #[test]
    fn test_migrate_preset_rejects_garbage() {
        let result = migrate_preset("{{ not a preset", OutputFormat::Json);
        assert!(matches!(result, Err(ExportError::Deserialize(_))));
    }

    #[test]
    fn test_read_from_missing_file_is_read_error() {
        let path = std::env::temp_dir().join("nux_test_preset_does_not_exist.yaml");
        let result = PresetFile::read_from_file(&path);
        assert!(matches!(result, Err(ExportError::ReadFile { .. })));
    }

    #[test]
    fn test_write_to_missing_dir_is_write_error() {
        let file = PresetFile::from_preset(&sample_preset(), "MG-30");
        let path = std::env::temp_dir()
            .join("nux_test_no_such_dir")
            .join("preset.yaml");
        let result = file.write_to_file(&path, OutputFormat::Yaml);
        assert!(matches!(result, Err(ExportError::WriteFile { .. })));
    }
}
