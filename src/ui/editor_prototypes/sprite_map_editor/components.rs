use std::sync::Arc;

use duplicate::duplicate;
use egui::*;
use egui_glow::CallbackFn;
use egui_phosphor as icons;
use inline_tweak::tweak;
use smwe_render::{palette_renderer::PaletteUniforms, tile_renderer::TileUniforms};
use smwe_widgets::vram_view::*;

use super::UiSpriteMapEditor;
use crate::ui::editing_mode::*;

impl UiSpriteMapEditor {
    pub(super) fn tile_selector(&mut self, ui: &mut Ui) {
        let vram_renderer = Arc::clone(&self.vram_renderer);
        let gfx_bufs = self.gfx_bufs;

        ui.strong("VRAM");
        ui.add(
            VramView::new(Arc::clone(&vram_renderer), gfx_bufs)
                .viewed_tiles(ViewedVramTiles::SpritesOnly)
                .selection(&mut self.selected_vram_tile)
                .zoom(2.),
        );
    }

    pub(super) fn tile_selection_preview(&mut self, ui: &mut Ui) {
        let vram_renderer = Arc::clone(&self.vram_renderer);
        let gfx_bufs = self.gfx_bufs;

        ui.strong("Selection preview");
        let px = self.pixels_per_point;
        let zoom = tweak!(8.);
        let (rect, _response) = ui.allocate_exact_size(Vec2::splat(zoom * 8. / px), Sense::hover());

        let screen_size = rect.size() * px;
        let offset = vec2(-(self.selected_vram_tile.0 as f32), -32. - self.selected_vram_tile.1 as f32) * zoom;

        ui.painter().add(PaintCallback {
            rect,
            callback: Arc::new(CallbackFn::new(move |_info, painter| {
                vram_renderer
                    .lock()
                    .expect("Cannot lock mutex on selected tile view's tile renderer")
                    .paint(painter.gl(), &TileUniforms { gfx_bufs, screen_size, offset, zoom });
            })),
        });
    }

    pub(super) fn palette_row_selector(&mut self, ui: &mut Ui) {
        ui.strong("Palette");
        Frame::canvas(ui.style()).show(ui, |ui| {
            let palette_renderer = Arc::clone(&self.palette_renderer);
            let uniforms = PaletteUniforms { palette_buf: self.gfx_bufs.palette_buf };
            let (rect, _response) = ui.allocate_exact_size(Vec2::splat(tweak!(230.)), Sense::click());
            ui.painter().add(PaintCallback {
                rect,
                callback: Arc::new(CallbackFn::new(move |_info, painter| {
                    palette_renderer
                        .lock()
                        .expect("Cannot lock mutex on palette renderer")
                        .paint(painter.gl(), &uniforms);
                })),
            });
        });
    }

