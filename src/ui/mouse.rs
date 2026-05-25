//! Mouse event handling — `impl App` block for all `handle_mouse_*` methods.
//!
//! Hit-testing recomputes the same layout geometry as the render pass so no
//! area tracking is needed in application state.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Position, Rect};

use super::app::App;
use super::state::{DRUM_PATTERNS, Overlay, Tab};

/// Return the centered popup `Rect` used by overlay renderers.
/// Mirrors the `centered_rect` helper in each overlay file.
fn popup_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

impl App {
    pub fn handle_mouse(&mut self, mouse: MouseEvent, frame_area: Rect) {
        let col = mouse.column;
        let row = mouse.row;
        let pos = Position { x: col, y: row };

        // Recompute the same top-level layout used in render().
        let [tab_row, content, _status] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(frame_area);

        // Overlays absorb all mouse events.
        if let Some(overlay) = self.active_overlay {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    self.handle_mouse_overlay_click(overlay, col, row, frame_area);
                }
                MouseEventKind::ScrollUp => self.handle_mouse_overlay_scroll(overlay, -1),
                MouseEventKind::ScrollDown => self.handle_mouse_overlay_scroll(overlay, 1),
                _ => {}
            }
            return;
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if tab_row.contains(pos) {
                    self.handle_mouse_tab_click(col, tab_row);
                } else if content.contains(pos) {
                    self.handle_mouse_content_click(col, row, content);
                }
            }
            MouseEventKind::ScrollUp if content.contains(pos) => {
                self.handle_mouse_content_scroll(-1);
            }
            MouseEventKind::ScrollDown if content.contains(pos) => {
                self.handle_mouse_content_scroll(1);
            }
            _ => {}
        }
    }

    fn handle_mouse_tab_click(&mut self, col: u16, tab_row: Rect) {
        // Mirror the right_width computation in render_tab_bar.
        let device_label_len = match &self.device_info {
            Some(info) => format!(" {} fw{}", info.model, info.fw_version).len(),
            None => " offline".len(),
        };
        let right_width = (2 + device_label_len) as u16;
        let tabs_width = tab_row.width.saturating_sub(right_width);

        if col < tab_row.x || col >= tab_row.x + tabs_width {
            return; // clicked the info area on the right
        }
        let offset = col - tab_row.x;

        // Tabs: "[F1] Presets"(12) + " "(1) + "[F2] Live Board"(15) + " "(1) + "[F3] About"(10)
        if offset < 13 {
            self.active_tab = Tab::Presets;
        } else if offset < 29 {
            self.active_tab = Tab::LiveBoard;
        } else {
            self.active_tab = Tab::About;
        }
    }

    fn handle_mouse_content_click(&mut self, col: u16, row: u16, content: Rect) {
        let pos = Position { x: col, y: row };
        match self.active_tab {
            Tab::Presets => {
                let [list_area, chain_area] =
                    Layout::vertical([Constraint::Percentage(65), Constraint::Percentage(35)])
                        .areas(content);

                if list_area.contains(pos) {
                    self.handle_mouse_preset_list_click(row, list_area);
                } else if chain_area.contains(pos) {
                    self.handle_mouse_chain_click(col, chain_area);
                }
            }
            Tab::LiveBoard => {
                let [scene_bar, grid_area] =
                    Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(content);

                if scene_bar.contains(pos) {
                    // Click scene bar → cycle scene.
                    self.active_scene = (self.active_scene + 1) % 3;
                } else if grid_area.contains(pos) && grid_area.width > 0 {
                    // Click a slot column in the live board grid.
                    let slot_idx = ((col - grid_area.x) as usize * 11) / grid_area.width as usize;
                    self.chain_slot_focus = slot_idx.min(10);
                }
            }
            Tab::About => {}
        }
    }

    fn handle_mouse_content_scroll(&mut self, delta: i32) {
        if self.active_tab == Tab::Presets && !self.chain_panel_active {
            if delta < 0 {
                self.preset_selected = self.preset_selected.saturating_sub(1);
            } else {
                let max = self.preset_count().saturating_sub(1);
                self.preset_selected = (self.preset_selected + 1).min(max);
            }
        }
    }

    fn handle_mouse_preset_list_click(&mut self, row: u16, list_area: Rect) {
        // Inner area starts after the top border (y+1) and the header row (+1 more).
        let first_data_y = list_area.y + 2;
        if row < first_data_y {
            return;
        }
        let clicked_idx = (row - first_data_y) as usize;
        if clicked_idx < self.library.presets.len() {
            self.preset_selected = clicked_idx;
            self.chain_panel_active = false;
        }
    }

    fn handle_mouse_chain_click(&mut self, col: u16, chain_area: Rect) {
        // Inner x starts after the left border.
        let inner_x = chain_area.x + 1;
        if col < inner_x {
            return;
        }
        // Chain line layout: [WAH]──[CMP]──... Each slot occupies 7 chars.
        // Slot n starts at inner_x + n*7.
        let slot_idx = ((col - inner_x) / 7) as usize;
        if slot_idx <= 10 {
            self.chain_slot_focus = slot_idx;
            self.chain_panel_active = true;
        }
    }

    fn handle_mouse_overlay_click(
        &mut self,
        overlay: Overlay,
        col: u16,
        row: u16,
        frame_area: Rect,
    ) {
        match overlay {
            Overlay::DevicePicker => {
                // overlay_device: WIDTH=44, HEIGHT=12; items start at popup.y+2 (border+blank).
                let popup = popup_rect(44, 12, frame_area);
                let first_item_y = popup.y + 2;
                let last_y = popup.y + popup.height - 1;
                if row < first_item_y || row >= last_y {
                    return;
                }
                let clicked = (row - first_item_y) as usize;
                if clicked < self.device_picker.devices.len() {
                    self.device_picker.selected = clicked;
                }
            }
            Overlay::Drum => {
                // overlay_drum: WIDTH=44, HEIGHT=16; VISIBLE=6 pattern rows from popup.y+1.
                let popup = popup_rect(44, 16, frame_area);
                let first_item_y = popup.y + 1;
                let visible = 6usize;
                if row < first_item_y || row >= first_item_y + visible as u16 {
                    return;
                }
                let clicked = (row - first_item_y) as usize;
                let start = self
                    .drum
                    .pattern_index
                    .saturating_sub(visible / 2)
                    .min(DRUM_PATTERNS.len().saturating_sub(visible));
                let item = start + clicked;
                if item < DRUM_PATTERNS.len() {
                    self.drum.pattern_index = item;
                }
            }
            Overlay::Conflict => {
                // Bottom border row contains [O]verride / [N]o / [R]ename+timestamp.
                // Divide the popup width into three equal click zones.
                let popup = popup_rect(52, 12, frame_area);
                if row == popup.y + popup.height - 1 {
                    let third = popup.width / 3;
                    if col < popup.x + third {
                        if let Some(conflict) = self.conflict.take() {
                            self.library.presets[conflict.existing_idx] = conflict.incoming;
                            let _ = self.storage.save_library(&self.library);
                            self.set_status("Preset overridden");
                        }
                        self.active_overlay = None;
                    } else if col < popup.x + 2 * third {
                        self.conflict = None;
                        self.active_overlay = None;
                        self.set_status("Pull cancelled");
                    } else {
                        if let Some(mut conflict) = self.conflict.take() {
                            let ts = conflict.incoming.modified.format("%Y%m%d-%H%M%S");
                            conflict.incoming.name = format!("{} ({})", conflict.incoming.name, ts);
                            conflict.incoming.slot = None;
                            self.library.presets.push(conflict.incoming);
                            let _ = self.storage.save_library(&self.library);
                            self.set_status("Preset saved with timestamp");
                        }
                        self.active_overlay = None;
                    }
                }
            }
            Overlay::Looper => {} // Transport buttons: keyboard preferred.
        }
    }

    fn handle_mouse_overlay_scroll(&mut self, overlay: Overlay, delta: i32) {
        match overlay {
            Overlay::DevicePicker => {
                if delta < 0 {
                    self.device_picker.selected = self.device_picker.selected.saturating_sub(1);
                } else {
                    let max = self.device_picker.devices.len().saturating_sub(1);
                    self.device_picker.selected = (self.device_picker.selected + 1).min(max);
                }
            }
            Overlay::Drum => {
                if delta < 0 {
                    self.drum.pattern_index = self.drum.pattern_index.saturating_sub(1);
                } else {
                    self.drum.pattern_index =
                        (self.drum.pattern_index + 1).min(DRUM_PATTERNS.len() - 1);
                }
            }
            Overlay::Looper => {
                // Scroll adjusts volume in the looper overlay.
                if delta < 0 {
                    self.looper.volume = (self.looper.volume + 1).min(127);
                } else {
                    self.looper.volume = self.looper.volume.saturating_sub(1);
                }
            }
            Overlay::Conflict => {}
        }
    }
}
