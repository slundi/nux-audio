//! Configurable TUI key bindings.
//!
//! Contains the default bindings, the loader for
//! `~/.config/nux-audio/keybindings.yaml`, the `(Context, Key) → Action`
//! lookup used by the key handlers, and the helpers that format bindings for
//! the help overlay and the key-hint bars.
//!
//! The file maps contexts to actions to a list of keys. Only the actions
//! listed are overridden, the others keep their defaults; an empty list
//! unbinds the action. A malformed entry never aborts the load: it is skipped
//! and reported in [`KeyMap::warnings`].

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;
use serde::de::IntoDeserializer;
use serde::de::value::StrDeserializer;

use super::action::{Action, Context};
use super::key::Key;
use super::state::Overlay;

/// Default bindings, in display order. `keybindings.example.yaml` mirrors this
/// table (a test keeps them in sync).
const DEFAULTS: &[(Context, Action, &[&str])] = &[
    (Context::Global, Action::Quit, &["q"]),
    (Context::Global, Action::Help, &["?"]),
    (Context::Global, Action::ChangeDevice, &["d"]),
    (Context::Global, Action::TabPresets, &["<F1>"]),
    (Context::Global, Action::TabLiveBoard, &["<F2>"]),
    (Context::Global, Action::TabAbout, &["<F3>"]),
    (Context::Global, Action::OpenLooper, &["l"]),
    (Context::Global, Action::OpenDrum, &["D"]),
    (Context::Presets, Action::Up, &["k", "<Up>"]),
    (Context::Presets, Action::Down, &["j", "<Down>"]),
    (Context::Presets, Action::Left, &["h", "<Left>"]),
    (Context::Presets, Action::Right, &["<Right>"]),
    (Context::Presets, Action::Confirm, &["<CR>"]),
    (Context::Presets, Action::Close, &["<Esc>"]),
    (Context::Presets, Action::NextScene, &["<Tab>"]),
    (Context::Presets, Action::GetPreset, &["g"]),
    (Context::Presets, Action::SendPreset, &["s"]),
    (Context::LiveBoard, Action::Left, &["h", "<Left>"]),
    (Context::LiveBoard, Action::Right, &["<Right>"]),
    (Context::LiveBoard, Action::NextScene, &["<Tab>"]),
    (Context::Help, Action::Close, &["<Esc>"]),
    (Context::DevicePicker, Action::Up, &["k", "<Up>"]),
    (Context::DevicePicker, Action::Down, &["j", "<Down>"]),
    (Context::DevicePicker, Action::Confirm, &["<CR>"]),
    (Context::DevicePicker, Action::Close, &["<Esc>"]),
    (Context::Looper, Action::ToggleRecord, &["r"]),
    (Context::Looper, Action::TogglePlay, &["<Space>"]),
    (Context::Looper, Action::Clear, &["c"]),
    (Context::Looper, Action::Increase, &["<Right>", "+"]),
    (Context::Looper, Action::Decrease, &["<Left>", "-"]),
    (Context::Looper, Action::Up, &["<Up>", "k"]),
    (Context::Looper, Action::Down, &["<Down>", "j"]),
    (Context::Looper, Action::Close, &["<Esc>"]),
    (Context::Drum, Action::Up, &["k", "<Up>"]),
    (Context::Drum, Action::Down, &["j", "<Down>"]),
    (Context::Drum, Action::Increase, &["<Right>", "+"]),
    (Context::Drum, Action::Decrease, &["<Left>", "-"]),
    (Context::Drum, Action::TogglePlay, &["<Space>"]),
    (Context::Drum, Action::Close, &["<Esc>"]),
    (Context::Conflict, Action::Override, &["o", "O"]),
    (Context::Conflict, Action::Discard, &["n", "N"]),
    (Context::Conflict, Action::Rename, &["r", "R"]),
    (Context::Conflict, Action::Close, &["<Esc>"]),
];

/// YAML shape: `context → action → keys`. Names stay strings so that one
/// unknown name only costs a warning, not the whole file.
type RawBindings = BTreeMap<String, BTreeMap<String, Vec<String>>>;

type Bindings = BTreeMap<(Context, Action), Vec<Key>>;

/// Resolved key bindings.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyMap {
    lookup: HashMap<(Context, Key), Action>,
    bindings: Bindings,
    /// Problems found while loading the user file (the entries concerned fall back to defaults).
    pub warnings: Vec<String>,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::build(BTreeMap::new(), Vec::new())
    }
}