    pub(super) fn editing_mode_selector(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            duplicate! {
                [
                    icon mode_name mode_desc mode_pattern mode_value;

                    [icons::ARROWS_OUT_CARDINAL]
                    ["Move mode"]
                    ["Double click to insert tile, single click to select, drag to move."]
                    [EditingMode::Move(_)]
                    [EditingMode::Move(None)];

                    [icons::RECTANGLE]
                    ["Select mode"]
                    ["Left-click and drag to select tiles."]
                    [EditingMode::Select]
                    [EditingMode::Select];

                    [icons::PENCIL]
                    ["Draw mode"]
                    ["Insert tiles while left mouse button is pressed."]
                    [EditingMode::Draw]
                    [EditingMode::Draw];

                    [icons::ERASER]
                    ["Erase mode"]
                    ["Delete tiles while left mouse button is pressed."]
                    [EditingMode::Erase]
                    [EditingMode::Erase];

                    [icons::EYEDROPPER]
                    ["Probe mode"]
                    ["Pick a tile from the canvas on left-click."]
                    [EditingMode::Probe]
                    [EditingMode::Probe];
                ]
                {
                    let button = if matches!(self.editing_mode, mode_pattern) {
                        Button::new(icon).fill(Color32::from_rgb(tweak!(200), tweak!(30), tweak!(70)))
                    } else {
                        Button::new(icon)
                    };

                    let tooltip = |ui: &mut Ui| {
                        ui.strong(mode_name);
                        ui.label(mode_desc);
                    };

                    if ui.add(button).on_hover_ui_at_pointer(tooltip).clicked() {
                        self.editing_mode = mode_value;
                    }
                }
            }
        });
    }

    pub(super) fn editing_area(&mut self, ui: &mut Ui, editing_area_size: Vec2) {
        let sprite_renderer = Arc::clone(&self.sprite_renderer);
        let gfx_bufs = self.gfx_bufs;
        let (canvas_rect, response) =
            ui.allocate_exact_size(editing_area_size / self.pixels_per_point, Sense::click_and_drag());
        let screen_size = canvas_rect.size() * self.pixels_per_point;
        let scale_pp = self.tile_size_px / self.pixels_per_point;
        let zoom = self.zoom;

        // Tiles
        ui.painter().add(PaintCallback {
            rect:     canvas_rect,
            callback: Arc::new(CallbackFn::new(move |_info, painter| {
                sprite_renderer
                    .lock()
                    .expect("Cannot lock mutex on sprite renderer")
                    .paint(painter.gl(), &TileUniforms { gfx_bufs, screen_size, offset: Vec2::ZERO, zoom });
            })),
        });

        // Grid
        if self.always_show_grid || ui.input(|i| i.modifiers.shift_only()) {
            let spacing = self.zoom * self.tile_size_px / self.pixels_per_point;
            let stroke = Stroke::new(1., Color32::from_white_alpha(tweak!(70)));
            for cell in 0..33 {
                let position = cell as f32 * spacing;
                ui.painter().hline(canvas_rect.min.x..=canvas_rect.max.x, canvas_rect.min.y + position, stroke);
                ui.painter().vline(canvas_rect.min.x + position, canvas_rect.min.y..=canvas_rect.max.y, stroke);
            }
        }

        // DEBUG: show selection bounds
        if self.debug_selection_bounds {
            if let Some(mut bounds) = self.selection_bounds {
                let scaling = self.zoom / self.pixels_per_point;
                bounds.min = canvas_rect.left_top() + (bounds.min.to_vec2() * scaling);
                bounds.max =
                    canvas_rect.left_top() + ((bounds.max.to_vec2() + Vec2::splat(self.tile_size_px)) * scaling);
                ui.painter().rect_stroke(bounds, Rounding::none(), Stroke::new(2., Color32::BLUE));
            }
        }

        // Interaction
        if let Some(hover_pos) = response.hover_pos() {
            let canvas_top_left_pos = canvas_rect.left_top();

            let relative_pointer_offset = hover_pos - canvas_rect.left_top();
            let relative_pointer_pos = relative_pointer_offset.to_pos2();

            let hovered_tile_offset = (relative_pointer_offset / scale_pp / self.zoom).floor();
            let hovered_tile_offset = hovered_tile_offset.clamp(vec2(0., 0.), vec2(31., 31.));
            let grid_cell_pos = (hovered_tile_offset * self.tile_size_px).to_pos2();

            let holding_shift = ui.input(|i| i.modifiers.shift_only());
            let holding_ctrl = ui.input(|i| i.modifiers.command_only());

            self.higlight_hovered_tiles(ui, relative_pointer_pos, canvas_rect.left_top());

            if self.editing_mode.inserted(&response) {
                self.handle_edition_insert(grid_cell_pos);
            }

            if let Some(selection) = self.editing_mode.selected(&response) {
                if let Selection::Drag(Some(selection_rect)) = selection {
                    ui.painter().rect_stroke(
                        selection_rect,
                        Rounding::none(),
                        Stroke::new(1., ui.visuals().selection.bg_fill),
                    );
                }
                self.handle_selection_plot(selection, !holding_ctrl, canvas_top_left_pos);
            }

            if let Some(drag_data) = self.editing_mode.dropped(&response) {
                self.handle_edition_drop_moved(drag_data, holding_shift, canvas_top_left_pos);
            }

            if let Some(drag_data) = self.editing_mode.moving(&response) {
                self.handle_edition_dragging(drag_data, holding_shift, canvas_top_left_pos);
            }

            if self.editing_mode.erased(&response) {
                self.handle_edition_erase(relative_pointer_pos);
            }

            if self.editing_mode.probed(&response) {
                self.handle_edition_probe(relative_pointer_pos);
            }
        }

        self.highlight_selected_tiles(ui, canvas_rect.left_top());
        self.hovering_selected_tile = false;
    }
}
