use crate::frame_context::FrameContext;

pub trait UiTool {
    fn tick(&mut self, ctx: &mut FrameContext) -> bool;
}

pub type WindowId = i32;

pub fn title_with_id(title: &str, id: WindowId) -> String {
    format!("{}##{}", title, id)
}
