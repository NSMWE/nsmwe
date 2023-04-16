#![allow(clippy::identity_op)]

pub mod compression;
pub mod disassembler;
pub mod graphics;
pub mod internal_header;
pub mod level;
pub mod objects;
pub mod snes_utils;

use std::{fs, path::Path};

use thiserror::Error;

pub use crate::internal_header::{RegionCode, RomInternalHeader};
use crate::{
    disassembler::{
        binary_block::{DataBlock, DataKind},
        RomDisassembly,
    },
    graphics::{gfx_file::GfxFileParseError, palette::ColorPaletteParseError, Gfx},
    internal_header::InternalHeaderParseError,
    level::{
        secondary_entrance::{SecondaryEntrance, SECONDARY_ENTRANCE_TABLE},
        Level,
        LevelParseError,
        LEVEL_COUNT,
    },
    objects::tilesets::{TilesetParseError, Tilesets},
    snes_utils::{
        addr::AddrSnes,
        rom::{Rom, RomError},
        rom_slice::SnesSlice,
    },
};

// -------------------------------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum RomParseError {
    #[error("ROM error:\n- {0}")]
    BadRom(RomError),
    #[error("Invalid GFX file {0:X}:\n- {1}")]
    GfxFile(usize, GfxFileParseError),
    #[error("Parsing internal header failed:\n- {0}")]
    InternalHeader(InternalHeaderParseError),
    #[error("File IO Error")]
    IoError,
    #[error("Failed to parse level {0:#X}:\n- {1}")]
    Level(u32, LevelParseError),
    #[error("Failed to read secondary entrance {0:#X}:\n- {1}")]
    SecondaryEntrance(usize, RomError),
    #[error("Could not parse color palettes:\n- {0}")]
    ColorPalettes(ColorPaletteParseError),
    #[error("Could not parse Map16 tiles:\n- {0}")]
    Map16Tilesets(TilesetParseError),
}

// -------------------------------------------------------------------------------------------------

pub struct SmwRom {
    pub disassembly:         RomDisassembly,
    pub internal_header:     RomInternalHeader,
    pub levels:              Vec<Level>,
    pub secondary_entrances: Vec<SecondaryEntrance>,
    pub gfx:                 Gfx,
    pub map16_tilesets:      Tilesets,
}

// -------------------------------------------------------------------------------------------------

impl SmwRom {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, RomParseError> {
        log::info!("Reading ROM from file: {}", path.as_ref().display());
        let smw_rom = fs::read(path)
            .map_err(|err| {
                log::error!("Could not read ROM: {}", err);
                RomParseError::IoError
            })
            .and_then(|rom_data| Rom::new(rom_data).map_err(RomParseError::BadRom))
            .and_then(Self::from_rom);

        if smw_rom.is_ok() {
            log::info!("Success parsing ROM");
        }

        smw_rom
    }

    pub fn from_rom(rom: Rom) -> Result<Self, RomParseError> {
        log::info!("Parsing internal ROM header");
        let internal_header = RomInternalHeader::parse(&rom).map_err(RomParseError::InternalHeader)?;

        log::info!("Creating disassembly map");
        let mut disassembly = RomDisassembly::new(rom, &internal_header);

        // Mark IRH
        disassembly.rom_slice_at_block(
            DataBlock {
                slice: SnesSlice::new(AddrSnes(0x00FFC0), internal_header::sizes::INTERNAL_HEADER),
                kind:  DataKind::InternalRomHeader,
            },
            |_| RomParseError::InternalHeader(InternalHeaderParseError::NotFound),
        )?;

        log::info!("Parsing level data");
        let levels = Self::parse_levels(&mut disassembly)?;

        log::info!("Parsing secondary entrances");
        let secondary_entrances = Self::parse_secondary_entrances(&mut disassembly)?;

        log::info!("Parsing GFX files");
        let gfx = Gfx::parse(&mut disassembly, &levels, &internal_header)?;

        log::info!("Parsing Map16 tilesets");
        let map16_tilesets = Tilesets::parse(&mut disassembly).map_err(RomParseError::Map16Tilesets)?;

        Ok(Self { disassembly, internal_header, levels, secondary_entrances, gfx, map16_tilesets })
    }

    fn parse_levels(disasm: &mut RomDisassembly) -> Result<Vec<Level>, RomParseError> {
        let mut levels = Vec::with_capacity(LEVEL_COUNT);
        for level_num in 0..LEVEL_COUNT as u32 {
            let level = Level::parse(disasm, level_num).map_err(|e| RomParseError::Level(level_num, e))?;
            levels.push(level);
        }
        Ok(levels)
    }

    fn parse_secondary_entrances(disasm: &mut RomDisassembly) -> Result<Vec<SecondaryEntrance>, RomParseError> {
        let mut secondary_entrances = Vec::with_capacity(SECONDARY_ENTRANCE_TABLE.size);
        for entrance_id in 0..SECONDARY_ENTRANCE_TABLE.size {
            let entrance = SecondaryEntrance::read_from_rom(disasm, entrance_id)
                .map_err(|e| RomParseError::SecondaryEntrance(entrance_id, e))?;
            secondary_entrances.push(entrance);
        }
        Ok(secondary_entrances)
    }
}
