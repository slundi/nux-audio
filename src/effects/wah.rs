// WAH		                    5 WAY ENCODER (ROTATE to CHANGE)	5 WAY ENCODER (TAP SW)	KNOB1	KNOB2
// CLYDE (60's TRUMPET SOUND)	CLYDE	        ON / BYPASS		
// CRY BABY	                    CRY BB	        ON / BYPASS		
// VOX V847                     WAH	V847	    ON / BYPASS		
// MORLEY BAD HORSIE WAH	    HORSE WAH	    ON / BYPASS	    CONTOUR	RANGE
// DIGITECH WHAMMY	            OCTAVE SHIFT	ON / BYPASS	    UP/DOWN	

use std::{collections::HashMap, sync::LazyLock};

use super::{EffectOption, Knob};

static WAHS: LazyLock<HashMap<&str, EffectOption>> = LazyLock::new(|| {
    HashMap::from([
        (
            "CLYDE",
            EffectOption {
                on_byte: 0x02,
                off_byte: 41,
                title: "Clyde EffectOption",
                ..Default::default()
            },
        ),
        (
            "CRY_BB",
            EffectOption {
                on_byte: 0x04,
                off_byte: 0x42,
                title: "Cry Baby",
                ..Default::default()
            },
        ),
        (
            "V847",
            EffectOption {
                on_byte: 0x06,
                off_byte: 0x43,
                title: "V847",
                ..Default::default()
            },
        ),
        (
            "HORSE_WAH",
            EffectOption {
                on_byte: 0x08,
                off_byte: 0x44,
                title: "Horse EffectOption",
                knobs: vec![
                    Knob {
                        id: "contour",
                        title: "CONTOUR",
                        range_start: Some(0),
                        range_end: Some(100),
                        ctrl: 12,
                        current_value: None,
                    },
                    Knob {
                        id: "range",
                        title: "RANGE",
                        range_start: Some(0),
                        range_end: Some(5),
                        ctrl: 13,
                        current_value: None,
                    },
                ],
                ..Default::default()
            },
        ),
        (
            "OCTAVE_SHIFT",
            EffectOption {
                on_byte: 0x0a,
                off_byte: 0x45,
                title: "Octave Shift",
                knobs: vec![Knob {
                    id: "up-down",
                    title: "",
                    range_start: Some(0),
                    range_end: Some(1),
                    ctrl: 12,
                    current_value: None,
                }],
                ..Default::default()
            },
        ),
    ])
});
