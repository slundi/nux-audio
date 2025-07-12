# Design

```mermaid
classDiagram

namespace Device {
    class Manufacturer {
        <<enumeration>>
    }

    class MidiDevice{
        +model: String
        +get_name()
    }

    class Patch {
        +name: str
        +effects: Vec<(Pedal,int)>
    }
}

MidiDevice "1" -- "*" Patch
MidiDevice "*" -- "1" Manufacturer
MidiDevice "*" -- "*" Pedal: available pedals

namespace Effect {
    class Category {
        <<enumeration>>
    }
    class Pedal {
        +name: str
    }
    class Knob {
        +label: str
        +min: int
        +max: int
    }
}

Pedal "*" -- "1" Category
Pedal *-- Knob
note for Pedal "aka Effect"

namespace Config {
    class Default {
        +value: int
    }

    class Preset {
        +name: str
        +value: int
    }
}

Default "0..1" -- "1" Knob
Preset "*" -- "1" Pedal
```

## Config

It is used to save default values on effects/pedals. Example:
* I want my IR to have a low pass a 12kHz
* It can be used to set default volume levels on various effect

Preset is to name a full pedal/effect config.

## Handle per device

Depending on the model, MIDI CC, SysEx messages, ... can be different.

Rabit holes:
* same manufacturer, same effects: different CC, ID, ...
