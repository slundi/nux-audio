FX LOOP S/R		5 WAY ENCODER (ROTATE to CHANGE)	5 WAY ENCODER (TAP SW)	KNOB1 	KNOB2 	KNOB4 (TAP SW)
	SEND / RETURN	SEND / RETURN	ON / BYPASS	SEND (-15/+15dB)	RETURN (-15/+15dB)	RESTORE

## CC messages

0x80 0x80 [message CC=0xb0] [control number/ID] [value]

## SysEx messages

* Query version: F0 43? **58 00** F7 / [240, 67, 88, 0, 247]
* Change preset: F0 00 21 21 [DEVICE_ID] 01 [PRESET] F7
* Change parameter: F0 00 21 21 [DEVICE_ID] 02 [EFFECT] [PARAM] [VALUE] F7
* Toggle effect: F0 00 21 21 [DEVICE_ID] 03 [EFFECT] [ON/OFF] F7
* Tuner: F0 00 21 21 [DEVICE_ID] 04 [ON/OFF] F7
* Looper: F0 00 21 21 [DEVICE_ID] 06 [ACTION] F7

### MG-30

```ts
enum SysExRequest {
  DEVICE_VERSION = "58 00",
  CURRENT_PRESET_BASIC = "58 70 15 00",
  REQUEST_UPDATED_EFFECT_ORDER_COMMAND = "58 70 0C 00",
}

const SysExResponsePattern = {
  DEVICE_VERSION: [0xf0, 0x43, 0x58, 0x10],
  CURRENT_PRESET_BASIC: [0xf0, 0x43, 0x58, 0x70, 0x15, 0x02],
  CURRENT_PRESET_DETAIL: [0xf0, 0x43, 0x58, 0x70, 0x0b, 0x02],
  UPDATED_EFFECTS_ORDER: [0xf0, 0x43, 0x58, 0x70, 0x0c, 0x02],
  PRESET_CHANGED: [0xc0],
  SCENE_CHANGED: [0xb0, 0x4f],
  EFFECT_CHANGED: [0xb0],
  EFFECT_ORDER_CHANGED: [0xf0, 0x43, 0x58, 0x70, 0x7e, 0x02, 0x0d],
};
```

```yaml
sysex_templates:
  preset_request:
    template: [0xF0, 0x43, 0x58, "{device_id}", 0x10, "{preset_number}", 0xF7]
    description: "Request preset data"
    parameters:
      - device_id: "Device ID (0x01-0x09)"
      - preset_number: "Preset number (0x00-0x63)"
      
  preset_write:
    template: [0xF0, 0x43, 0x58, "{device_id}", 0x11, "{preset_number}", "{data}", 0xF7]
    description: "Write preset data"
    parameters:
      - device_id: "Device ID"
      - preset_number: "Preset number"
      - data: "Preset data (variable length)"
      
  effect_change:
    template: [0xF0, 0x43, 0x58, "{device_id}", 0x20, "{effect_id}", "{value}", 0xF7]
    description: "Change effect parameter"
    parameters:
      - device_id: "Device ID"
      - effect_id: "Effect ID"
      - value: "Parameter value"
      
  effect_bypass:
    template: [0xF0, 0x43, 0x58, "{device_id}", 0x21, "{effect_id}", "{bypass_state}", 0xF7]
    description: "Bypass/enable effect"
    parameters:
      - device_id: "Device ID"
      - effect_id: "Effect ID"
      - bypass_state: "0=bypass, 1=enable"
      
  amp_change:
    template: [0xF0, 0x43, 0x58, "{device_id}", 0x30, "{amp_id}", 0xF7]
    description: "Change amp model"
    parameters:
      - device_id: "Device ID"
      - amp_id: "Amp model ID"
      
  amp_parameter:
    template: [0xF0, 0x43, 0x58, "{device_id}", 0x31, "{param_id}", "{value}", 0xF7]
    description: "Change amp parameter"
    parameters:
      - device_id: "Device ID"
      - param_id: "Parameter ID (1=gain, 2=bass, 3=mid, 4=treble, 5=presence, 7=volume)"
      - value: "Parameter value (0-100)"
      
  # Enable secret amps? (Mighty Plug Pro/Space)
  secret_amps_unlock:
    template: [0xF0, 0x43, 0x58, "{device_id}", 0x7F, 0x53, 0x45, 0x43, 0x52, 0x45, 0x54, 0xF7]
    description: "Unlock secret amp models"
    parameters:
      - device_id: "Device ID (0x03 or 0x04 only)"
      
  tempo_sync:
    template: [0xF0, 0x43, 0x58, "{device_id}", 0x40, "{tempo_high}", "{tempo_low}", 0xF7]
    description: "Set tempo (BPM)"
    parameters:
      - device_id: "Device ID"
      - tempo_high: "Tempo high byte"
      - tempo_low: "Tempo low byte"
      
  # Drum/metronome control
  drum_control:
    template: [0xF0, 0x43, 0x58, "{device_id}", 0x50, "{command}", "{pattern}", "{volume}", 0xF7]
    description: "Control drum machine"
    parameters:
      - device_id: "Device ID"
      - command: "0=stop, 1=start, 2=pattern"
      - pattern: "Drum pattern ID (0-15)"
      - volume: "Drum volume (0-100)"

```

### Responses

#### Version query

Nux Mighty Plug Pro:  / f0 43 58 00 f7
