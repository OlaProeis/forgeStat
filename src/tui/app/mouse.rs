use super::{App, BorderType, DragState, Panel};
use std::time::{Duration, Instant};

/// Time window for double-click detection (300ms)
const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(300);
/// Maximum distance for double-click detection (3 pixels)
const DOUBLE_CLICK_DISTANCE: u16 = 3;

impl App {
    pub(super) fn handle_mouse_click(&mut self, column: u16, row: u16) {
        let point = ratatui::layout::Position { x: column, y: row };

        // Check for double-click
        let now = Instant::now();
        let is_double_click = self.last_click_time.map_or(false, |last_time| {
            let time_ok = now.duration_since(last_time) <= DOUBLE_CLICK_THRESHOLD;
            let pos_ok = self.last_click_pos.map_or(false, |(last_x, last_y)| {
                let dx = if column > last_x {
                    column - last_x
                } else {
                    last_x - column
                };
                let dy = if row > last_y {
                    row - last_y
                } else {
                    last_y - row
                };
                dx <= DOUBLE_CLICK_DISTANCE && dy <= DOUBLE_CLICK_DISTANCE
            });
            time_ok && pos_ok
        });

        // Update last click tracking
        self.last_click_time = Some(now);
        self.last_click_pos = Some((column, row));

        // Handle double-click -> zoom
        if is_double_click {
            // Find which panel was clicked and zoom it
            for (i, area) in self.panel_areas.iter().enumerate() {
                if let Some(area) = area {
                    if area.contains(point) {
                        self.selected_panel = Panel::VALUES[i];
                        self.toggle_zoom();
                        return;
                    }
                }
            }
            return;
        }

        // Single click - just select the panel
        for (i, area) in self.panel_areas.iter().enumerate() {
            if let Some(area) = area {
                if area.contains(point) {
                    self.selected_panel = Panel::VALUES[i];
                    break;
                }
            }
        }
    }

