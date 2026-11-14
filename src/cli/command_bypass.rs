// Per-slot bypass toggle via MIDI CC.
//
// CC value 0x7F = slot active (on); 0x00 = slot bypassed (off).
// The device logic lives in `NuxSession::set_bypass`; this file parses and prints.

use super::error::CliError;
use crate::session::{BypassState, NuxSession};

/// Parse `"on"` / `"off"` (and common aliases) for the bypass state positional argument.
pub fn parse_bypass_state(s: &str) -> Result<BypassState, String> {
    match s.to_lowercase().as_str() {
        "on" | "1" | "true" | "enable" => Ok(BypassState::On),
        "off" | "0" | "false" | "disable" => Ok(BypassState::Off),
        _ => Err(format!("Expected 'on' or 'off', got '{s}'")),
    }
}

/// Send the bypass CC for an effect slot.
///
/// `On` → CC value `0x7F` (slot active); `Off` → `0x00` (slot bypassed).
pub async fn run(
    device_name: Option<&str>,
    category_str: &str,
    state: BypassState,
) -> Result<(), CliError> {
    let category = super::parse_category(category_str)?;
    let mut session = NuxSession::connect(device_name).await?;
    let (cc, value) = session.set_bypass(category, state).await?;
    println!(
        "{} {} → CC{cc}=0x{value:02X} ({})",
        category.as_str(),
        if state == BypassState::On {
            "ON"
        } else {
            "OFF"
        },
        session.device().model
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bypass_state_variants() {
        assert_eq!(parse_bypass_state("on"), Ok(BypassState::On));
        assert_eq!(parse_bypass_state("ON"), Ok(BypassState::On));
        assert_eq!(parse_bypass_state("1"), Ok(BypassState::On));
        assert_eq!(parse_bypass_state("enable"), Ok(BypassState::On));
        assert_eq!(parse_bypass_state("off"), Ok(BypassState::Off));
        assert_eq!(parse_bypass_state("0"), Ok(BypassState::Off));
        assert_eq!(parse_bypass_state("disable"), Ok(BypassState::Off));
        assert!(parse_bypass_state("maybe").is_err());
    }
}
