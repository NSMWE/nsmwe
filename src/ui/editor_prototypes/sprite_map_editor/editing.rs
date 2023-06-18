use egui::{pos2, vec2, Pos2};
use smwe_math::space::{OnCanvas, OnGrid, OnScreen};
use smwe_widgets::vram_view::VramSelectionMode;

use super::UiSpriteMapEditor;
use crate::ui::editing_mode::{Drag, Selection, SnapToGrid};

impl UiSpriteMapEditor {
    pub(super) fn handle_edition_insert(&mut self, grid_cell_pos: OnCanvas<Pos2>) {
        if self.last_inserted_tile != grid_cell_pos {
            match self.vram_selection_mode {
                VramSelectionMode::SingleTile => self.add_selected_tile_at(grid_cell_pos),
                VramSelectionMode::TwoByTwoTiles => {
                    let current_selection = self.selected_vram_tile;
                    for offset in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                        self.selected_vram_tile.0 = current_selection.0 + offset.0;
                        self.selected_vram_tile.1 = current_selection.1 + offset.1;
                        let offset = OnGrid(vec2(offset.0 as f32, offset.1 as f32)).to_canvas(self.tile_size_px);
                        let pos = OnCanvas(grid_cell_pos.0 + offset.0);
                        self.add_selected_tile_at(pos);
                    }
                    self.selected_vram_tile = current_selection;
                }
            }
            self.last_inserted_tile = grid_cell_pos;
        }
        self.unselect_all_tiles();
    }

    pub(super) fn handle_selection_plot(
        &mut self, selection: Selection, clear_previous_selection: bool, canvas_top_left_pos: OnScreen<Pos2>,
    ) {
        match selection {
            Selection::Click(Some(origin)) => {
                let pos = origin.0 - canvas_top_left_pos.0;
                self.select_tile_at(OnScreen(pos.to_pos2()), clear_previous_selection);
            }
            Selection::Drag(Some(selection_rect)) => {
                self.select_tiles_in(
                    OnScreen(selection_rect.0.translate(-canvas_top_left_pos.0.to_vec2())),
                    clear_previous_selection,
                );
            }
            _ => {}
        }
    }

    pub(super) fn handle_edition_drop_moved(
        &mut self, drag_data: Drag, snap_to_grid: bool, canvas_top_left_pos: OnScreen<Pos2>,
    ) {
        if !self.any_tile_contains_pointer(drag_data.from, canvas_top_left_pos) {
            return;
        }

        self.move_selected_tiles_by(
            drag_data.delta().to_canvas(self.pixels_per_point, self.zoom),
            snap_to_grid.then(|| {
                let pointer_in_canvas = drag_data.from.relative_to(canvas_top_left_pos);
                let hovered_tile_exact_offset = pointer_in_canvas
                    .to_grid(self.pixels_per_point, self.zoom, self.tile_size_px)
                    .clamp(OnGrid(pos2(0., 0.)), self.grid_size.to_pos2())
                    .to_screen(self.pixels_per_point, self.zoom, self.tile_size_px);
                let cell_origin =
                    OnScreen(pointer_in_canvas.relative_to(hovered_tile_exact_offset).0.to_vec2() / self.zoom);
                SnapToGrid { cell_origin }
            }),
        );
    }

    pub(super) fn handle_edition_dragging(
        &mut self, mut drag_data: Drag, snap_to_grid: bool, canvas_top_left_pos: OnScreen<Pos2>,
    ) {
        if !self.any_tile_contains_pointer(drag_data.from, canvas_top_left_pos) {
            return;
        }

        if snap_to_grid {
            let sel_bounds = self.selection_bounds.expect("unset even though some tiles are selected");

            let bounds_min_grid = OnCanvas(sel_bounds.0.min).to_grid(self.tile_size_px);
            let started_tile = drag_data.from.relative_to(canvas_top_left_pos).to_grid(
                self.pixels_per_point,
                self.zoom,
                self.tile_size_px,
            );
            let hovered_tile = drag_data.to.relative_to(canvas_top_left_pos).to_grid(
                self.pixels_per_point,
                self.zoom,
                self.tile_size_px,
            );

            let bounds_at_grid_exact_offset =
                bounds_min_grid.to_screen(self.pixels_per_point, self.zoom, self.tile_size_px).to_vec2().0;
            let started_tile_exact_offset =
                started_tile.to_screen(self.pixels_per_point, self.zoom, self.tile_size_px).to_vec2().0;
            let hovered_tile_exact_offset =
                hovered_tile.to_screen(self.pixels_per_point, self.zoom, self.tile_size_px).to_vec2().0;

            let bounds_screen = OnCanvas(sel_bounds.0.min).to_screen(self.pixels_per_point, self.zoom);
            let bounds_offset = bounds_screen.to_vec2().0 - bounds_at_grid_exact_offset;
            drag_data.from = OnScreen(canvas_top_left_pos.0 + started_tile_exact_offset + bounds_offset);
            drag_data.to = OnScreen(canvas_top_left_pos.0 + hovered_tile_exact_offset);
        }

        // todo restrict moving selection display to canvas
        // move_offset.x = move_offset.x.clamp(-bounds.min.x, (31. * self.scale) - bounds.max.x);
        // move_offset.y = move_offset.y.clamp(-bounds.min.y, (31. * self.scale) - bounds.max.y);

        self.selection_offset = Some(drag_data.delta());
    }

    pub(super) fn handle_edition_erase(&mut self, relative_pointer_pos: OnScreen<Pos2>) {
        self.delete_tiles_at(relative_pointer_pos);
        self.unselect_all_tiles();
    }

    pub(super) fn handle_edition_probe(&mut self, relative_pointer_pos: OnScreen<Pos2>) {
        self.probe_tile_at(relative_pointer_pos);
        self.unselect_all_tiles();
    }
}