    /// Check if a point is on a vertical border and return the border index if so
    pub(super) fn get_vertical_border_at(&self, column: u16, row: u16) -> Option<usize> {
        let point = ratatui::layout::Position { x: column, y: row };
        for (i, border) in self.vertical_borders.iter().enumerate() {
            if let Some(border) = border {
                if border.contains(point) {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Check if a point is on a horizontal border and return the border index if so
    pub(super) fn get_horizontal_border_at(&self, column: u16, row: u16) -> Option<usize> {
        let point = ratatui::layout::Position { x: column, y: row };
        for (i, border) in self.horizontal_borders.iter().enumerate() {
            if let Some(border) = border {
                if border.contains(point) {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Start dragging a border for resize
    pub(super) fn start_border_drag(
        &mut self,
        border_type: BorderType,
        border_index: usize,
        column: u16,
        row: u16,
    ) {
        self.drag_state = Some(DragState {
            border_index,
            border_type,
            last_mouse_pos: (column, row),
        });
        log::info!("Started dragging {:?} border {}", border_type, border_index);
    }

    /// Handle mouse drag during resize operation
    pub(super) fn handle_drag(&mut self, column: u16, row: u16) {
        let Some(drag) = self.drag_state else {
            return;
        };

        let delta_x = column as i32 - drag.last_mouse_pos.0 as i32;
        let delta_y = row as i32 - drag.last_mouse_pos.1 as i32;

        match drag.border_type {
            BorderType::Vertical => {
                self.resize_vertical_border(drag.border_index, delta_x);
            }
            BorderType::Horizontal => {
                self.resize_horizontal_border(drag.border_index, delta_y);
            }
        }

        self.drag_state = Some(DragState {
            border_index: drag.border_index,
            border_type: drag.border_type,
            last_mouse_pos: (column, row),
        });
    }

    /// Resize columns by moving a vertical border
    /// border_index: 0=row1 border, 1-2=row2 borders, 3=row3 border
    fn resize_vertical_border(&mut self, border_index: usize, delta_x: i32) {
        const MIN_WIDTH_PCT: u16 = 20;

        let (columns, _row_idx): (&mut Vec<crate::core::config::PanelLayout>, usize) =
            match border_index {
                0 => (&mut self.layout_config.row1_columns, 0),
                1 => (&mut self.layout_config.row2_columns, 1),
                2 => (&mut self.layout_config.row2_columns, 1),
                3 => (&mut self.layout_config.row3_columns, 2),
                _ => return,
            };

        if columns.len() < 2 {
            return;
        }

        let delta_pct = delta_x as i32;

        let (left_idx, right_idx) = match border_index {
            0 => (0, 1),
            1 => (0, 1),
            2 => (1, 2),
            3 => (0, 1),
            _ => return,
        };

        let current_left = columns[left_idx].width_pct as i32;
        let current_right = columns[right_idx].width_pct as i32;

        let new_left =
            (current_left + delta_pct).clamp(MIN_WIDTH_PCT as i32, (100 - MIN_WIDTH_PCT) as i32);
        let new_right =
            (current_right - delta_pct).clamp(MIN_WIDTH_PCT as i32, (100 - MIN_WIDTH_PCT) as i32);

        let total = new_left + new_right;
        let normalized_left = (new_left * 100 / total) as u16;
        let normalized_right = (100 - normalized_left) as u16;

        columns[left_idx].width_pct = normalized_left;
        columns[right_idx].width_pct = normalized_right;
    }

    /// Resize rows by moving a horizontal border
    /// border_index: 0=between row1/row2, 1=between row2/row3
    fn resize_horizontal_border(&mut self, border_index: usize, delta_y: i32) {
        const MIN_HEIGHT_PCT: u16 = 20;

        let (row1_ref, row2_ref): (
            &mut crate::core::config::PanelLayout,
            &mut crate::core::config::PanelLayout,
        ) = match border_index {
            0 => (&mut self.layout_config.row1, &mut self.layout_config.row2),
            1 => (&mut self.layout_config.row2, &mut self.layout_config.row3),
            _ => return,
        };

        let delta_pct = delta_y as i32;

        let current_top = row1_ref.height_pct as i32;
        let current_bottom = row2_ref.height_pct as i32;

        let (new_top, new_bottom) = if border_index == 0 {
            let new_top = (current_top + delta_pct)
                .clamp(MIN_HEIGHT_PCT as i32, (100 - MIN_HEIGHT_PCT) as i32);
            let new_bottom = (current_bottom - delta_pct)
                .clamp(MIN_HEIGHT_PCT as i32, (100 - MIN_HEIGHT_PCT) as i32);
            (new_top, new_bottom)
        } else {
            let new_mid = (current_top + delta_pct)
                .clamp(MIN_HEIGHT_PCT as i32, (100 - MIN_HEIGHT_PCT) as i32);
            let new_bottom = (current_bottom - delta_pct)
                .clamp(MIN_HEIGHT_PCT as i32, (100 - MIN_HEIGHT_PCT) as i32);
            (new_mid, new_bottom)
        };

        let r1 = if border_index == 0 {
            new_top
        } else {
            self.layout_config.row1.height_pct as i32
        };
        let r2 = if border_index == 0 {
            new_bottom
        } else {
            new_top
        };
        let r3 = if border_index == 0 {
            self.layout_config.row3.height_pct as i32
        } else {
            new_bottom
        };

        let total = r1 + r2 + r3;
        let normalized_r1 = (r1 * 100 / total) as u16;
        let normalized_r2 = (r2 * 100 / total) as u16;
        let normalized_r3 = 100u16.saturating_sub(normalized_r1 + normalized_r2);

        self.layout_config.row1.height_pct = normalized_r1;
        self.layout_config.row2.height_pct = normalized_r2;
        self.layout_config.row3.height_pct = normalized_r3;
    }

    /// End the current drag operation and save the layout
    pub(super) fn end_drag(&mut self) {
        if self.drag_state.is_some() {
            self.drag_state = None;
            if let Err(e) = self.layout_config.save() {
                log::warn!("Failed to save layout config after resize: {}", e);
            } else {
                log::info!("Saved layout config after resize");
            }
        }
    }
}
