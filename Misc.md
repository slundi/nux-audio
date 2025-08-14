FX LOOP S/R		5 WAY ENCODER (ROTATE to CHANGE)	5 WAY ENCODER (TAP SW)	KNOB1 	KNOB2 	KNOB4 (TAP SW)
	SEND / RETURN	SEND / RETURN	ON / BYPASS	SEND (-15/+15dB)	RETURN (-15/+15dB)	RESTORE

## CC messages

0x80 0x80 [message CC=0xb0] [control number/ID] [value]

## SysEx messages

Query version: F0 43? **58 00** F7 / [240, 67, 88, 0, 247]
Change preset: F0 00 21 21 [DEVICE_ID] 01 [PRESET] F7
Change parameter: F0 00 21 21 [DEVICE_ID] 02 [EFFECT] [PARAM] [VALUE] F7
Toggle effect: F0 00 21 21 [DEVICE_ID] 03 [EFFECT] [ON/OFF] F7
Tuner: F0 00 21 21 [DEVICE_ID] 04 [ON/OFF] F7
Looper: F0 00 21 21 [DEVICE_ID] 06 [ACTION] F7

### Responses

#### Version query

Nux Mighty Plug Pro:  / f0 43 58 00 f7
