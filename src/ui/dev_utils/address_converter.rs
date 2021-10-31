use std::fmt::Write;

use imgui::{Ui, Window};
use smwe_rom::snes_utils::addr::{Addr, AddrPc, AddrSnes};

use self::{helpers::*, modes::*};
use crate::{
    frame_context::FrameContext,
    ui::{
        color,
        tool::{title_with_id, UiTool, WindowId},
    },
};

pub struct UiAddressConverter {
    title: String,

    conversion_mode: ConversionMode,
    include_header:  bool,

    text_pc:    String,
    text_snes:  String,
    text_error: String,
}

impl UiTool for UiAddressConverter {
    fn tick(&mut self, ctx: &mut FrameContext) -> bool {
        let mut running = true;

        let title = std::mem::take(&mut self.title);
        Window::new(&title)
            .always_auto_resize(true)
            .resizable(false)
            .collapsible(false)
            .scroll_bar(false)
            .opened(&mut running)
            .build(ctx.ui, || {
                self.mode_selection(ctx.ui);
                self.conversions(ctx.ui);
            });
        self.title = title;

        if !running {
            log::info!("Closed Address Converter");
        }
        running
    }
}

impl UiAddressConverter {
    pub fn new(id: WindowId) -> Self {
        log::info!("Opened Address Converter");
        UiAddressConverter {
            title:           title_with_id("Address converter", id),
            conversion_mode: ConversionMode::LoRom,
            include_header:  false,
            text_pc:         String::from("0"),
            text_snes:       String::from("8000"),
            text_error:      String::from(""),
        }
    }

    fn mode_selection(&mut self, ui: &Ui) {
        let lorom_changed = ui.radio_button("PC and LoROM", &mut self.conversion_mode, ConversionMode::LoRom);
        let hirom_changed = ui.radio_button("PC and HiROM", &mut self.conversion_mode, ConversionMode::HiRom);
        if lorom_changed || hirom_changed {
            log::info!("Conversion mode changed to {}", self.conversion_mode);
            self.update_addresses(ConvDir::PcToSnes);
        }

        if ui.checkbox("Include header", &mut self.include_header) {
            log::info!("Inclusion of SMC header: {}", if self.include_header { "ON" } else { "OFF" });
            let addr_pc = usize::from_str_radix(&self.text_pc, 16).unwrap_or(0);
            let addr_pc = adjust_to_header(addr_pc, self.include_header);
            self.text_pc.clear();
            write!(&mut self.text_pc, "{:x}", addr_pc).unwrap();
            self.update_addresses(ConvDir::PcToSnes);
        }
    }

    fn conversions(&mut self, ui: &Ui) {
        self.address_input(ui, ConvDir::PcToSnes);
        self.address_input(ui, ConvDir::SnesToPc);
        if !self.text_error.is_empty() {
            ui.text_colored(color::TEXT_ERROR, &self.text_error);
        }
    }

    fn address_input(&mut self, ui: &Ui, direction: ConvDir) {
        let (label, buf) = match direction {
            ConvDir::PcToSnes => ("PC", &mut self.text_pc),
            ConvDir::SnesToPc => ("SNES", &mut self.text_snes),
        };

        if ui.input_text(label, buf).chars_hexadecimal(true).chars_noblank(true).auto_select_all(true).build() {
            log::info!("Changed input '{}' to: {}", direction, buf);
            self.update_addresses(direction);
        }
    }

    fn update_addresses(&mut self, direction: ConvDir) {
        let (buf_src, buf_dst) = match direction {
            ConvDir::PcToSnes => (&mut self.text_pc, &mut self.text_snes),
            ConvDir::SnesToPc => (&mut self.text_snes, &mut self.text_pc),
        };

        let addr_src = {
            let addr = usize::from_str_radix(buf_src, 16).unwrap_or(0);
            if self.include_header {
                match direction {
                    ConvDir::PcToSnes => adjust_to_header(addr, false),
                    ConvDir::SnesToPc => adjust_to_header(addr, true),
                }
            } else {
                addr
            }
        };

        let addr_dst = match direction {
            ConvDir::PcToSnes => {
                let res = match self.conversion_mode {
                    ConversionMode::LoRom => AddrSnes::try_from_lorom(AddrPc(addr_src)),
                    ConversionMode::HiRom => AddrSnes::try_from_hirom(AddrPc(addr_src)),
                };
                match res {
                    Ok(addr) => Ok(addr.0),
                    Err(err) => Err(err),
                }
            }
            ConvDir::SnesToPc => {
                let res = match self.conversion_mode {
                    ConversionMode::LoRom => AddrPc::try_from_lorom(AddrSnes(addr_src)),
                    ConversionMode::HiRom => AddrPc::try_from_hirom(AddrSnes(addr_src)),
                };
                match res {
                    Ok(addr) => Ok(addr.0),
                    Err(err) => Err(err),
                }
            }
        };

        if let Err(msg) = addr_dst {
            self.text_error = msg.to_string();
        } else {
            let addr_dst = addr_dst.unwrap();
            buf_dst.clear();
            write!(buf_dst, "{:x}", addr_dst).unwrap();
            self.text_error.clear();
        }
    }
}

mod modes {
    use std::{cmp::PartialEq, fmt};

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum ConversionMode {
        LoRom,
        HiRom,
    }

    pub enum ConvDir {
        PcToSnes,
        SnesToPc,
    }

    impl fmt::Display for ConversionMode {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(match self {
                ConversionMode::LoRom => "PC ↔ LoRom",
                ConversionMode::HiRom => "PC ↔ HiRom",
            })
        }
    }

    impl fmt::Display for ConvDir {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(match self {
                ConvDir::PcToSnes => "PC → SNES",
                ConvDir::SnesToPc => "SNES → PC",
            })
        }
    }
}

mod helpers {
    use smwe_rom::snes_utils::rom::SMC_HEADER_SIZE;

    pub fn adjust_to_header(addr: usize, include_header: bool) -> usize {
        if include_header {
            addr + SMC_HEADER_SIZE
        } else if addr >= SMC_HEADER_SIZE {
            addr - SMC_HEADER_SIZE
        } else {
            0
        }
    }
}
