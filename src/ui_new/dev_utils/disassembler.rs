use std::{cell::RefCell, collections::BTreeMap, fmt::Write, ops::Deref};

use eframe::egui::{Align, Color32, DragValue, Layout, RichText, SidePanel, Ui, Window};
use egui_extras::{Size, TableBuilder};
use inline_tweak::tweak;
use itertools::Itertools;
use smwe_rom::{
    disassembler::{binary_block::BinaryBlock, instruction::Instruction},
    snes_utils::addr::{Addr, AddrPc, AddrSnes},
};

use crate::{frame_context::EFrameContext, ui_new::tool::UiTool};

pub struct UiDisassembler {
    current_address_scroll: u32,
    address_y_map:          BTreeMap<AddrSnes, f32>,
    opt_draw_debug_info:    bool,
}

impl Default for UiDisassembler {
    fn default() -> Self {
        log::info!("Opened disassembler");
        Self {
            current_address_scroll: AddrSnes::MIN.0 as u32,
            address_y_map:          BTreeMap::new(),
            opt_draw_debug_info:    false,
        }
    }
}

impl UiTool for UiDisassembler {
    fn update(&mut self, ui: &mut Ui, ctx: &mut EFrameContext) -> bool {
        let mut running = true;

        Window::new("Disassembler") //
            .min_width(512.0)
            .min_height(128.0)
            .vscroll(true)
            .open(&mut running)
            .resizable(true)
            .show(ui.ctx(), |ui| {
                SidePanel::left("disasm_switches_panel").show_inside(ui, |ui| self.switches(ui, ctx));
                self.display_code(ui, ctx);
            });

        if !running {
            log::info!("Closed disassembler");
        }
        running
    }
}

impl UiDisassembler {
    fn switches(&mut self, ui: &mut Ui, ctx: &mut EFrameContext) {
        let project = ctx.project_ref.as_ref().unwrap().borrow();
        let disasm = &project.rom_data.disassembly;

        ui.add(
            DragValue::new(&mut self.current_address_scroll)
                .clamp_range({
                    let min = AddrSnes::MIN;
                    let max = AddrSnes::try_from_lorom(AddrPc(disasm.rom_bytes().len())).unwrap();
                    min.0..=max.0 - 1
                })
                .prefix("$")
                .custom_formatter(|n, _| format!("{:06X}", n as i64)),
        );
        ui.label("Address");

        ui.checkbox(&mut self.opt_draw_debug_info, "Draw debug info");
    }

