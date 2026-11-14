# Roadmap

## Phase 1: Foundation & Discovery

* [x] **Core Data Structures**: `Manufacturer`, `Device` (model, device_id, MIDI port maps), `Preset` (number, name, scene, effect_order, effects map), `EffectOption` / `KnobDef` / `KnobState`, `ControllerState` — all implemented in `src/`.
* [x] **Device Registry**: `devices.toml` embedded at compile time and parsed at startup into a `static REGISTRY: LazyLock<DeviceRegistry>` (`src/lib/registry.rs`, `nux_audio::registry`). Each entry carries `device_id`, `model_code`, `preset_count`, `midi_port_prefix`, and `signal_chain`. `Device::extract_supported()` now delegates to `REGISTRY.match_port()` (longest-prefix match) instead of hardcoded branches. Effect model catalogs remain compiled-in Rust; `effects.yaml` is still reference-only.
* [x] **Hardware abstraction**: `MidiManager` with `midir` for port enumeration, connect, send, disconnect on Linux/ALSA. `Device::detect()` matches MIDI port names against 6 supported Nux device models.
* [x] **Handshake Protocol**: `NuxSession::connect()` selects the device (`select_device` / `device_ports`), opens a `MidiConnection`, sends `DEVICE_VERSION_REQUEST` and waits up to 3 s for the `[0xF0, 0x43, 0x58, 0x10]` response via `MidiTransport::request_sysex` (subscribes before sending, so a fast reply cannot be missed). Firmware version is parsed by `extract_version()`; `nux info` displays it alongside model, device ID, preset count, and signal chain from the registry.
* [x] **Session service** (`src/session/`): `NuxSession<T: MidiTransport>` owns transport + device + firmware generation and exposes `set_bypass`, `set_knob`, `set_model`, `fetch_preset`, `current_preset[_raw]`, `preset_detail_raw`, all returning a typed `SessionError`. CLI commands are thin wrappers (parse → session → print) returning `CliError`; `nux-cli` now exits with status 1 on error. The TUI can reuse the same service (Phase 5 wiring pending).
* [x] **Library split**: pure data and decoding live in the `nux_audio` library crate (`src/lib/`): `effects` (categories, catalogs, lookups), `preset` (model + layout-independent decoders), `driver`, `registry`, `nux_sys_ex_commands`, `string`. One library error type, `NuxAudioError`. The `nux-cli` binary keeps MIDI I/O (`midi`, `device`), `session`, `export`/`storage`, CLI and TUI. Library is fully documented (`#![deny(missing_docs)]`).
* [x] **Device drivers** (`nux_audio::driver`): `DeviceDriver` trait owns everything device- or firmware-specific (bypass CC, knob/model lookups, model-change payload, preset requests + response headers, basic/detail decoders). `DriverKind::for_model` picks the family from the registry model code; `from_version_response` builds the generation-aware driver. `Mg30Driver` is the only implementation and holds the single `FirmwareGen` branch: GEN1 (fw 4.0.3) vs GEN2 (fw ≥ 5) detail layout. The GEN2 decoder reads only confirmed fields (slot, BPM + sync, name) — effect order, scenes and parallel stay empty until mapped. Devices without a driver (Mighty series) are refused at handshake with `SessionError::UnsupportedDevice`.
  - [ ] Mighty series driver (needs protocol captures).
  - [ ] GEN2 detail layout: effect chain, scenes, parallel blend.

## Phase 2: Communication Engine

* [x] **SysEx Templating**: `build_sysex_message()` and `send_sysex()` / `send_sysex_raw()` implemented. All known MG-30 request/response byte patterns defined as constants. Parameterized typed builders for MG-30 (`mg30_request_preset`, `mg30_change_preset`, `mg30_save_preset`, `mg30_looper_control`, `mg30_set_effect_order`) and Mighty series (`mighty_change_preset`). `SysExTemplate` + `TemplateSlot` system for runtime parameter substitution; predefined templates in `nux_sys_ex_commands::templates` mirror each typed builder and are verified by tests.
* [x] **Real-time Control (CC)**: All CC numbers mapped in `KnobDef.ctrl` for every effect slot and model. `MidiManager::send_cc(control, value)` sends a validated CC message (rejects values > 127 with `MidiError::CcValueOutOfRange`). `MidiManager::send_raw_midi` exposes the raw output port. `EffectCategory::bypass_cc()` maps each slot to its bypass CC (discriminant 0–10; `Vol` returns `None`). `CC_BYPASS_ON` / `CC_BYPASS_OFF` constants provided.
* [x] **Asynchronous handling**: `MidiDispatcher` backed by `tokio::sync::broadcast` bridges sync midir callbacks to async tokio. `connect_with_dispatcher()` on `MidiManager` replaces the old `spawn_blocking` + `recv_timeout` pattern. `info()` uses `dispatcher.await_response()` with a typed timeout.