impl KeyMap {
    /// Parse `yaml` and merge it over the defaults. Never fails: on a YAML
    /// error the defaults are returned with a warning.
    pub fn from_yaml(yaml: &str) -> Self {
        if yaml.trim().is_empty() {
            return Self::default();
        }
        let raw: RawBindings = match serde_saphyr::from_str::<Option<RawBindings>>(yaml) {
            Ok(raw) => raw.unwrap_or_default(),
            Err(error) => {
                return Self::build(
                    BTreeMap::new(),
                    vec![format!("invalid YAML, using defaults: {error}")],
                );
            }
        };
        let mut warnings = Vec::new();
        let overrides = Self::parse_overrides(raw, &mut warnings);
        Self::build(overrides, warnings)
    }

    /// Validate the raw names and keys, keeping only usable overrides.
    fn parse_overrides(raw: RawBindings, warnings: &mut Vec<String>) -> Bindings {
        let known: BTreeSet<(Context, Action)> = DEFAULTS
            .iter()
            .map(|(context, action, _)| (*context, *action))
            .collect();
        let mut overrides = Bindings::new();
        for (context_name, actions) in raw {
            let Some(context) = parse_name::<Context>(&context_name) else {
                warnings.push(format!("unknown context '{context_name}'"));
                continue;
            };
            for (action_name, key_names) in actions {
                let Some(action) = parse_name::<Action>(&action_name) else {
                    warnings.push(format!("{context_name}: unknown action '{action_name}'"));
                    continue;
                };
                if !known.contains(&(context, action)) {
                    warnings.push(format!(
                        "{context_name}: action '{action_name}' does not exist in this context"
                    ));
                    continue;
                }
                let requested = key_names.len();
                let mut keys = Vec::new();
                for key_name in &key_names {
                    match key_name.parse::<Key>() {
                        Ok(key) if key.key != KeyCode::Null => keys.push(key),
                        Ok(_) => warnings.push(format!(
                            "{context_name}.{action_name}: empty key is not allowed"
                        )),
                        Err(error) => {
                            warnings.push(format!("{context_name}.{action_name}: {error}"));
                        }
                    }
                }
                // Every key was invalid: keep the default rather than silently unbinding.
                if requested > 0 && keys.is_empty() {
                    continue;
                }
                overrides.insert((context, action), keys);
            }
        }
        overrides
    }

    /// Merge `overrides` over the defaults and resolve key conflicts.
    ///
    /// A user binding beats a default one; between two user bindings the first
    /// (in context/action order) wins. A tab key already bound globally is dropped.
    fn build(overrides: Bindings, mut warnings: Vec<String>) -> Self {
        let mut bindings = default_bindings();
        let overridden: BTreeSet<(Context, Action)> = overrides.keys().copied().collect();
        bindings.extend(overrides);

        let mut lookup: HashMap<(Context, Key), Action> = HashMap::new();
        let mut resolved = Bindings::new();
        for user_pass in [true, false] {
            for (&(context, action), keys) in &bindings {
                if overridden.contains(&(context, action)) != user_pass {
                    continue;
                }
                let entry = resolved.entry((context, action)).or_default();
                for key in keys {
                    if entry.contains(key) {
                        continue;
                    }
                    if let Some(owner) = lookup.get(&(context, key.clone())) {
                        warnings.push(format!(
                            "{context:?}: key '{key}' is already bound to {owner:?}, ignored for {action:?}"
                        ));
                        continue;
                    }
                    lookup.insert((context, key.clone()), action);
                    entry.push(key.clone());
                }
            }
        }

        // Tabs only see keys that Global did not take.
        for ((context, action), keys) in &mut resolved {
            if !context.is_tab() {
                continue;
            }
            keys.retain(|key| {
                let shadowed = lookup.contains_key(&(Context::Global, key.clone()));
                if shadowed {
                    warnings.push(format!(
                        "{context:?}: key '{key}' for {action:?} is shadowed by a global binding"
                    ));
                    lookup.remove(&(*context, key.clone()));
                }
                !shadowed
            });
        }

        Self {
            lookup,
            bindings: resolved,
            warnings,
        }
    }

