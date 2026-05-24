// Per-slot bypass toggle via MIDI CC.
//
// CC value 0x7F = slot active (on); 0x00 = slot bypassed (off).
// The CC number is read from `EffectCategory::bypass_cc()`.

use crate::effects::{CC_BYPASS_OFF, CC_BYPASS_ON, EffectCategory};

/// Whether to activate or bypass an effect slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BypassState {
    On,
    Off,
}

/// Parse `"on"` / `"off"` (and common aliases) for use as a clap `value_parser`.
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
pub async fn run(device_name: &Option<String>, category_str: &str, state: BypassState) {
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

    let cc = match category.bypass_cc() {
        Some(cc) => cc,
        None => {
            eprintln!("The '{}' slot has no bypass toggle.", category.as_str());
            return;
        }
    };

    let mut session = match super::connect_and_detect(device_name).await {
        Some(s) => s,
        None => return,
    };

    let value = match state {
        BypassState::On => CC_BYPASS_ON,
        BypassState::Off => CC_BYPASS_OFF,
    };

    match session.manager.send_cc(cc, value) {
        Ok(()) => println!(
            "{} {} → CC{cc}=0x{value:02X} ({})",
            category.as_str(),
            if state == BypassState::On {
                "ON"
            } else {
                "OFF"
            },
            session.device.model
        ),
        Err(e) => eprintln!("Failed to send bypass CC: {e}"),
    }
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
