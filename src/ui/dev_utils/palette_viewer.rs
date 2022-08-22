use eframe::egui::{
    Color32,
    ColorImage,
    ComboBox,
    DragValue,
    TextureFilter,
    TextureHandle,
    TopBottomPanel,
    Ui,
    Window,
};
use num_enum::TryFromPrimitive;
use smwe_rom::graphics::palette::{ColorPalette, OverworldState};

use crate::{frame_context::FrameContext, ui::tool::UiTool};

#[repr(usize)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, TryFromPrimitive)]
pub enum PaletteContext {
    Level     = 0,
    Overworld = 1,
}

pub struct UiPaletteViewer {
    palette_context:      PaletteContext,
    palette_image_handle: Option<TextureHandle>,
    // Level viewer
    level_num:            i32,
    // Overworld viewer
    submap_num:           i32,
    special_completed:    bool,
}

impl Default for UiPaletteViewer {
    fn default() -> Self {
        log::info!("Opened Palette Viewer");
        UiPaletteViewer {
            palette_context:      PaletteContext::Level,
            palette_image_handle: None,
            level_num:            0,
            submap_num:           0,
            special_completed:    false,
        }
    }
}

impl UiTool for UiPaletteViewer {
    fn update(&mut self, ui: &mut Ui, ctx: &mut FrameContext) -> bool {
        let mut running = true;

        if self.palette_image_handle.is_none() {
            self.update_palette_image(ui, ctx);
        }

        Window::new("Color palettes") //
            .resizable(false)
            .open(&mut running)
            .show(ui.ctx(), |ui| {
                TopBottomPanel::top("palette_selectors_panel").show_inside(ui, |ui| {
                    self.context_selector(ui, ctx);
                    match self.palette_context {
                        PaletteContext::Level => self.selectors_level(ui, ctx),
                        PaletteContext::Overworld => self.selectors_overworld(ui, ctx),
                    }
                });
                ui.centered_and_justified(|ui| self.display_palette(ui));
            });

        if !running {
            log::info!("Closed Palette Viewer");
        }
        running
    }
}

impl UiPaletteViewer {
    fn context_selector(&mut self, ui: &mut Ui, ctx: &mut FrameContext) {
        let mut context_changed = false;
        let mut context_raw = self.palette_context as usize;
        let context_names = ["Level", "Overworld"];

        ComboBox::from_label("Context").selected_text(context_names[context_raw]).show_ui(ui, |ui| {
            for (context_idx, &context_name) in context_names.iter().enumerate() {
                if ui.selectable_value(&mut context_raw, context_idx, context_name).changed() {
                    context_changed = true;
                }
            }
        });

        self.palette_context = PaletteContext::try_from(context_raw).unwrap_or(PaletteContext::Level);

        if context_changed {
            self.update_palette_image(ui, ctx);
        }
    }

    fn selectors_level(&mut self, ui: &mut Ui, ctx: &mut FrameContext) {
        let level_count = {
            let project = ctx.project_ref.as_ref().unwrap().borrow();
            project.rom_data.levels.len() as i32
        };

        ui.horizontal(|ui| {
            if ui
                .add({
                    DragValue::new(&mut self.level_num)
                        .clamp_range(0..=level_count - 1)
                        .custom_formatter(|n, _| format!("{:03X}", n as i64))
                })
                .changed()
            {
                log::info!("Showing color palette for level {:X}", self.level_num);
                self.update_palette_image(ui, ctx);
            }
            ui.label("Level number");
        });
    }

    fn selectors_overworld(&mut self, ui: &mut Ui, ctx: &mut FrameContext) {
        if ui.checkbox(&mut self.special_completed, "Special world completed").changed() {
            log::info!(
                "Showing color palette for {}",
                if self.special_completed { "post-special world" } else { "pre-special world" }
            );
            self.update_palette_image(ui, ctx);
        }

        let submap_count = {
            let project = ctx.project_ref.as_ref().unwrap().borrow();
            project.rom_data.color_palettes.ow_specific_set.layer2_indices.len() as i32
        };

        ui.horizontal(|ui| {
            if ui
                .add({
                    DragValue::new(&mut self.submap_num)
                        .clamp_range(0..=submap_count - 1)
                        .custom_formatter(|n, _| format!("{:X}", n as i64))
                })
                .changed()
            {
                log::info!("Showing color palette for submap {:X}", self.submap_num);
                self.update_palette_image(ui, ctx);
            }
            ui.label("Submap number");
        });
    }

    fn update_palette_image(&mut self, ui: &mut Ui, ctx: &mut FrameContext) {
        let mut update_image = |palette: &dyn ColorPalette| {
            let mut image = ColorImage::new([16, 16], Color32::BLACK);
            for y in 0..=0xF {
                for x in 0..=0xF {
                    let color = palette.get_color_at(y, x).unwrap();
                    image[(x, y)] = Color32::from(color);
                }
            }

            self.palette_image_handle = Some(ui.ctx().load_texture("palette-image", image, TextureFilter::Nearest));
        };

        let project = ctx.project_ref.as_ref().unwrap().borrow();
        let rom = &project.rom_data;
        match self.palette_context {
            PaletteContext::Level => {
                let header = &rom.levels[self.level_num as usize].primary_header;
                update_image(&rom.color_palettes.get_level_palette(header).unwrap());
            }
            PaletteContext::Overworld => {
                let ow_state =
                    if self.special_completed { OverworldState::PostSpecial } else { OverworldState::PreSpecial };
                update_image(&rom.color_palettes.get_submap_palette(self.submap_num as usize, ow_state).unwrap());
            }
        }
    }

    fn display_palette(&mut self, ui: &mut Ui) {
        const CELL_SIZE: f32 = 20.0;
        let image_handle: &TextureHandle = self.palette_image_handle.as_ref().unwrap();
        ui.image(image_handle, image_handle.size_vec2() * CELL_SIZE);
    }
}
