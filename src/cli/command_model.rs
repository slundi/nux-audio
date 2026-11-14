// Effect model switch via SysEx.
//
// Accepts a model by hex ID (0x45), decimal ID (69), or case-insensitive name.
// Wire format is provisional — not yet verified against hardware captures.

use super::error::CliError;
use crate::session::{ModelRef, NuxSession};

/// Parse the MODEL argument.
///
/// - `"0x45"` or `"0X45"` — hex model ID
/// - `"69"` — decimal model ID
/// - anything else — a catalog name, matched case-insensitively by the session
pub fn parse_model_ref(model_str: &str) -> Result<ModelRef, CliError> {
    if let Some(hex) = model_str
        .strip_prefix("0x")
        .or_else(|| model_str.strip_prefix("0X"))
    {
        return u8::from_str_radix(hex, 16)
            .map(ModelRef::Id)
            .map_err(|_| CliError::InvalidModelId(model_str.to_owned()));
    }
    Ok(match model_str.parse::<u8>() {
        Ok(id) => ModelRef::Id(id),
        Err(_) => ModelRef::Name(model_str.to_owned()),
    })
}

/// Switch the active model within an effect slot.
pub async fn run(
    device_name: Option<&str>,
    category_str: &str,
    model_str: &str,
) -> Result<(), CliError> {
    let category = super::parse_category(category_str)?;
    let model = parse_model_ref(model_str)?;
    let mut session = NuxSession::connect(device_name).await?;
    let change = session.set_model(category, &model)?;
    let name_part = change
        .title
        .map(|title| format!(" ({title})"))
        .unwrap_or_default();
    println!(
        "{} model → 0x{:02X}{name_part} [provisional SysEx] ({})",
        category.as_str(),
        change.id,
        session.device().model
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_model_ref_hex() {
        assert_eq!(parse_model_ref("0x45").unwrap(), ModelRef::Id(0x45));
        assert_eq!(parse_model_ref("0X0a").unwrap(), ModelRef::Id(0x0A));
    }

    #[test]
    fn parse_model_ref_invalid_hex_errors() {
        assert!(matches!(
            parse_model_ref("0xZZ"),
            Err(CliError::InvalidModelId(raw)) if raw == "0xZZ"
        ));
        assert!(parse_model_ref("0x100").is_err());
    }

    #[test]
    fn parse_model_ref_decimal_and_name() {
        assert_eq!(parse_model_ref("69").unwrap(), ModelRef::Id(69));
        assert_eq!(
            parse_model_ref("Hall").unwrap(),
            ModelRef::Name("Hall".to_owned())
        );
        // Out-of-range decimal is not an ID — falls through to a name lookup.
        assert_eq!(
            parse_model_ref("300").unwrap(),
            ModelRef::Name("300".to_owned())
        );
    }
}