    /// Action bound to `key` in `context`, if any.
    pub fn action_for(&self, context: Context, key: &Key) -> Option<Action> {
        self.lookup.get(&(context, key.clone())).copied()
    }

    /// Action bound to the terminal key `event` in `context`, if any.
    pub fn action_for_event(&self, context: Context, event: KeyEvent) -> Option<Action> {
        self.action_for(context, &Key::from_event(event))
    }

    /// Keys bound to `action` in `context` (empty when unbound).
    pub fn keys(&self, context: Context, action: Action) -> &[Key] {
        self.bindings
            .get(&(context, action))
            .map_or(&[], Vec::as_slice)
    }

    /// All keys of `action`, human readable, joined with `/` (`-` when unbound).
    pub fn label(&self, context: Context, action: Action) -> String {
        let keys = self.keys(context, action);
        if keys.is_empty() {
            return "-".to_owned();
        }
        keys.iter().map(display_key).collect::<Vec<_>>().join("/")
    }

    /// First key of `action`, human readable (`-` when unbound).
    pub fn primary(&self, context: Context, action: Action) -> String {
        self.keys(context, action)
            .first()
            .map_or_else(|| "-".to_owned(), display_key)
    }

    /// Hint-bar text such as ` j/k:move  q:quit`. Each entry shows the first
    /// key of every listed action; entries with no bound key are left out.
    pub fn hints(&self, entries: &[(Context, &[Action], &str)]) -> String {
        let mut text = String::new();
        for (context, actions, description) in entries {
            let keys: Vec<String> = actions
                .iter()
                .filter(|action| !self.keys(*context, **action).is_empty())
                .map(|action| self.primary(*context, *action))
                .collect();
            if keys.is_empty() {
                continue;
            }
            text.push_str(&format!(" {}:{description} ", keys.join("/")));
        }
        text.trim_end().to_owned()
    }
}

impl Context {
    /// Context whose bindings apply while `overlay` is open.
    pub fn of_overlay(overlay: Overlay) -> Self {
        match overlay {
            Overlay::DevicePicker => Self::DevicePicker,
            Overlay::Looper => Self::Looper,
            Overlay::Drum => Self::Drum,
            Overlay::Conflict => Self::Conflict,
            Overlay::Help => Self::Help,
        }
    }
}

impl Key {
    /// Normalise a terminal event so it compares equal to a parsed config key:
    /// `SHIFT` is implied by an uppercase letter and meaningless on symbols.
    pub fn from_event(event: KeyEvent) -> Self {
        let mut modifiers = event.modifiers;
        if let KeyCode::Char(ch) = event.code {
            if ch.is_uppercase() {
                modifiers.insert(KeyModifiers::SHIFT);
            } else if !ch.is_alphabetic() {
                modifiers.remove(KeyModifiers::SHIFT);
            }
        }
        Self {
            key: event.code,
            modifiers,
        }
    }
}

/// Compact label for the UI: `Enter`, `↑`, `Ctrl+x`, `F1`, `Space`.
pub fn display_key(key: &Key) -> String {
    let mut prefix = String::new();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        prefix.push_str("Ctrl+");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        prefix.push_str("Alt+");
    }
    let shown_shift = key.modifiers.contains(KeyModifiers::SHIFT)
        && !matches!(key.key, KeyCode::Char(ch) if ch.is_alphabetic());
    if shown_shift {
        prefix.push_str("Shift+");
    }
    let name = match key.key {
        KeyCode::Char(' ') => "Space".to_owned(),
        KeyCode::Char(ch) => ch.to_string(),
        KeyCode::Enter => "Enter".to_owned(),
        KeyCode::Up => "↑".to_owned(),
        KeyCode::Down => "↓".to_owned(),
        KeyCode::Left => "←".to_owned(),
        KeyCode::Right => "→".to_owned(),
        KeyCode::Backspace => "Bksp".to_owned(),
        KeyCode::BackTab => "Shift+Tab".to_owned(),
        KeyCode::Delete => "Del".to_owned(),
        KeyCode::F(number) => format!("F{number}"),
        other => format!("{other}"),
    };
    format!("{prefix}{name}")
}

fn default_bindings() -> Bindings {
    DEFAULTS
        .iter()
        .map(|(context, action, keys)| {
            let parsed = keys
                .iter()
                .filter_map(|name| name.parse::<Key>().ok())
                .collect();
            ((*context, *action), parsed)
        })
        .collect()
}

