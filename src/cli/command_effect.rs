// MIDI CC knob control for a specific effect slot.
//
// Exposes `EffectParameter`, `KnobIdentifier`, parameter parsers, and `run()`.

use crate::effects::{EffectCategory, find_knob_def};
use crate::midi::MidiSend;

#[derive(Debug, Clone)]
pub struct EffectParameter {
    pub knob: KnobIdentifier,
    pub value: u8,
}

#[derive(Debug, Clone)]
pub enum KnobIdentifier {
    Id(u8),
    Name(String),
}

impl std::fmt::Display for EffectParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.knob {
            KnobIdentifier::Id(id) => write!(f, "{}={}", id, self.value),
            KnobIdentifier::Name(name) => write!(f, "{}={}", name, self.value),
        }
    }
}

/// Parse a single `"knob=value"` CLI argument.
///
/// `knob` may be a numeric CC number (`u8`) or a name string.
pub fn parse_effect_parameters(s: &str) -> Result<EffectParameter, String> {
    let parts: Vec<&str> = s.split('=').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid format '{}'. Expected: 'knob=value'", s));
    }
    let knob_str = parts[0].trim();
    let value_str = parts[1].trim();
    let knob = if let Ok(id) = knob_str.parse::<u8>() {
        KnobIdentifier::Id(id)
    } else {
        KnobIdentifier::Name(knob_str.to_string())
    };
    let value = value_str.parse::<u8>().map_err(|_| {
        format!(
            "Invalid value '{}'. Expected: number between 0 and 255",
            value_str
        )
    })?;
    Ok(EffectParameter { knob, value })
}

/// Parse a comma-separated list of `"knob=value"` parameters.
pub fn parse_effect_params_from_string(input: &str) -> Result<Vec<EffectParameter>, String> {
    input
        .split(',')
        .map(|param| parse_effect_parameters(param.trim()))
        .collect()
}

/// Send a single knob parameter to the device via CC.
///
/// Resolves named knobs (e.g. `"gain"`) to CC numbers via the static catalog.
/// Returns `(cc, value)` on success, or an error string for display.
/// Extracted from `run()` so it can be tested without a real MIDI connection.
pub(crate) async fn send_effect(
    manager: &mut impl MidiSend,
    category: EffectCategory,
    param: &EffectParameter,
) -> Result<(u8, u8), String> {
    let cc = match &param.knob {
        KnobIdentifier::Id(n) => *n,
        KnobIdentifier::Name(name) => {
            let knob = find_knob_def(category, name.as_str()).ok_or_else(|| {
                format!(
                    "Knob '{name}' not found in {} catalog. Use a CC number directly (e.g. 22=80).",
                    category.as_str()
                )
            })?;
            // Range validation is only applied to named knobs — numeric CC ids
            // are the raw escape hatch and the caller opts out of catalog checks.
            if let Some(end) = knob.range_end
                && param.value > end
            {
                return Err(format!(
                    "Value {} out of range for {} knob '{}' (0..={end})",
                    param.value,
                    category.as_str(),
                    name,
                ));
            }
            knob.ctrl
        }
    };
    manager
        .send_cc(cc, param.value)
        .await
        .map_err(|e| format!("Failed to send CC{cc}={}: {e}", param.value))?;
    Ok((cc, param.value))
}

