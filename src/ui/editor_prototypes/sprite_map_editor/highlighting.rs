use egui::*;
use smwe_math::space::{OnCanvas, OnGrid, OnScreen};
use smwe_widgets::vram_view::VramSelectionMode;

use super::UiSpriteMapEditor;
use crate::ui::{
    editing_mode::*,
    style::{CellSelectorStyle, EditorStyle},
};

impl UiSpriteMapEditor {
    pub(super) fn higlight_hovered_tiles(
        &mut self, ui: &mut Ui, relative_pointer_pos: OnScreen<Pos2>, canvas_left_top: OnScreen<Pos2>,
    ) {
        let pointer_pos_canvas = relative_pointer_pos.to_canvas(self.pixels_per_point, self.zoom);
        match self.editing_mode {
            EditingMode::Move(_) => {
                if self
                    .selected_sprite_tile_indices
                    .iter()
                    .map(|&i| self.sprite_tiles[i])
                    .any(|tile| tile.contains_point(pointer_pos_canvas))
                {
                    self.hovering_selected_tile = true;
                } else if let Some(hovered_tile) =
                    self.sprite_tiles.iter().find(|&&tile| tile.contains_point(pointer_pos_canvas))
                {
                    let tile_pos_in_canvas = hovered_tile.pos().to_screen(self.pixels_per_point, self.zoom);
                    let exact_tile_pos = OnScreen(canvas_left_top.0 + tile_pos_in_canvas.0.to_vec2());
                    self.highlight_tile_at(
                        ui,
                        exact_tile_pos,
                        CellSelectorStyle::get_from_egui(ui.ctx(), |style| style.hovered_tile_highlight_color),
                        1.,
                    );
                } else {
                    let (selection_scale, max_selected_tile) = match self.vram_selection_mode {
                        VramSelectionMode::SingleTile => (1., self.grid_size.0),
                        VramSelectionMode::TwoByTwoTiles => (2., self.grid_size.0 - vec2(1., 1.)),
                    };
                    let tile_pos_in_canvas = relative_pointer_pos
                        .to_grid(self.pixels_per_point, self.zoom, self.tile_size_px)
                        .clamp(OnGrid(pos2(0., 0.)), OnGrid(max_selected_tile.to_pos2()))
                        .to_screen(self.pixels_per_point, self.zoom, self.tile_size_px);
                    let exact_tile_pos = OnScreen(canvas_left_top.0 + tile_pos_in_canvas.0.to_vec2());
                    self.highlight_tile_at(
                        ui,
                        exact_tile_pos,
                        CellSelectorStyle::get_from_egui(ui.ctx(), |style| style.hovered_void_highlight_color),
                        selection_scale,
                    );
                }
            }
            EditingMode::Erase => {
                if let Some(hovered_tile) =
                    self.sprite_tiles.iter().find(|&&tile| tile.contains_point(pointer_pos_canvas))
                {
                    let tile_pos_in_canvas = hovered_tile.pos().to_screen(self.pixels_per_point, self.zoom);
                    let exact_tile_pos = OnScreen(canvas_left_top.0 + tile_pos_in_canvas.0.to_vec2());
                    self.highlight_tile_at(
                        ui,
                        exact_tile_pos,
                        CellSelectorStyle::get_from_egui(ui.ctx(), |style| style.delete_highlight_color),
                        1.,
                    );
                }
            }
            _ => {}
        }
    }

    pub(super) fn highlight_tile_at(&self, ui: &mut Ui, point: OnScreen<Pos2>, color: impl Into<Color32>, scale: f32) {
        ui.painter().rect_filled(
            Rect::from_min_size(
                point.0,
                OnCanvas(Vec2::splat(self.tile_size_px * scale)).to_screen(self.pixels_per_point, self.zoom).0,
            ),
            Rounding::none(),
            color,
        );
    }

    pub(super) fn highlight_selected_tiles(&mut self, ui: &mut Ui, canvas_pos: OnScreen<Pos2>) {
        let selection_offset = self.selection_offset.take().unwrap_or_default();
        for tile in self.selected_sprite_tile_indices.iter().map(|&idx| &self.sprite_tiles[idx]) {
            self.highlight_tile_at(
                ui,
                OnScreen(
                    canvas_pos.0
                        + selection_offset.0
                        + tile.pos().to_screen(self.pixels_per_point, self.zoom).0.to_vec2(),
                ),
                CellSelectorStyle::get_from_egui(ui.ctx(), |style| {
                    if self.hovering_selected_tile {
                        style.hovered_tile_highlight_color
                    } else {
                        style.selection_highlight_color
                    }
                }),
                1.,
            );
        }
    }
}