/// Parse a snake_case enum name from the YAML file.
fn parse_name<T: for<'de> Deserialize<'de>>(name: &str) -> Option<T> {
    let deserializer: StrDeserializer<'_, serde::de::value::Error> = name.into_deserializer();
    T::deserialize(deserializer).ok()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn key(text: &str) -> Key {
        text.parse().expect("valid test key")
    }

    #[test]
    fn defaults_have_no_warnings_and_all_keys_parse() {
        let map = KeyMap::default();
        assert!(map.warnings.is_empty(), "{:?}", map.warnings);
        for (context, action, names) in DEFAULTS {
            assert_eq!(
                map.keys(*context, *action).len(),
                names.len(),
                "{context:?}/{action:?}"
            );
        }
    }

    #[test]
    fn example_file_equals_defaults() {
        let example = KeyMap::from_yaml(include_str!("../../keybindings.example.yaml"));
        assert!(example.warnings.is_empty(), "{:?}", example.warnings);
        assert_eq!(example, KeyMap::default());
    }

    #[test]
    fn default_lookup() {
        let map = KeyMap::default();
        assert_eq!(
            map.action_for(Context::Global, &key("q")),
            Some(Action::Quit)
        );
        assert_eq!(
            map.action_for(Context::Global, &key("D")),
            Some(Action::OpenDrum)
        );
        assert_eq!(
            map.action_for(Context::Presets, &key("<Up>")),
            Some(Action::Up)
        );
        assert_eq!(map.action_for(Context::Global, &key("x")), None);
    }

    #[test]
    fn override_replaces_only_the_listed_action() {
        let map = KeyMap::from_yaml("global:\n  quit: [\"x\"]\n");
        assert!(map.warnings.is_empty(), "{:?}", map.warnings);
        assert_eq!(
            map.action_for(Context::Global, &key("x")),
            Some(Action::Quit)
        );
        assert_eq!(map.action_for(Context::Global, &key("q")), None);
        assert_eq!(
            map.action_for(Context::Global, &key("?")),
            Some(Action::Help)
        );
    }

    #[test]
    fn empty_list_unbinds() {
        let map = KeyMap::from_yaml("global:\n  quit: []\n");
        assert_eq!(map.action_for(Context::Global, &key("q")), None);
        assert!(map.keys(Context::Global, Action::Quit).is_empty());
        assert_eq!(map.label(Context::Global, Action::Quit), "-");
    }

    #[rstest]
    #[case("")]
    #[case("# only a comment\n")]
    #[case("{}")]
    fn empty_documents_give_defaults(#[case] yaml: &str) {
        assert_eq!(KeyMap::from_yaml(yaml), KeyMap::default());
    }

    #[test]
    fn invalid_yaml_falls_back_to_defaults_with_warning() {
        let map = KeyMap::from_yaml("global: [unclosed");
        assert_eq!(map.warnings.len(), 1);
        assert_eq!(
            map.action_for(Context::Global, &key("q")),
            Some(Action::Quit)
        );
    }

    #[rstest]
    #[case("nope:\n  quit: [\"x\"]\n", "unknown context")]
    #[case("global:\n  fly: [\"x\"]\n", "unknown action")]
    #[case("global:\n  get_preset: [\"x\"]\n", "does not exist in this context")]
    #[case("global:\n  quit: [\"<Bogus>\"]\n", "Invalid key")]
    fn bad_entries_warn_and_keep_defaults(#[case] yaml: &str, #[case] expected: &str) {
        let map = KeyMap::from_yaml(yaml);
        assert!(
            map.warnings
                .iter()
                .any(|warning| warning.contains(expected)),
            "{:?}",
            map.warnings
        );
        assert_eq!(
            map.action_for(Context::Global, &key("q")),
            Some(Action::Quit)
        );
    }

    #[test]
    fn partially_invalid_list_keeps_valid_keys() {
        let map = KeyMap::from_yaml("global:\n  quit: [\"x\", \"<Bogus>\"]\n");
        assert_eq!(map.warnings.len(), 1);
        assert_eq!(
            map.action_for(Context::Global, &key("x")),
            Some(Action::Quit)
        );
        assert_eq!(map.action_for(Context::Global, &key("q")), None);
    }

    #[test]
    fn user_binding_beats_default_on_conflict() {
        // `q` is the default of quit: binding it to help steals it.
        let map = KeyMap::from_yaml("global:\n  help: [\"q\"]\n");
        assert_eq!(
            map.action_for(Context::Global, &key("q")),
            Some(Action::Help)
        );
        assert!(map.keys(Context::Global, Action::Quit).is_empty());
        assert!(!map.warnings.is_empty());
    }

    #[test]
    fn two_user_bindings_on_one_key_keep_the_first() {
        let map = KeyMap::from_yaml("global:\n  quit: [\"x\"]\n  help: [\"x\"]\n");
        // Actions are ordered Quit < Help.
        assert_eq!(
            map.action_for(Context::Global, &key("x")),
            Some(Action::Quit)
        );
        assert!(map.keys(Context::Global, Action::Help).is_empty());
        assert!(!map.warnings.is_empty());
    }

    #[test]
    fn tab_key_shadowed_by_global_is_dropped() {
        let map = KeyMap::from_yaml("presets:\n  get_preset: [\"q\"]\n");
        assert_eq!(map.action_for(Context::Presets, &key("q")), None);
        assert_eq!(
            map.action_for(Context::Global, &key("q")),
            Some(Action::Quit)
        );
        assert!(
            map.warnings
                .iter()
                .any(|warning| warning.contains("shadowed"))
        );
    }

    #[test]
    fn same_key_in_different_contexts_is_allowed() {
        let map = KeyMap::default();
        assert_eq!(
            map.action_for(Context::Looper, &key("r")),
            Some(Action::ToggleRecord)
        );
        assert_eq!(
            map.action_for(Context::Conflict, &key("r")),
            Some(Action::Rename)
        );
    }

    #[test]
    fn duplicate_keys_in_one_list_are_deduplicated() {
        let map = KeyMap::from_yaml("global:\n  quit: [\"x\", \"x\"]\n");
        assert_eq!(map.keys(Context::Global, Action::Quit).len(), 1);
        assert!(map.warnings.is_empty());
    }

    fn event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[rstest]
    #[case(KeyCode::Char('D'), KeyModifiers::SHIFT, "D")]
    #[case(KeyCode::Char('D'), KeyModifiers::NONE, "D")]
    #[case(KeyCode::Char('?'), KeyModifiers::SHIFT, "?")]
    #[case(KeyCode::Char('q'), KeyModifiers::NONE, "q")]
    #[case(KeyCode::Char('x'), KeyModifiers::CONTROL, "<C-x>")]
    #[case(KeyCode::F(1), KeyModifiers::NONE, "<F1>")]
    #[case(KeyCode::BackTab, KeyModifiers::SHIFT, "<S-Tab>")]
    fn event_normalisation_matches_config_syntax(
        #[case] code: KeyCode,
        #[case] modifiers: KeyModifiers,
        #[case] config: &str,
    ) {
        assert_eq!(Key::from_event(event(code, modifiers)), key(config));
    }

    #[test]
    fn action_for_event_handles_shifted_symbol() {
        let map = KeyMap::default();
        let shifted_question = event(KeyCode::Char('?'), KeyModifiers::SHIFT);
        assert_eq!(
            map.action_for_event(Context::Global, shifted_question),
            Some(Action::Help)
        );
    }

    #[rstest]
    #[case("<CR>", "Enter")]
    #[case("<Up>", "↑")]
    #[case("<Space>", "Space")]
    #[case("<F2>", "F2")]
    #[case("<C-x>", "Ctrl+x")]
    #[case("D", "D")]
    #[case("?", "?")]
    #[case("<Esc>", "Esc")]
    fn key_labels(#[case] config: &str, #[case] expected: &str) {
        assert_eq!(display_key(&key(config)), expected);
    }

    #[test]
    fn label_joins_all_keys_and_primary_takes_first() {
        let map = KeyMap::default();
        assert_eq!(map.label(Context::Presets, Action::Up), "k/↑");
        assert_eq!(map.primary(Context::Presets, Action::Up), "k");
    }

    #[test]
    fn hints_skip_unbound_actions() {
        let map = KeyMap::from_yaml("global:\n  help: []\n");
        let hints = map.hints(&[
            (Context::Global, &[Action::Quit], "quit"),
            (Context::Global, &[Action::Help], "help"),
            (Context::Presets, &[Action::Up, Action::Down], "move"),
        ]);
        assert_eq!(hints, " q:quit  k/j:move");
    }
}
