use nom::{combinator::map, multi::many0, number::complete::le_u16};
use thiserror::Error;

use crate::{
    disassembler::{
        binary_block::{DataBlock, DataKind},
        RomDisassembly,
    },
    objects::map16::{Map16Tile, Tile8x8},
    snes_utils::{
        addr::{AddrSnes, AddrVram},
        rom_slice::SnesSlice,
    },
};

// -------------------------------------------------------------------------------------------------

#[derive(Debug, Error)]
#[error("Could not parse AnimatedTileData table.")]
pub struct AnimatedTileDataParseError;

// -------------------------------------------------------------------------------------------------

const ANIM_SRC_ADDRESSES_TABLE: SnesSlice = SnesSlice::new(AddrSnes(0x05B999), 416);
const ANIM_DST_ADDRESSES_TABLE: SnesSlice = SnesSlice::new(AddrSnes(0x05B93B), 48);
const ANIM_BEHAVIOUR_TABLE: SnesSlice = SnesSlice::new(AddrSnes(0x05B96B), 46);

// -------------------------------------------------------------------------------------------------

pub struct AnimatedTileData {
    pub src_addresses: Vec<AddrSnes>,
    pub dst_addresses: Vec<AddrVram>,
    pub behaviours:    Vec<u8>,
    pub switches:      Vec<u8>,
    pub tilesets:      Vec<u8>,
}

impl AnimatedTileData {
    pub fn parse(disasm: &mut RomDisassembly) -> anyhow::Result<Self> {
        let src_addresses = {
            let data_block = DataBlock { slice: ANIM_SRC_ADDRESSES_TABLE, kind: DataKind::AnimatedTileData };
            disasm
                .rom_slice_at_block(data_block, |_| AnimatedTileDataParseError)?
                .parse(many0(map(le_u16, |a| AddrSnes(a as _).with_bank(0x7E))))?
        };
        let dst_addresses = {
            let data_block = DataBlock { slice: ANIM_DST_ADDRESSES_TABLE, kind: DataKind::AnimatedTileData };
            disasm
                .rom_slice_at_block(data_block, |_| AnimatedTileDataParseError)?
                .parse(many0(map(le_u16, AddrVram)))?
        };
        let (behaviours, switches, tilesets) = {
            let data_block = DataBlock { slice: ANIM_BEHAVIOUR_TABLE, kind: DataKind::AnimatedTileData };
            let bytes = disasm.rom_slice_at_block(data_block, |_| AnimatedTileDataParseError)?.as_bytes()?;
            (bytes[..24].to_vec(), bytes[18..18 + 15].to_vec(), bytes[32..32 + 14].to_vec())
        };
        Ok(Self { src_addresses, dst_addresses, behaviours, switches, tilesets })
    }

    pub fn is_tile_animated(&self, tile: &Tile8x8) -> bool {
        self.dst_addresses.contains(&tile.tile_vram_addr())
    }

    pub fn get_animation_frames_for_block(
        &self, block: &Map16Tile, tileset: usize, blue_pswitch: bool, silver_pswitch: bool, on_off_switch: bool,
    ) -> Option<[AddrSnes; 4]> {
        let vram_addr = block.upper_left.tile_vram_addr();
        let dst_index = self.dst_addresses.iter().position(|&addr| addr == vram_addr)?;
        let src_index = {
            let i = match self.behaviours[dst_index] {
                0 => dst_index,
                1 => {
                    let switch_state = match self.switches[dst_index] {
                        0 => blue_pswitch,
                        1 => silver_pswitch,
                        2 => on_off_switch,
                        _ => unreachable!(),
                    };
                    if switch_state {
                        dst_index + 0x26
                    } else {
                        dst_index
                    }
                }
                2 => dst_index + self.tilesets[tileset] as usize,
                _ => unreachable!(),
            };
            ((i as u16 & 0xFF) << 3) as usize
        };

        Some([
            self.src_addresses[src_index + 0],
            self.src_addresses[src_index + 1],
            self.src_addresses[src_index + 2],
            self.src_addresses[src_index + 3],
        ])
    }
}
