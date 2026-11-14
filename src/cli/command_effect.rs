// MIDI CC knob control for a specific effect slot.
//
// Exposes the `knob=value` parameter parsers and `run()`; knob resolution and
// range checks live in `NuxSession::set_knob`.

use super::error::CliError;
use crate::session::{EffectParameter, KnobIdentifier, NuxSession};

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
/// Numeric knobs are treated as CC numbers directly. Every parameter is attempted;
/// failures are printed as they happen and summarised in the returned error.
pub async fn run(
    device_name: Option<&str>,
    effect: &str,
    parameters: &[EffectParameter],
) -> Result<(), CliError> {
    let category = super::parse_category(effect)?;
    let mut session = NuxSession::connect(device_name).await?;

    let mut failed = 0usize;
    for param in parameters {
        match session.set_knob(category, param).await {
            Ok((cc, value)) => println!(
                "CC{cc}={value} → {} ({})",
                category.as_str(),
                session.device().model
            ),
            Err(e) => {
                eprintln!("{param}: {e}");
                failed += 1;
            }
        }
    }

    if failed > 0 {
        return Err(CliError::PartialFailure {
            failed,
            total: parameters.len(),
        });
    }
    Ok(())
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

    #[test]
    fn test_parse_numeric_vs_named_knob() {
        let numeric = parse_effect_parameters("22=80").unwrap();
        assert!(matches!(numeric.knob, KnobIdentifier::Id(22)));
        let named = parse_effect_parameters(" gain = 80 ").unwrap();
        assert!(matches!(named.knob, KnobIdentifier::Name(ref name) if name == "gain"));
        assert_eq!(named.value, 80);
    }
}