/// Send MIDI CC messages for each knob parameter on the given effect category.
///
/// Named knobs (e.g. `"gain"`) are resolved to CC numbers via the static catalog.
/// Numeric knobs are treated as CC numbers directly.
pub async fn run(device_name: &Option<String>, effect: &str, parameters: &[EffectParameter]) {
    let category = match effect.parse::<EffectCategory>() {
        Ok(c) => c,
        Err(()) => {
            eprintln!(
                "Unknown effect category '{effect}'. \
                 Try: wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir, vol"
            );
            return;
        }
    };

    let mut session = match super::connect_and_detect(device_name).await {
        Some(s) => s,
        None => return,
    };

    let mut errors = 0usize;
    for param in parameters {
        match send_effect(&mut session.manager, category, param).await {
            Ok((cc, value)) => println!(
                "CC{cc}={value} → {} ({})",
                category.as_str(),
                session.device.model
            ),
            Err(e) => {
                eprintln!("{e}");
                errors += 1;
            }
        }
    }

    if errors > 0 {
        eprintln!("{errors} error(s) occurred");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::MockMidi;

    #[test]
    fn test_parse_multiple_effect_params() {
        let result = parse_effect_params_from_string("gain=80,tone=50,level=100");
        assert!(result.is_ok());
        let params = result.unwrap();
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].value, 80);
        assert_eq!(params[1].value, 50);
        assert_eq!(params[2].value, 100);
    }

    #[test]
    fn test_invalid_effect_param_format() {
        let result = parse_effect_parameters("invalid_format");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid format"));
    }

    #[test]
    fn test_invalid_effect_param_value() {
        let result = parse_effect_parameters("gain=invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid value"));
    }

    #[tokio::test]
    async fn test_send_effect_named_knob_resolves_cc() {
        let mut mock = MockMidi::new();
        let param = EffectParameter {
            knob: KnobIdentifier::Name("gain".to_owned()),
            value: 80,
        };
        let (cc, value) = send_effect(&mut mock, EffectCategory::Amp, &param)
            .await
            .unwrap();
        // Amp gain is CC22 per MIDI_CC_assignments.md
        assert_eq!(cc, 22);
        assert_eq!(value, 80);
        assert_eq!(mock.sent, vec![vec![0xB0, 22, 80]]);
    }

    #[tokio::test]
    async fn test_send_effect_numeric_cc_direct() {
        let mut mock = MockMidi::new();
        let param = EffectParameter {
            knob: KnobIdentifier::Id(52),
            value: 64,
        };
        let (cc, value) = send_effect(&mut mock, EffectCategory::Delay, &param)
            .await
            .unwrap();
        assert_eq!(cc, 52);
        assert_eq!(value, 64);
        assert_eq!(mock.sent, vec![vec![0xB0, 52, 64]]);
    }

    #[tokio::test]
    async fn test_send_effect_unknown_knob_name_errors() {
        let mut mock = MockMidi::new();
        let param = EffectParameter {
            knob: KnobIdentifier::Name("nonexistent".to_owned()),
            value: 50,
        };
        let result = send_effect(&mut mock, EffectCategory::Amp, &param).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
        assert!(mock.sent.is_empty());
    }

    #[tokio::test]
    async fn test_send_effect_propagates_midi_error() {
        let mut mock = MockMidi {
            should_fail: true,
            ..MockMidi::new()
        };
        let param = EffectParameter {
            knob: KnobIdentifier::Id(22),
            value: 80,
        };
        let result = send_effect(&mut mock, EffectCategory::Amp, &param).await;
        assert!(result.is_err());
    }

    // Range-validation for named knobs — the "raw CC by numeric id" path
    // intentionally bypasses this to remain an escape hatch.

    #[tokio::test]
    async fn test_send_effect_rejects_value_above_knob_range() {
        // IR "mic" is CC64 with range 0..=7 (DYN421/S57/…).
        let mut mock = MockMidi::new();
        let param = EffectParameter {
            knob: KnobIdentifier::Name("mic".to_owned()),
            value: 100, // way above the 0..=7 range
        };
        let err = send_effect(&mut mock, EffectCategory::Ir, &param)
            .await
            .unwrap_err();
        assert!(err.contains("out of range"), "unexpected error: {err}");
        assert!(err.contains("0..=7"), "unexpected error: {err}");
        assert!(
            mock.sent.is_empty(),
            "invalid value must not be transmitted"
        );
    }

    #[tokio::test]
    async fn test_send_effect_accepts_value_at_range_end() {
        // "mic" range is 0..=7 — value 7 must be accepted.
        let mut mock = MockMidi::new();
        let param = EffectParameter {
            knob: KnobIdentifier::Name("mic".to_owned()),
            value: 7,
        };
        let (cc, value) = send_effect(&mut mock, EffectCategory::Ir, &param)
            .await
            .unwrap();
        assert_eq!(cc, 64);
        assert_eq!(value, 7);
    }

    #[tokio::test]
    async fn test_send_effect_rejects_one_over_range_end() {
        // Boundary: mode range is 0..=6, so 7 must be rejected.
        let mut mock = MockMidi::new();
        let param = EffectParameter {
            knob: KnobIdentifier::Name("mode".to_owned()),
            value: 7,
        };
        let err = send_effect(&mut mock, EffectCategory::Gate, &param)
            .await
            .unwrap_err();
        assert!(err.contains("0..=6"), "unexpected error: {err}");
        assert!(mock.sent.is_empty());
    }

    #[tokio::test]
    async fn test_send_effect_numeric_id_skips_range_check() {
        // IR mic (CC64) has range 0..=7 — but with a raw numeric id, we do NOT
        // consult the catalog, so value 100 (still ≤ 127 MIDI limit) succeeds.
        let mut mock = MockMidi::new();
        let param = EffectParameter {
            knob: KnobIdentifier::Id(64),
            value: 100,
        };
        let (cc, value) = send_effect(&mut mock, EffectCategory::Ir, &param)
            .await
            .unwrap();
        assert_eq!(cc, 64);
        assert_eq!(value, 100);
    }
}
