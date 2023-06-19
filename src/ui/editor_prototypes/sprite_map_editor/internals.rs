use std::{fs, ops::Not, path::PathBuf};

use duplicate::duplicate;
use egui::emath::*;
use itertools::Itertools;
use num::Integer;
use paste::paste;
use rfd::{MessageButtons, MessageDialog, MessageLevel};
use smwe_math::coordinates::{OnCanvas, OnScreen};
use smwe_render::tile_renderer::{Tile, TileJson};

use super::UiSpriteMapEditor;
use crate::ui::editing_mode::SnapToGrid;

impl UiSpriteMapEditor {
    pub(super) fn create_new_map(&mut self) {
        self.sprite_tiles.clear();
        self.upload_tiles();
    }

    pub(super) fn open_map(&mut self, path: PathBuf) {
        match fs::read_to_string(path) {
            Err(e) => {
                MessageDialog::new()
                    .set_title("Failed to open selected file.")
                    .set_description(&format!("{e:?}"))
                    .set_level(MessageLevel::Error)
                    .set_buttons(MessageButtons::Ok)
                    .show();
            }
            Ok(s) => match serde_json::from_str::<Vec<TileJson>>(&s) {
                Err(e) => {
                    MessageDialog::new()
                        .set_title("Failed to deserialize sprite tile map from JSON.")
                        .set_description(&format!("{e:?}"))
                        .set_level(MessageLevel::Error)
                        .set_buttons(MessageButtons::Ok)
                        .show();
                }
                Ok(tiles) => {
                    self.sprite_tiles = tiles.into_iter().map(Tile::from).collect_vec();
                    self.upload_tiles();
                }
            },
        }
    }

    pub(super) fn save_map_as(&mut self, path: PathBuf) {
        let tiles = self.sprite_tiles.iter().map(|&t| TileJson::from(t)).collect_vec();
        match serde_json::to_string_pretty(&tiles) {
            Err(e) => {
                MessageDialog::new()
                    .set_title("Failed to serialize sprite tile map into JSON.")
                    .set_description(&format!("{e:?}"))
                    .set_level(MessageLevel::Error)
                    .set_buttons(MessageButtons::Ok)
                    .show();
            }
            Ok(s) => {
                if let Err(e) = fs::write(path, s) {
                    MessageDialog::new()
                        .set_title("Save sprite tile map to selected file.")
                        .set_description(&format!("{e:?}"))
                        .set_level(MessageLevel::Error)
                        .set_buttons(MessageButtons::Ok)
                        .show();
                }
            }
        }
    }

    pub(super) fn update_cpu(&mut self) {
        smwe_emu::emu::decompress_sublevel(&mut self.cpu, self.level_num);
        println!("Updated CPU");
    }

    pub(super) fn update_renderers(&mut self) {
        self.gfx_bufs.upload_palette(&self.gl, &self.cpu.mem.cgram);
        self.gfx_bufs.upload_vram(&self.gl, &self.cpu.mem.vram);
    }

    pub(super) fn upload_tiles(&self) {
        self.sprite_renderer
            .lock()
            .expect("Cannot lock mutex on sprite renderer")
            .set_tiles(&self.gl, self.sprite_tiles.clone());
    }

    pub(super) fn any_tile_contains_pointer(
        &mut self, pointer_pos: OnScreen<Pos2>, canvas_top_left_pos: OnScreen<Pos2>,
    ) -> bool {
        self.selected_sprite_tile_indices.iter().map(|&i| self.sprite_tiles[i]).any(|tile| {
            let pointer_in_canvas =
                pointer_pos.relative_to(canvas_top_left_pos).to_canvas(self.pixels_per_point, self.zoom);
            tile.contains_point(pointer_in_canvas)
        })
    }

    pub(super) fn move_selected_tiles_by(&mut self, mut move_offset: OnCanvas<Vec2>, snap_to_grid: Option<SnapToGrid>) {
        if self.selected_sprite_tile_indices.is_empty() {
            return;
        }

        let bounds = self.selection_bounds.expect("unset even though some tiles are selected");
        move_offset = move_offset.clamp(
            -bounds.left_top().to_vec2(),
            OnCanvas::<Vec2>::splat(31. * self.tile_size_px) - bounds.right_bottom().to_vec2(),
        );

        for &idx in self.selected_sprite_tile_indices.iter() {
            self.sprite_tiles[idx].move_by(move_offset);
            if let Some(snap_to_grid) = snap_to_grid {
                self.sprite_tiles[idx].snap_to_grid(self.tile_size_px as u32, snap_to_grid.cell_origin);
            }
        }

        self.compute_selection_bounds();
        self.upload_tiles();
    }

