use eframe::egui::Ui;
use egui::WidgetText;
use enum_dispatch::enum_dispatch;

use crate::ui::{
    dev_utils::{
        address_converter::UiAddressConverter,
        disassembler::UiDisassembler,
        gfx_viewer::UiGfxViewer,
        palette_viewer::UiPaletteViewer,
        rom_info::UiRomInfo,
        tiles16x16::UiTiles16x16,
    },
    editor_prototypes::{block_editor::UiBlockEditor, code_editor::UiCodeEditor},
    frame_context::EditorToolTabViewer,
};

#[enum_dispatch]
pub enum DockableEditorToolEnum {
    UiAddressConverter,
    UiBlockEditor,
    UiCodeEditor,
    UiDisassembler,
    UiGfxViewer,
    UiPaletteViewer,
    UiRomInfo,
    UiTiles16x16,
}

#[enum_dispatch(DockableEditorToolEnum)]
pub trait DockableEditorTool {
    fn update(&mut self, ui: &mut Ui, ctx: &mut EditorToolTabViewer);
    fn title(&self) -> WidgetText;
}
