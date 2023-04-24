use wdc65816::{Cpu, Mem};
use std::collections::HashSet;
use crate::rom::Rom;

#[derive(Clone)]
pub struct CheckedMem<'a> {
    pub cart: &'a Rom,
    pub wram: Vec<u8>,
    pub regs: Vec<u8>,
    pub vram: Vec<u8>,
    pub extram: Vec<u8>,
    pub uninit: HashSet<usize>,
    pub error: Option<u32>,
    pub err_value: Option<u8>,
    pub last_store: Option<u32>,
}

impl<'a> CheckedMem<'a> {
    pub fn load_u16(&mut self, addr: u32) -> u16 {
        let l = self.load(addr);
        let h = self.load(addr + 1);
        u16::from_le_bytes([l,h])
    }
    pub fn load_u24(&mut self, addr: u32) -> u32 {
        let l = self.load(addr);
        let h = self.load(addr + 1);
        let b = self.load(addr + 2);
        u32::from_le_bytes([l,h,b,0])
    }
    pub fn store_u16(&mut self, addr: u32, val: u16) {
        let val = val.to_le_bytes();
        self.store(addr, val[0]);
        self.store(addr + 1, val[1]);
    }
    pub fn store_u24(&mut self, addr: u32, val: u32) {
        let val = val.to_le_bytes();
        self.store(addr, val[0]);
        self.store(addr + 1, val[1]);
        self.store(addr + 2, val[2]);
    }
    pub fn process_dma_ch(&mut self, ch: u32) {
        let a = self.load_u24(0x4302 + ch);
        let size = self.load_u16(0x4305 + ch) as u32;
        let b = self.load(0x4301 + ch);
        let params = self.load(0x4300 + ch);
        // TODO: turn this into reg writes
        if b == 0x18 {
            let dest = self.load_u16(0x2116) as u32;
            //println!("DMA size {:04X}: VRAM ${:02X}:{:04X} => ${:04X}", size, a_bank, a, dest);
            if params & 0x8 != 0 { // fill transfer
                /*let value = self.load(a_bank, a);
                for i in dest..dest+size {
                    self.vram[i as usize * 2] = value;
                }*/
            } else {
                for i in 0..size {
                    self.vram[(dest*2 + i) as usize] = self.load(a + i);
                }
            }
        } else if b == 0x19 {
            let _dest = self.load_u16(0x2116);
            //println!("DMA size {:04X}: VRAMh ${:02X}:{:04X} => ${:04X}", size, a_bank, a, dest);
            if params & 0x8 != 0 { // fill transfer
                /*let value = self.load(a_bank, a);
                for i in dest..dest+size {
                    self.vram[i as usize * 2] = value;
                }*/
            }
        } else {
            println!("DMA size {size:04X}: ${b:02X} ${a:06X}");
        }
    }
    pub fn process_dma(&mut self) {
        let dma = self.load(0x420B);
        if dma != 0 {
            for i in 0..8 {
                if dma & (1<<i) != 0 {
                    self.process_dma_ch(i * 0x10);
                }
            }
            self.store(0x420B, 0);
        }
    }
    pub fn map(&mut self, addr: u32, write: Option<u8>) -> u8 {
        let track_uninit = false;
        let bank = addr >> 16;
        let mutable = if bank & 0xFE == 0x7E {
            let ptr = (addr & 0x1FFFF) as usize;
            if track_uninit {
                if write.is_none() && !self.uninit.contains(&ptr) {
                    println!("Uninit read: ${:06X}", 0x7E0000 + ptr);
                }
                self.uninit.insert(ptr);
            }
            &mut self.wram[ptr]
        } else if bank == 0x60 {
            let ptr = (addr & 0xFFFF) as usize;
            &mut self.extram[ptr]
        } else if addr & 0xFFFF < 0x2000 {
            let ptr = (addr & 0x1FFF) as usize;
            if track_uninit {
                if write.is_none() && !self.uninit.contains(&ptr) {
                    println!("Uninit read: ${:06X}", 0x7E0000 + ptr);
                }
                self.uninit.insert(ptr);
            }
            &mut self.wram[ptr]
        } else if addr & 0xFFFF < 0x8000 {
            let ptr = (addr & 0x7FFF) as usize;
            if track_uninit {
                if write.is_none() && !self.uninit.contains(&ptr) {
                    //println!("Uninit read: ${:04X}", ptr);
                }
                self.uninit.insert(ptr);
            }
            // TODO: be more accurate
            if let Some(value) = write {
                if ptr == 0x2118 {
                    let addr = self.load_u16(0x2116);
                    self.vram[(addr as usize) * 2 + 0] = value;
                } else if ptr == 0x2119 {
                    let addr = self.load_u16(0x2116);
                    self.vram[(addr as usize) * 2 + 1] = value;
                    self.store_u16(0x2116, addr + 1);
                }
            }
            &mut self.regs[ptr-0x2000]
        } else if addr & 0xFFFF > 0x8000 {
            if let Some(c) = self.cart.read(addr) {
                return c;
            } else {
                self.error = Some(addr);
                self.err_value.get_or_insert(0)
            }
        } else {
            self.error = Some(addr);
            self.err_value.get_or_insert(0)
        };
        if let Some(c) = write {
            *mutable = c;
        }
        *mutable
    }
}
impl<'a> Mem for CheckedMem<'a> {
    fn load(&mut self, addr: u32) -> u8 {
        let value = self.map(addr, None);
        //println!("ld ${:06X} = {:02X}", addr, value);
        value
    }
    fn store(&mut self, addr: u32, value: u8) {
        //println!("st ${:06X} = {:02X}", addr, value);
        self.map(addr, Some(value));
        self.last_store = Some(addr);
    }
}

pub fn decompress_sublevel(cpu: &mut Cpu<CheckedMem>, id: u16) -> u64 {
    let now = std::time::Instant::now();
    cpu.emulation = false;
    // set submap
    cpu.mem.store(0x1F11, (id>>8) as _);
    cpu.s = 0x1FF;
    cpu.pc = 0x2000;
    cpu.pbr = 0x00;
    cpu.dbr = 0x00;
    cpu.trace = true;
    // quasi-loader bytecode
    cpu.mem.store(0x2000, 0x22);
    cpu.mem.store_u24(0x2001, cpu.mem.cart.resolve("CODE_05D796").unwrap());
    cpu.mem.store(0x2004, 0x22);
    cpu.mem.store_u24(0x2005, cpu.mem.cart.resolve("CODE_05801E").unwrap());
    cpu.mem.store(0x2008, 0x22);
    cpu.mem.store_u24(0x2009, cpu.mem.cart.resolve("UploadSpriteGFX").unwrap());
    cpu.mem.store(0x200C, 0x22);
    cpu.mem.store_u24(0x200D, cpu.mem.cart.resolve("CODE_00A993").unwrap());
    let mut cy = 0;
    loop {
        cy += cpu.dispatch() as u64;
        //if cy > cy_limit { break; }
        if cpu.ill {
            println!("ILLEGAL INSTR");
            break;
        }
        if cpu.pc == 0xD89F && cpu.pbr == 0x05 {
            cpu.a &= 0xFF00;
            cpu.a |= id & 0xFF;
        }
        if cpu.pc == 0x2010 { break; }
        cpu.mem.process_dma();
    }
    println!("took {}µs", now.elapsed().as_micros());
    cy
}
