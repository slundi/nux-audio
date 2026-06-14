// Per-slot bypass toggle via MIDI CC.
//
// CC value 0x7F = slot active (on); 0x00 = slot bypassed (off).
// The CC number is read from `EffectCategory::bypass_cc()`.

use crate::effects::{CC_BYPASS_OFF, CC_BYPASS_ON, EffectCategory};
use crate::midi::MidiSend;

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

/// Core bypass logic: resolve the CC number and send it via `manager`.
///
/// Returns `(cc, value)` on success, or an error string suitable for `eprintln!`.
/// Extracted from `run()` so it can be tested without a real MIDI connection.
pub(crate) fn send_bypass(
    manager: &mut impl MidiSend,
    category: EffectCategory,
    state: BypassState,
) -> Result<(u8, u8), String> {
    let cc = category
        .bypass_cc()
        .ok_or_else(|| format!("The '{}' slot has no bypass toggle.", category.as_str()))?;
    let value = match state {
        BypassState::On => CC_BYPASS_ON,
        BypassState::Off => CC_BYPASS_OFF,
    };
    manager
        .send_cc(cc, value)
        .map_err(|e| format!("Failed to send bypass CC: {e}"))?;
    Ok((cc, value))
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

    let mut session = match super::connect_and_detect(device_name).await {
        Some(s) => s,
        None => return,
    };

    match send_bypass(&mut session.manager, category, state) {
        Ok((cc, value)) => println!(
            "{} {} → CC{cc}=0x{value:02X} ({})",
            category.as_str(),
            if state == BypassState::On {
                "ON"
            } else {
                "OFF"
            },
            session.device.model
        ),
        Err(e) => eprintln!("{e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::MockMidi;

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

    #[test]
    fn test_send_bypass_amp_on() {
        let mut mock = MockMidi::new();
        let (cc, value) = send_bypass(&mut mock, EffectCategory::Amp, BypassState::On).unwrap();
        assert_eq!(cc, 3); // Amp bypass CC = discriminant 3
        assert_eq!(value, CC_BYPASS_ON);
        assert_eq!(mock.sent, vec![vec![0xB0, 3, CC_BYPASS_ON]]);
    }

    #[test]
    fn test_send_bypass_reverb_off() {
        let mut mock = MockMidi::new();
        let (cc, value) = send_bypass(&mut mock, EffectCategory::Reverb, BypassState::Off).unwrap();
        assert_eq!(cc, 8); // Reverb bypass CC = discriminant 8
        assert_eq!(value, CC_BYPASS_OFF);
        assert_eq!(mock.sent, vec![vec![0xB0, 8, CC_BYPASS_OFF]]);
    }

    #[test]
    fn test_send_bypass_vol_has_no_cc() {
        let mut mock = MockMidi::new();
        let result = send_bypass(&mut mock, EffectCategory::Vol, BypassState::On);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no bypass toggle"));
        assert!(mock.sent.is_empty());
    }

    #[test]
    fn test_send_bypass_propagates_midi_error() {
        let mut mock = MockMidi {
            should_fail: true,
            ..MockMidi::new()
        };
        let result = send_bypass(&mut mock, EffectCategory::Amp, BypassState::On);
        assert!(result.is_err());
    }
}