## Phase 3: Preset Management

* [x] **Preset Fetcher**: All parsers now return `Result` with `NuxAudioError` bounds checks (no panics). `extract_current_preset_basic_data()` parses preset number and active scene. The detail response is decoded per firmware by `Mg30Driver::decode_preset_detail`: GEN1 (`decode_detail_gen1`) parses tempo (ms/beat), parallel blend, effect signal-chain order, preset name, and three scene bypass snapshots; GEN2 (`decode_detail_gen2`) parses slot, BPM + sync and name. `extract_effects_order()` bug fixed (`sliced[i]` instead of `data[i]`). `extract_name()` bounds-safe 3-byte pair decoder. `SceneSnapshot([u8; 3])` added to `Preset`. Per-slot effect model IDs in the detail response require hardware captures to confirm offsets.
* [ ] **Bulk Operations**: Listing and downloading all presets not implemented.
* [x] **Serialization**: `PresetFile` (versioned, device-tagged) with `EffectSlotFile` / `KnobFile` owned types in `src/export.rs`. Serialize with `--yaml` (default) or `--json` global CLI flags via `OutputFormat`. `to_string_fmt` / `from_str_fmt` for in-memory round-trips; `write_to_file` / `read_from_file` (auto-detects format from `.json` / `.yaml` extension). Full YAML+JSON round-trip tests.

## Phase 4: Effect Control

* [x] **Effect model catalog**: Full MG-30 static catalog complete — wah, comp, efx, amp, eq, gate, mod, delay, reverb, ir, vol (100+ models with correct CC assignments).
* [x] **CC send**: `nux effect <category> <knob>=<value>` sends MIDI CC in real time. Named knobs resolved via catalog (`EffectCategory` impl `FromStr`; `find_cc_for_knob`). Numeric knobs treated as direct CC numbers. `MidiManager::connect_output` added for send-only connections.
* [x] **Bypass toggle**: `nux bypass <category> on|off` sends the slot's bypass CC (`EffectCategory::bypass_cc()`). `Vol` and `Sr` are rejected with a clear error. Accepts `on/off/1/0/enable/disable`.
* [x] **Model switch**: `nux model <category> <model>` sends `mg30_change_effect_model` SysEx. Model resolved by hex ID (`0x45`), decimal ID (`69`), or case-insensitive name search (`find_model_by_name`). SysEx format provisional pending hardware capture.

## Phase 5: CLI (stateless)

Commands like `nux set amp-gain 50` or `nux dump-preset > my_tone.yaml`

* [~] **CLI structure**: `bpaf`-based commands exist — `devices`, `info`, `preset show/get/send`, `effect`, `bypass`, `model`, `listen`, `raw`, `tui`. `preset get` fetches and writes to `--yaml`/`--json` file. `preset send` exits with code 2 (write-preset SysEx opcode not yet reverse-engineered). `raw` can send arbitrary hex/decimal SysEx.
* [~] **Functional commands**: `preset show`, `preset get`, `effect <cat> <knob>=<value>`, `bypass`, `model`, `info` all wired to real MIDI. `preset send` and bulk operations still pending.

## Phase 6: TUI (stateful)

### Design Decisions

**Layout**: single-pane preset list + bottom effect-chain panel + top tab bar.

```
┌──────────────────────────────────────────────────────────────────┐
│ nux-audio  [Presets]  [Live Board]  [About]        MG-30 fw5.0.2 │
├──────────────────────────────────────────────────────────────────┤
│  Src  Slot  Name               Effects                           │
│  📟   001   Crunch Lead        [AMP·DLY·REV]                     │
│  📟   002   Clean Jazz         [COMP·AMP·REV]                    │
│  📄    —    My Blues Driver    [WAH·AMP·MOD·DLY]                 │
│  📟📄  003   Lead Boost *      [WAH·AMP·MOD·DLY]  ← local+device │
├──────────────────────────────────────────────────────────────────┤
│ Effect Chain: Clean Jazz  [Scene 1]  [Scene 2]  [Scene 3]        │
│ [    ]──[COMP]──[    ]──[ AMP ]──[ EQ ]──[    ]──[    ]──[REV]  │
│          Ratio:4         TubeCl   Low:40            Hall         │
│          The:-10         Gain:30  Mid:50            Mix:35       │
└──────────────────────────────────────────────────────────────────┘
 d:device  g:get  s:send  e:export  /:search  ?:help
```