    pub(super) fn add_selected_tile_at(&mut self, pos: OnCanvas<Pos2>) {
        let tile_idx = (self.selected_vram_tile.0 + self.selected_vram_tile.1 * 16) as usize;
        let mut tile = self.tile_palette[tile_idx + (32 * 16)];
        tile.0[0] = pos.0.x.floor() as u32;
        tile.0[1] = pos.0.y.floor() as u32;
        self.sprite_tiles.push(tile);
        self.upload_tiles();
    }

    pub(super) fn select_tile_at(&mut self, pos: OnScreen<Pos2>, clear_previous_selection: bool) {
        if clear_previous_selection {
            self.unselect_all_tiles();
        }

        if let Some((idx, _)) = self
            .sprite_tiles
            .iter()
            .enumerate()
            .rev()
            .find(|(_, &tile)| tile.contains_point(pos.to_canvas(self.pixels_per_point, self.zoom)))
        {
            self.selected_sprite_tile_indices.insert(idx);
        }
        self.compute_selection_bounds();
    }

    pub(super) fn select_tiles_in(&mut self, rect: OnScreen<Rect>, clear_previous_selection: bool) {
        if clear_previous_selection {
            self.unselect_all_tiles();
        }

        let indices = self
            .sprite_tiles
            .iter()
            .enumerate()
            .filter(|(_, &tile)| tile.intersects_rect(rect.to_canvas(self.pixels_per_point, self.zoom)))
            .map(|(i, _)| i)
            .collect_vec();
        self.mark_tiles_as_selected(indices.into_iter());
    }

    pub(super) fn unselect_all_tiles(&mut self) {
        self.selected_sprite_tile_indices.clear();
        self.selection_bounds = None;
    }

    pub(super) fn mark_tiles_as_selected(&mut self, indices: impl IntoIterator<Item = usize>) {
        for index in indices {
            self.selected_sprite_tile_indices.insert(index);
        }
        self.compute_selection_bounds();
    }

    pub(super) fn compute_selection_bounds(&mut self) {
        self.selection_bounds = self.selected_sprite_tile_indices.is_empty().not().then(|| {
            duplicate! {
                [dimension; [x]; [y]]
                paste! {
                    let ([<min_tile_ dimension>], [<max_tile_ dimension>]) = self
                        .selected_sprite_tile_indices
                        .iter()
                        .map(|&i| self.sprite_tiles[i].pos())
                        .minmax_by(|a, b| a.0.dimension.total_cmp(&b.0.dimension))
                        .into_option()
                        .map(|(min, max)| (min.0.dimension, max.0.dimension))
                        .unwrap();
                }
            }
            OnCanvas(Rect::from_min_max(pos2(min_tile_x, min_tile_y), pos2(max_tile_x, max_tile_y)))
        });
    }

    pub(super) fn delete_tiles_at(&mut self, pos: OnScreen<Pos2>) {
        self.sprite_tiles.retain(|&tile| !tile.contains_point(pos.to_canvas(self.pixels_per_point, self.zoom)));
        self.upload_tiles();
    }

    pub(super) fn probe_tile_at(&mut self, pos: OnScreen<Pos2>) {
        if let Some(tile) = self
            .sprite_tiles
            .iter()
            .rev()
            .find(|&&tile| tile.contains_point(pos.to_canvas(self.pixels_per_point, self.zoom)))
        {
            let (y, x) = tile.tile_num().div_rem(&16);
            self.selected_vram_tile = (x, y - 96);
        };
    }

    pub(super) fn update_tile_palette(&mut self) {
        for tile in self.tile_palette.iter_mut() {
            tile.0[3] &= 0xC0FF;
            tile.0[3] |= (self.selected_palette + 8) << 8;
        }
        self.vram_renderer
            .lock()
            .expect("Cannot lock mutex on VRAM renderer")
            .set_tiles(&self.gl, self.tile_palette.clone());
    }
}
