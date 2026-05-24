// MIDI CC knob control for a specific effect slot.
//
// Exposes `EffectParameter`, `KnobIdentifier`, parameter parsers, and `run()`.

use crate::effects::{EffectCategory, find_cc_for_knob};

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
        let cc = match &param.knob {
            KnobIdentifier::Id(n) => *n,
            KnobIdentifier::Name(name) => match find_cc_for_knob(category, name.as_str()) {
                Some(cc) => cc,
                None => {
                    eprintln!(
                        "Knob '{name}' not found in {} catalog. \
                         Use a CC number directly (e.g. 22=80).",
                        category.as_str()
                    );
                    errors += 1;
                    continue;
                }
            },
        };
        match session.manager.send_cc(cc, param.value) {
            Ok(()) => println!(
                "CC{cc}={} → {} ({})",
                param.value,
                category.as_str(),
                session.device.model
            ),
            Err(e) => {
                eprintln!("Failed to send CC{cc}={}: {e}", param.value);
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
}