    fn display_code(&mut self, ui: &mut Ui, ctx: &mut EFrameContext) {
        const COLOR_ADDRESS: Color32 = Color32::from_rgba_premultiplied(0xaa, 0xaa, 0xaa, 0xff);
        const COLOR_DATA: Color32 = Color32::from_rgba_premultiplied(0xdd, 0xdd, 0xee, 0xff);
        const COLOR_CODE: Color32 = Color32::from_rgba_premultiplied(0xee, 0xdd, 0xdd, 0xff);
        const COLOR_BRANCH_TARGET: Color32 = Color32::from_rgba_premultiplied(0xbb, 0xaa, 0xaa, 0xff);
        const COLOR_CODE_HEX: Color32 = Color32::from_rgba_premultiplied(0xdd, 0xcc, 0xcc, 0xff);
        const COLOR_DEBUG_NOTE: Color32 = Color32::from_rgba_premultiplied(0xee, 0xee, 0x55, 0xff);

        let project = ctx.project_ref.as_ref().unwrap().borrow();
        let disasm = &project.rom_data.disassembly;

        let str_buf = RefCell::new(String::with_capacity(256));

        let write_hex = |bytes: &mut dyn Iterator<Item = u8>| {
            let mut str_buf = str_buf.borrow_mut();
            str_buf.clear();
            let mut num_bytes = 0usize;
            for byte in bytes {
                write!(str_buf, "{:02X} ", byte).unwrap();
                num_bytes += 1;
            }
            (str_buf, num_bytes)
        };

        let curr_pc_addr_scroll = AddrPc::try_from_lorom(AddrSnes(self.current_address_scroll as usize)).unwrap().0;
        let first_block_idx = disasm.chunks.partition_point(|(a, _)| a.0 < curr_pc_addr_scroll).max(1) - 1;
        let mut current_address = curr_pc_addr_scroll;

        let row_height = tweak!(15.0);
        let header_height = tweak!(30.0);
        let total_rows = {
            let spacing = ui.spacing().item_spacing;
            ((ui.available_height() - header_height) / (row_height + spacing.y)) as _
        };

        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Min))
            .column(Size::exact(tweak!(90.0)))
            .column(Size::exact(tweak!(170.0)))
            .column(Size::exact(tweak!(250.0)))
            .column(Size::exact(tweak!(50.0)))
            .column(Size::exact(tweak!(70.0)))
            .header(header_height, |mut th| {
                th.col(|ui| {
                    ui.heading("Label");
                });
                th.col(|ui| {
                    ui.heading("Bytes");
                });
                th.col(|ui| {
                    ui.heading("Code");
                });
                th.col(|ui| {
                    ui.heading("A Size");
                });
                th.col(|ui| {
                    ui.heading("X&Y Size");
                });
            })
            .body(|mut tb| {
                let mut lines_drawn_so_far = 0;
                'draw_lines: for (chunk_idx, (chunk_pc, chunk)) in
                    disasm.chunks.iter().enumerate().skip(first_block_idx)
                {
                    if lines_drawn_so_far >= total_rows {
                        break 'draw_lines;
                    }

                    let chunk_pc = *chunk_pc;
                    let next_chunk_pc = disasm
                        .chunks
                        .get(chunk_idx + 1)
                        .map(|c| c.0)
                        .unwrap_or_else(|| AddrPc::from(disasm.rom_bytes().len()));
                    let chunk_bytes = &disasm.rom_bytes()[chunk_pc.0..next_chunk_pc.0];

                    match chunk {
                        BinaryBlock::EndOfRom => break 'draw_lines,
                        BinaryBlock::Unknown | BinaryBlock::Unused | BinaryBlock::Data(_) => {
                            let stride = 8;
                            let skip_lines = (current_address - chunk_pc.0) / stride;
                            let chunks = chunk_bytes.iter().copied().chunks(stride);
                            for (line_number, mut byte_line) in chunks.into_iter().enumerate().skip(skip_lines) {
                                let line_addr_str = {
                                    let pc = AddrPc(chunk_pc.0 + line_number * stride);
                                    let snes = AddrSnes::try_from_lorom(pc).unwrap();
                                    format!("DATA_{:06X}:", snes.0)
                                };

                                let (bytes_str, num_bytes) = write_hex(&mut byte_line);
                                current_address += num_bytes;

                                let data_str = {
                                    let mut s = String::with_capacity(40);
                                    write!(s, ".db ").unwrap();
                                    for byte in bytes_str.split(' ').filter(|s| !s.is_empty()) {
                                        write!(s, "${},", byte).unwrap();
                                    }
                                    s.pop().unwrap();
                                    s
                                };

                                tb.row(row_height, |mut tr| {
                                    tr.col(|ui| {
                                        ui.monospace(RichText::new(line_addr_str).color(COLOR_ADDRESS));
                                    });
                                    tr.col(|ui| {
                                        ui.monospace(RichText::new(bytes_str.deref()).color(COLOR_CODE_HEX));
                                    });
                                    tr.col(|ui| {
                                        ui.monospace(RichText::new(data_str).color(COLOR_DATA));
                                    });
                                    tr.col(|_ui| {});
                                    tr.col(|_ui| {});
                                });

                                lines_drawn_so_far += 1;
                                if lines_drawn_so_far >= total_rows {
                                    break 'draw_lines;
                                }
                            }
                        }
                        BinaryBlock::Code(code) => {
                            let first_instruction = code.instructions.partition_point(|i| i.offset.0 < current_address);

                            for ins in code.instructions.iter().copied().skip(first_instruction) {
                                let Instruction { offset: addr, x_flag, m_flag, .. } = ins;

                                let line_addr_str = format!("CODE_{:06X}:", AddrSnes::try_from_lorom(addr).unwrap().0);

                                let (code_bytes_str, num_bytes) = {
                                    let mut b_it = disasm
                                        .rom_bytes()
                                        .iter()
                                        .copied()
                                        .skip(addr.0)
                                        .take(ins.opcode.instruction_size());
                                    write_hex(&mut b_it)
                                };
                                current_address += num_bytes;

                                let code_str = format!("{}", ins.display());

                                tb.row(row_height, |mut tr| {
                                    tr.col(|ui| {
                                        ui.monospace(RichText::new(line_addr_str).color(COLOR_ADDRESS));
                                    });
                                    tr.col(|ui| {
                                        ui.monospace(RichText::new(code_bytes_str.deref()).color(COLOR_CODE_HEX));
                                    });
                                    tr.col(|ui| {
                                        ui.monospace(RichText::new(code_str).color(COLOR_CODE));
                                    });
                                    tr.col(|ui| {
                                        ui.monospace(
                                            RichText::new(format!("{}", 8 * (m_flag as u32 + 1))).color(COLOR_CODE),
                                        );
                                    });
                                    tr.col(|ui| {
                                        ui.monospace(
                                            RichText::new(format!("{}", 8 * (x_flag as u32 + 1))).color(COLOR_CODE),
                                        );
                                    });
                                });

                                lines_drawn_so_far += 1;
                                if lines_drawn_so_far >= total_rows {
                                    break 'draw_lines;
                                }
                            }
                        }
                    }
                    current_address = next_chunk_pc.0;
                }
            });
    }
}