- `📟` = device only, `📄` = local only, `📟📄` = both exist
- `*` + color on name = local has unsaved/unsynced changes
- Green border on effect block = active, dim = bypassed

**Editing**:
- All editing is local-only. Device presets are pulled to a local copy first (`g`).
- Editing a local preset sends CC messages live to the device (if connected) for immediate audio feedback.
- Changes tracked with `*` indicator. `:w` / `S` persists to XDG local store.
- In the bottom panel, `←`/`→` navigates effect slots; `Enter` expands to show all knobs.
- Knob navigation: `j`/`k` between knobs, `+`/`-` or `←`/`→` to adjust, CC sent instantly.
- `m` on a focused effect block opens a popup model picker for that slot.
- `r` resets knob to default, `u` undoes last change, `Esc` collapses.

**Scenes**: `Tab` cycles Scene 1 → 2 → 3 in the bottom panel (and sends scene-switch CC).

**Tabs** (3 total):
1. **[Presets]** — single-pane list + bottom effect chain panel
2. **[Live Board]** — full-screen CC performance view, all effect slots and knobs visible
3. **[About]** — version, repo URL, full key bindings reference

**Overlays** (floating popups):
- `d` → device picker (on startup if multiple devices found)
- `l` → looper control (rec/play/stop/undo, BPM, vol)
- `D` (uppercase) → drum machine (pattern list, BPM ±, vol, play/stop)

**Key bindings**:

| Key | Action |
|-----|--------|
| `d` | device picker popup (ONLY SHOWN if multiple supported devices) |
| `g` | get device preset → local copy |
| `s` | send local preset → device slot |
| `e` | export preset to file |
| `/` | search/filter preset list |
| `1`–`9` | jump to effect slot in bottom panel / live board |
| `Shift+1`–`9` | toggle bypass for that slot |
| `n` | noise gate slot |
| `v` | volume slot |
| `l` | looper overlay |
| `D` | drum overlay |
| `m` | model picker for focused effect |
| `Tab` | cycle scenes (Scene 1/2/3) in bottom panel |
| `r` | reset focused knob to default |
| `u` | undo last knob change |
| `F1`/`F2`/`F3` | switch top-level tabs |
| `?` | help / key bindings |

### Tasks

* [x] 6.1 **Persistence**: XDG local config and preset library stored as YAML via `serde-saphyr`.
  - `~/.local/share/nux-audio/config.toml` — last device, preferences.
  - `~/.local/share/nux-audio/presets/mg30.yaml` — one file per device model, all presets as a list.
  - Each preset entry carries `slot: Option<u8>` (null = local-only), `firmware_gen`, `name`, `modified` timestamp, and per-effect knob values.
  - `slot` drives the source icon: null → `📄`, set → `📟`, both copies exist → `📟📄`.
* [x] 6.2 **Preset list**: single-pane with `📟`/`📄` source icons, slot, name, effect summary. Empty-state message when no presets exist. (search and get/send wired in later phases).
* [x] 6.3 **Bottom effect chain panel**: all 11 MG-30 slots visualized, active slots green, scene switcher with `Tab`, slot focus with `←`/`→`, `Enter` to focus/unfocus.
* [x] 6.4 **Live Board tab**: full-screen 11-slot grid, one column per effect, active/inactive state, scene bar.
* [x] 6.7 **About tab**: project info (name, version, repo) and full key-bindings reference table.
* [x] 6.5 **Overlays**: device picker (`d`), looper (`l`), drum machine (`D`). Nerd Font status icon (󰣱/󰣲/spinner) in tab bar; icon legend in About tab.
* [~] 6.6 **Get/Send**: `g` pulls the device's current preset into the local library; `s` (send) is not possible yet.
  - Conflict popup (`[O]verride` / `[N]o` / `[R]ename+timestamp`) when slot already exists in library.
  - Transient status messages in status bar (auto-clears after ~3 s).
  - [x] Real device I/O: the TUI talks to a `session::worker` task over channels (`SessionCommand` → `SessionEvent`), so device round-trips never block the UI. Device picker connects for real (spinner + busy label while connecting, model + firmware in the tab bar, error on failure); `g` fetches the real current preset.
  - [ ] `s`: blocked on the write-preset SysEx (same as CLI `preset send`); currently reports "not supported" and leaves the library untouched.
  - [ ] Looper / drum overlays still local-only (status bar shows a "UI only — not sent to device" banner while they are open).
  - [ ] Live device updates (knob turns / preset changes pushed by the pedal) — `MidiConnection::subscribe` exists, not yet surfaced as events.
