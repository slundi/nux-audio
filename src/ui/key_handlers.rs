//! Key-event handlers: translate a key into an [`Action`] through the
//! configurable [`super::keymap::KeyMap`], then apply it to [`App`].
//!
//! Lookup order: an open overlay absorbs every key (its own context only);
//! otherwise the `Global` context is tried, then the active tab's context.
//! `Ctrl+C` quits unconditionally so a bad configuration cannot lock the user in.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::action::{Action, Context, GlobalAction};
use super::app::App;
use super::state::{DRUM_PATTERNS, Overlay, Tab};

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.handle_action(GlobalAction::Quit);
            return;
        }

        // Overlay absorbs all keys when active.
        if let Some(overlay) = self.active_overlay {
            self.handle_overlay_key(overlay, key);
            return;
        }

        if let Some(action) = self.keymap.action_for_event(Context::Global, key) {
            self.handle_global_key(action);
            return;
        }

        match self.active_tab {
            Tab::Presets => self.handle_presets_key(key),
            Tab::LiveBoard => self.handle_liveboard_key(key),
            Tab::About => {}
        }
    }

    fn handle_global_key(&mut self, action: Action) {
        match action {
            Action::Quit => self.handle_action(GlobalAction::Quit),
            Action::Help => self.handle_action(GlobalAction::Help),
            Action::ChangeDevice => self.handle_action(GlobalAction::ChangeDevice),
            Action::TabPresets => self.active_tab = Tab::Presets,
            Action::TabLiveBoard => self.active_tab = Tab::LiveBoard,
            Action::TabAbout => self.active_tab = Tab::About,
            Action::OpenLooper => self.open_overlay(Overlay::Looper),
            Action::OpenDrum => self.open_overlay(Overlay::Drum),
            _ => {}
        }
    }

    fn handle_overlay_key(&mut self, overlay: Overlay, key: KeyEvent) {
        let context = Context::of_overlay(overlay);
        let Some(action) = self.keymap.action_for_event(context, key) else {
            return;
        };
        if action == Action::Close {
            self.active_overlay = None;
            return;
        }
        match overlay {
            Overlay::DevicePicker => self.apply_device_picker_action(action),
            Overlay::Looper => self.apply_looper_action(action),
            Overlay::Drum => self.apply_drum_action(action),
            Overlay::Conflict => self.apply_conflict_action(action),
            Overlay::Help => {}
        }
    }

    fn apply_device_picker_action(&mut self, action: Action) {
        match action {
            Action::Up => {
                self.device_picker.selected = self.device_picker.selected.saturating_sub(1);
            }
            Action::Down => {
                let max = self.device_picker.devices.len().saturating_sub(1);
                self.device_picker.selected = (self.device_picker.selected + 1).min(max);
            }
            Action::Confirm => {
                if let Some(model) = self
                    .device_picker
                    .devices
                    .get(self.device_picker.selected)
                    .cloned()
                {
                    self.active_overlay = None;
                    self.connect_to(model);
                }
            }
            _ => {}
        }
    }

    fn apply_looper_action(&mut self, action: Action) {
        match action {
            Action::ToggleRecord => {
                self.looper.is_recording = !self.looper.is_recording;
                if self.looper.is_recording {
                    self.looper.is_playing = false;
                }
            }
            Action::TogglePlay => {
                self.looper.is_playing = !self.looper.is_playing;
                if self.looper.is_playing {
                    self.looper.is_recording = false;
                }
            }
            Action::Clear => {
                self.looper.is_playing = false;
                self.looper.is_recording = false;
            }
            Action::Increase => self.looper.bpm = (self.looper.bpm + 1).min(240),
            Action::Decrease => self.looper.bpm = self.looper.bpm.saturating_sub(1).max(20),
            Action::Up => self.looper.volume = (self.looper.volume + 1).min(127),
            Action::Down => self.looper.volume = self.looper.volume.saturating_sub(1),
            _ => {}
        }
    }

    fn apply_drum_action(&mut self, action: Action) {
        match action {
            Action::Up => {
                self.drum.pattern_index = self.drum.pattern_index.saturating_sub(1);
            }
            Action::Down => {
                self.drum.pattern_index =
                    (self.drum.pattern_index + 1).min(DRUM_PATTERNS.len() - 1);
            }
            Action::Increase => self.drum.bpm = (self.drum.bpm + 1).min(240),
            Action::Decrease => self.drum.bpm = self.drum.bpm.saturating_sub(1).max(20),
            Action::TogglePlay => self.drum.is_playing = !self.drum.is_playing,
            _ => {}
        }
    }

    fn handle_presets_key(&mut self, key: KeyEvent) {
        let Some(action) = self.keymap.action_for_event(Context::Presets, key) else {
            return;
        };
        let chain_active = self.chain_panel_active;
        match action {
            Action::Close => self.chain_panel_active = false,
            Action::Confirm => self.chain_panel_active = !chain_active,
            Action::NextScene => self.active_scene = (self.active_scene + 1) % 3,
            Action::GetPreset if !chain_active => self.handle_action(GlobalAction::GetPreset),
            Action::SendPreset if !chain_active => self.handle_action(GlobalAction::SendPreset),
            Action::Up if !chain_active => {
                self.preset_selected = self.preset_selected.saturating_sub(1);
            }
            Action::Down if !chain_active => {
                let max = self.preset_count().saturating_sub(1);
                self.preset_selected = (self.preset_selected + 1).min(max);
            }
            Action::Left if chain_active => self.focus_previous_slot(),
            Action::Right if chain_active => self.focus_next_slot(),
            _ => {}
        }
    }

    fn handle_liveboard_key(&mut self, key: KeyEvent) {
        let Some(action) = self.keymap.action_for_event(Context::LiveBoard, key) else {
            return;
        };
        match action {
            Action::NextScene => self.active_scene = (self.active_scene + 1) % 3,
            Action::Left => self.focus_previous_slot(),
            Action::Right => self.focus_next_slot(),
            _ => {}
        }
    }

    fn focus_previous_slot(&mut self) {
        self.chain_slot_focus = self.chain_slot_focus.saturating_sub(1);
    }

    fn focus_next_slot(&mut self) {
        self.chain_slot_focus = (self.chain_slot_focus + 1).min(10);
    }

    fn apply_conflict_action(&mut self, action: Action) {
        match action {
            // Override: replace the existing entry with the incoming one.
            Action::Override => {
                if let Some(conflict) = self.conflict.take() {
                    self.library.presets[conflict.existing_idx] = conflict.incoming;
                    let _ = self.storage.save_library(&self.library);
                    self.set_status("Preset overridden");
                }
                self.active_overlay = None;
            }
            // No: discard the incoming preset, keep the existing one.
            Action::Discard => {
                self.conflict = None;
                self.active_overlay = None;
                self.set_status("Pull cancelled");
            }
            // Rename: save the incoming as a new local-only entry with a timestamp suffix.
            Action::Rename => {
                if let Some(mut conflict) = self.conflict.take() {
                    let ts = conflict.incoming.modified.format("%Y%m%d-%H%M%S");
                    conflict.incoming.name = format!("{} ({})", conflict.incoming.name, ts);
                    conflict.incoming.slot = None; // local copy — not tied to a slot
                    self.library.presets.push(conflict.incoming);
                    let _ = self.storage.save_library(&self.library);
                    self.set_status("Preset saved with timestamp");
                }
                self.active_overlay = None;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use rstest::rstest;

    use super::*;
    use crate::ui::keymap::KeyMap;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn char_key(ch: char) -> KeyEvent {
        press(KeyCode::Char(ch))
    }

    fn app_with(yaml: &str) -> App {
        let mut app = App::new();
        app.keymap = KeyMap::from_yaml(yaml);
        app
    }

    #[test]
    fn ctrl_c_quits_even_when_quit_is_unbound() {
        let mut app = app_with("global:\n  quit: []\n");
        app.handle_key(char_key('q'));
        assert!(!app.should_quit);
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn ctrl_c_quits_from_an_overlay() {
        let mut app = app_with("");
        app.active_overlay = Some(Overlay::Help);
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn rebound_quit_key_works_and_old_key_does_not() {
        let mut app = app_with("global:\n  quit: [\"x\"]\n");
        app.handle_key(char_key('q'));
        assert!(!app.should_quit);
        app.handle_key(char_key('x'));
        assert!(app.should_quit);
    }

    #[test]
    fn shifted_letter_opens_drum_overlay() {
        let mut app = app_with("");
        app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
        assert_eq!(app.active_overlay, Some(Overlay::Drum));
    }

    #[test]
    fn global_modifier_binding() {
        let mut app = app_with("global:\n  open_looper: [\"<C-l>\"]\n");
        app.handle_key(char_key('l'));
        assert_eq!(app.active_overlay, None);
        app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL));
        assert_eq!(app.active_overlay, Some(Overlay::Looper));
    }

    #[test]
    fn overlay_close_key_is_configurable() {
        let mut app = app_with("help:\n  close: [\"x\"]\n");
        app.active_overlay = Some(Overlay::Help);
        app.handle_key(press(KeyCode::Esc));
        assert_eq!(app.active_overlay, Some(Overlay::Help));
        app.handle_key(char_key('x'));
        assert_eq!(app.active_overlay, None);
    }

    #[test]
    fn overlay_absorbs_global_keys() {
        let mut app = app_with("");
        app.active_overlay = Some(Overlay::Help);
        app.handle_key(char_key('q'));
        assert!(!app.should_quit);
    }

    #[test]
    fn presets_navigation_with_default_and_extra_keys() {
        let mut app = app_with("presets:\n  down: [\"n\", \"<Down>\"]\n");
        app.library.presets = vec![];
        app.preset_selected = 0;
        // No presets: selection is clamped at 0.
        app.handle_key(char_key('n'));
        assert_eq!(app.preset_selected, 0);
    }

    #[test]
    fn scene_cycles_on_both_tabs() {
        let mut app = app_with("");
        app.handle_key(press(KeyCode::Tab));
        assert_eq!(app.active_scene, 1);
        app.active_tab = Tab::LiveBoard;
        app.handle_key(press(KeyCode::Tab));
        assert_eq!(app.active_scene, 2);
        app.handle_key(press(KeyCode::Tab));
        assert_eq!(app.active_scene, 0);
    }

    #[test]
    fn chain_panel_focus_and_slot_navigation() {
        let mut app = app_with("");
        app.handle_key(press(KeyCode::Enter));
        assert!(app.chain_panel_active);
        app.handle_key(press(KeyCode::Right));
        app.handle_key(press(KeyCode::Right));
        app.handle_key(char_key('h'));
        assert_eq!(app.chain_slot_focus, 1);
        app.handle_key(press(KeyCode::Esc));
        assert!(!app.chain_panel_active);
        // Slot navigation is ignored while the panel is not focused.
        app.handle_key(press(KeyCode::Right));
        assert_eq!(app.chain_slot_focus, 1);
    }

    #[test]
    fn chain_slot_focus_is_clamped() {
        let mut app = app_with("");
        app.active_tab = Tab::LiveBoard;
        for _ in 0..20 {
            app.handle_key(press(KeyCode::Right));
        }
        assert_eq!(app.chain_slot_focus, 10);
        for _ in 0..20 {
            app.handle_key(press(KeyCode::Left));
        }
        assert_eq!(app.chain_slot_focus, 0);
    }

    #[test]
    fn get_and_send_preset_ignored_while_chain_panel_focused() {
        let mut app = app_with("");
        app.chain_panel_active = true;
        app.handle_key(char_key('g'));
        app.handle_key(char_key('s'));
        assert!(app.status_message.is_none());
        app.chain_panel_active = false;
        app.handle_key(char_key('g'));
        assert!(app.status_message.is_some(), "offline pull sets a status");
    }

    #[test]
    fn looper_actions() {
        let mut app = app_with("");
        app.active_overlay = Some(Overlay::Looper);
        app.handle_key(char_key('r'));
        assert!(app.looper.is_recording);
        app.handle_key(char_key(' '));
        assert!(app.looper.is_playing && !app.looper.is_recording);
        app.handle_key(char_key('r'));
        assert!(app.looper.is_recording && !app.looper.is_playing);
        app.handle_key(char_key('c'));
        assert!(!app.looper.is_recording && !app.looper.is_playing);

        let bpm = app.looper.bpm;
        app.handle_key(char_key('+'));
        assert_eq!(app.looper.bpm, bpm + 1);
        app.handle_key(char_key('-'));
        assert_eq!(app.looper.bpm, bpm);
        let volume = app.looper.volume;
        app.handle_key(press(KeyCode::Up));
        assert_eq!(app.looper.volume, volume + 1);
        app.handle_key(char_key('j'));
        assert_eq!(app.looper.volume, volume);
    }

    #[test]
    fn looper_bpm_is_clamped() {
        let mut app = app_with("");
        app.active_overlay = Some(Overlay::Looper);
        for _ in 0..300 {
            app.handle_key(press(KeyCode::Right));
        }
        assert_eq!(app.looper.bpm, 240);
        for _ in 0..300 {
            app.handle_key(press(KeyCode::Left));
        }
        assert_eq!(app.looper.bpm, 20);
    }

    #[test]
    fn drum_actions() {
        let mut app = app_with("");
        app.active_overlay = Some(Overlay::Drum);
        app.handle_key(char_key(' '));
        assert!(app.drum.is_playing);
        app.handle_key(char_key('j'));
        assert_eq!(app.drum.pattern_index, 1);
        app.handle_key(char_key('k'));
        assert_eq!(app.drum.pattern_index, 0);
        let bpm = app.drum.bpm;
        app.handle_key(press(KeyCode::Right));
        assert_eq!(app.drum.bpm, bpm + 1);
    }

    #[test]
    fn device_picker_navigation_is_clamped() {
        let mut app = app_with("");
        app.active_overlay = Some(Overlay::DevicePicker);
        app.device_picker.devices = vec!["A".to_owned(), "B".to_owned()];
        app.device_picker.selected = 0;
        for _ in 0..5 {
            app.handle_key(char_key('j'));
        }
        assert_eq!(app.device_picker.selected, 1);
        for _ in 0..5 {
            app.handle_key(press(KeyCode::Up));
        }
        assert_eq!(app.device_picker.selected, 0);
    }

    #[rstest]
    #[case('n')]
    #[case('N')]
    fn conflict_discard_keys(#[case] ch: char) {
        let mut app = app_with("");
        app.active_overlay = Some(Overlay::Conflict);
        let modifiers = if ch.is_uppercase() {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        };
        app.handle_key(KeyEvent::new(KeyCode::Char(ch), modifiers));
        assert_eq!(app.active_overlay, None);
        assert!(app.status_message.is_some());
    }

    #[test]
    fn tab_keys_are_configurable() {
        let mut app = app_with("global:\n  tab_about: [\"3\"]\n");
        app.handle_key(char_key('3'));
        assert_eq!(app.active_tab, Tab::About);
        // The old default no longer switches tab.
        app.active_tab = Tab::Presets;
        app.handle_key(press(KeyCode::F(3)));
        assert_eq!(app.active_tab, Tab::Presets);
    }
}
