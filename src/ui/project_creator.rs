use std::path::Path;

use eframe::egui::{Button, Ui, Window};
use rfd::FileDialog;

use crate::{
    project::Project,
    ui::style::{EditorStyle, ErrorStyle},
};

#[derive(Debug)]
pub struct UiProjectCreator {
    project_title: String,
    base_rom_path: String,

    err_project_title:    String,
    err_base_rom_path:    String,
    err_project_creation: String,
}

impl Default for UiProjectCreator {
    fn default() -> Self {
        log::info!("Opened Project Creator");
        let mut myself = UiProjectCreator {
            project_title: String::from("My SMW hack"),
            base_rom_path: String::from("./smw.smc"),

            err_project_title:    String::new(),
            err_base_rom_path:    String::new(),
            err_project_creation: String::new(),
        };
        myself.handle_rom_file_path();
        myself
    }
}

impl UiProjectCreator {
    pub fn update(&mut self, ui: &Ui) -> bool {
        let mut opened = true;
        let mut created_or_cancelled = false;

        Window::new("Create new project").auto_sized().resizable(false).collapsible(false).open(&mut opened).show(
            ui.ctx(),
            |ui| {
                self.input_project_title(ui);
                self.input_rom_file_path(ui);
                self.create_or_cancel(ui, &mut created_or_cancelled);
            },
        );

        let running = opened && !created_or_cancelled;
        if !running {
            log::info!("Closed Project Creator");
        }
        running
    }

    fn input_project_title(&mut self, ui: &mut Ui) {
        ui.label("Project title");
        if ui.text_edit_singleline(&mut self.project_title).changed() {
            self.handle_project_title();
        }
        if !self.err_project_title.is_empty() {
            ui.colored_label(ErrorStyle::get_from_egui(ui.ctx(), |style| style.text_color), &self.err_project_title);
        }
    }

    fn handle_project_title(&mut self) {
        if self.project_title.is_empty() {
            self.err_project_title = String::from("Project title cannot be empty.");
        } else {
            self.err_project_title.clear();
        }
    }

    fn input_rom_file_path(&mut self, ui: &mut Ui) {
        ui.label("Base ROM file");
        ui.horizontal(|ui| {
            if ui.text_edit_singleline(&mut self.base_rom_path).changed() {
                self.handle_rom_file_path();
            }
            if ui.small_button("Browse...").clicked() {
                self.open_file_selector();
            }
        });
        if !self.err_base_rom_path.is_empty() {
            ui.colored_label(ErrorStyle::get_from_egui(ui.ctx(), |style| style.text_color), &self.err_base_rom_path);
        }
    }

    fn handle_rom_file_path(&mut self) {
        let file_path = Path::new(&self.base_rom_path);
        if !file_path.exists() {
            self.err_base_rom_path = format!("File '{}' does not exist.", self.base_rom_path);
        } else if file_path.is_dir() {
            self.err_base_rom_path = format!("'{}' is not a file.", self.base_rom_path);
        } else {
            self.err_base_rom_path.clear();
        }
    }

    fn open_file_selector(&mut self) {
        log::info!("Opened File Selector");
        match FileDialog::new().add_filter("SNES ROM File (*.smc, *.sfc)", &["smc", "sfc"]).pick_file() {
            Some(path) => {
                self.base_rom_path = String::from(path.to_str().unwrap());
                self.handle_rom_file_path();
            }
            None => log::error!("Cannot open SMW ROM"),
        }
    }

    fn create_or_cancel(&mut self, ui: &mut Ui, created_or_cancelled: &mut bool) {
        ui.horizontal(|ui| {
            if ui.add_enabled(self.no_creation_errors(), Button::new("Create").small()).clicked() {
                log::info!("Attempting to create a new project");
                self.handle_project_creation(ui, created_or_cancelled);
            }
            if ui.small_button("Cancel").clicked() {
                log::info!("Cancelled project creation");
                *created_or_cancelled = true;
            }
        });
        if !self.err_project_creation.is_empty() {
            ui.colored_label(ErrorStyle::get_from_egui(ui.ctx(), |style| style.text_color), &self.err_project_creation);
        }
    }

    fn handle_project_creation(&mut self, ui: &Ui, created_or_cancelled: &mut bool) {
        match Project::new(&self.base_rom_path) {
            Ok(project) => {
                log::info!("Success creating a new project");
                ui.data_mut(|data| {
                    data.insert_temp(Project::project_title_id(), project.title);
                    data.insert_temp(Project::rom_id(), project.rom);
                });
                *created_or_cancelled = true;
                self.err_project_creation.clear();
            }
            Err(err) => {
                log::info!("Failed to create a new project: {err}");
                self.err_project_creation = err.to_string();
            }
        }
    }

    fn no_creation_errors(&self) -> bool {
        self.err_base_rom_path.is_empty() && self.err_project_title.is_empty()
    }
}