* [x] 6.8 **Mouse support**: tab-bar clicks, preset-list row click + scroll, chain-panel slot click, live-board slot/scene click, device picker click + scroll, drum pattern click + scroll, conflict popup O/N/R click zones.
* [x] 6.9 **Configurable key bindings**: every TUI key goes through `ui::keymap::KeyMap` (`(Context, Key) → Action`), loaded from `~/.config/nux-audio/keybindings.yaml` (separate from `config.yaml` so `save_config` never rewrites it). Contexts: `global`, `presets`, `live_board`, `help`, `device_picker`, `looper`, `drum`, `conflict`. Only the actions listed in the file are overridden; several keys per action; `[]` unbinds. Conflicts and bad entries are logged and reported in the status bar, the entry falls back to its default. `Ctrl+C` always quits. Help overlay, tab titles and hint bars are generated from the active bindings. `keybindings.example.yaml` documents the defaults (a test keeps it in sync). The About tab table is still static.

## Phase 7: Community & Extensibility

* [x] **Error Handling**: `thiserror` used throughout. Some panics remain (e.g. `midir_test`).
  - `MidiManager` / `MidiSend` return a typed `MidiError` (no more `Box<dyn Error>`); preset file I/O returns `ExportError`, so `export.rs` no longer depends on `cli::error::CliError`. `CliError` wraps both via `#[from]`.
* [~] **Logging**: `tracing` + `tracing-subscriber` configured; XDG log file placement done in `configure_logger`.
* [x] **Testing**: Mock MIDI backend for CI/CD testing without physical hardware — `MockMidi` implements `MidiTransport` with a FIFO of canned device responses; all session operations (handshake, bypass, knobs, models, preset fetch) are unit-tested through it.
* [ ] **External Mapping Support**: Allow users to load custom YAML files for unsupported pedals.
* [ ] **Documentation**: Document the SysEx/CC mapping process for community contributors.

## Phase 8: GUI

## Old roadmap

* [x] Connect to a MIDI device
* [~] Detect the device type — `Device::detect()` works for 6 Nux models; not yet used for routing
* [~] Get the device version — request sent, response not yet parsed into device state
* [~] Get the current preset — basic data and name parsers done; detail and effects parsing skeletal
* [ ] List device presets when applicable
* [ ] Get/export device preset when applicable
* [ ] Send/import device preset when applicable
* [ ] Apply preset when applicable
* [x] Create all effects — MG-30 static catalog complete
* [ ] Control effect
* [ ] async calls to MIDI

There is reverse engineering to do so there are unknowns.

### Unknown

For now the focus is on MG-30. Other Nux devices (Mighty series) may share some SysEx patterns.

Various unknown features:
* [x] Use XDG for the log file
* [ ] Use XDG for configuration and preset folders
* [ ] Convert presets for another device
* [x] MIDI CC mapping — numbers documented; send not yet implemented
* [ ] drum & loop
* [ ] conversion between various patch format when possible or lossy

### TUI ideas

- [ ] keyboard mapping
- [ ] startup: device choice
- [ ] board **Amp Academy** / **MLD Bass Preamp + DI**:
  - [ ] current preset
  - [ ] IR choice
  - [ ] select amp
  - [ ] amp control (`g` gain, `l` low/bass, `m` middle, `h` high/treble, `v`: master volume, `b` bias)
  - [ ] display IR
  - [ ] select scene (keys: `shift + s`, `shift + 1`, `shift + 2`, `shift + 3`)
  - [ ] effect configuration (`n` noise gate, `r` reverb, `d` delay, `e` equalizer, `x` efx, `c` compression, `i` IR, `a` amp), `shift` to enable/disable

### Dropped

* Connect to a MIDI device through BLE (Bluetooth Low Energy): the program will not handle USB or bluetooth connection. MIDIR lib can communicate with any MIDI connected devices so this is useless.
