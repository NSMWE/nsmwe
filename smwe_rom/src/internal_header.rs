use std::{clone::Clone, convert::TryFrom, fmt};

use nom::{
    bytes::complete::take,
    combinator::map_res,
    map,
    number::complete::{le_u16, le_u8},
    sequence::{pair, preceded},
    take_str,
};
use num_enum::{IntoPrimitive, TryFromPrimitive};

pub use self::address_spaces::*;
use crate::{
    addr::AddrPc,
    error::{ParseErr, RomParseError},
};

pub mod address_spaces {
    use crate::addr::{AddrPc, AddrSpacePc};
    pub const HEADER_LOROM: AddrSpacePc = AddrPc(0x007FC0)..=AddrPc(0x008000);
    pub const HEADER_HIROM: AddrSpacePc = AddrPc(0x00FFC0)..=AddrPc(0x010000);
}

#[rustfmt::skip]
pub mod offsets {
    pub const COMPLEMENT_CHECK: usize = 0x1C;
    pub const CHECKSUM:         usize = 0x1E;
}

#[rustfmt::skip]
pub mod sizes {
    pub const INTERNAL_HEADER:   usize = 32;
    pub const INTERNAL_ROM_NAME: usize = 21;
}

// -------------------------------------------------------------------------------------------------

pub struct RomInternalHeader {
    pub internal_rom_name: String,
    pub map_mode:          MapMode,
    pub rom_type:          RomType,
    pub rom_size:          u8,
    pub sram_size:         u8,
    pub region_code:       RegionCode,
    pub developer_id:      u8,
    pub version_number:    u8,
}

#[derive(Copy, Clone, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum MapMode {
    SlowLoRom   = 0b100000,
    SlowHiRom   = 0b100001,
    SlowExLoRom = 0b100010,
    SlowExHiRom = 0b100100,
    FastLoRom   = 0b110000,
    FastHiRom   = 0b110001,
    FastExLoRom = 0b110010,
    FastExHiRom = 0b110100,
}

#[derive(Copy, Clone, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum RomType {
    Rom               = 0x00,
    RomRam            = 0x01,
    RomRamSram        = 0x02,

    RomDsp            = 0x03,
    RomSuperFx        = 0x13,
    RomObc1           = 0x23,
    RomSa1            = 0x33,
    RomSdd1           = 0x43,
    RomSrtc           = 0x53,
    RomOther          = 0xE3,
    RomCustom         = 0xF3,

    RomDspRam         = 0x04,
    RomSuperFxRam     = 0x14,
    RomObc1Ram        = 0x24,
    RomSa1Ram         = 0x34,
    RomSdd1Ram        = 0x44,
    RomSRtcRam        = 0x54,
    RomOtherRam       = 0xE4,
    RomCustomRam      = 0xF4,

    RomDspRamSram     = 0x05,
    RomSuperFxRamSram = 0x15,
    RomObc1RamSram    = 0x25,
    RomSa1RamSram     = 0x35,
    RomSdd1RamSram    = 0x45,
    RomSRtcRamSram    = 0x55,
    RomOtherRamSram   = 0xE5,
    RomCustomRamSram  = 0xF5,

    RomDspSram        = 0x06,
    RomSuperFxSram    = 0x16,
    RomObc1Sram       = 0x26,
    RomSa1Sram        = 0x36,
    RomSdd1Sram       = 0x46,
    RomSRtcSram       = 0x56,
    RomOtherSram      = 0xE6,
    RomCustomSram     = 0xF6,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum RegionCode {
    Japan        = 0x00,
    NorthAmerica = 0x01,
    Europe       = 0x02,
    Sweden       = 0x03,
    Finland      = 0x04,
    Denmark      = 0x05,
    France       = 0x06,
    Netherlands  = 0x07,
    Spain        = 0x08,
    Germany      = 0x09,
    Italy        = 0x0A,
    China        = 0x0B,
    Indonesia    = 0x0C,
    Korea        = 0x0D,
    Global       = 0x0E,
    Canada       = 0x0F,
    Brazil       = 0x10,
    Australia    = 0x11,
    Other1       = 0x12,
    Other2       = 0x13,
    Other3       = 0x14,
}

// -------------------------------------------------------------------------------------------------

impl RomInternalHeader {
    pub fn parse(rom_data: &[u8]) -> Result<Self, RomParseError> {
        let begin: usize = RomInternalHeader::find(rom_data)?.into();
        let (input, _) = take(begin)(rom_data)
            .map_err(|_: ParseErr| RomParseError::InternalHeader("Isolating Internal ROM Header"))?;

        let (input, internal_rom_name) = map!(input, take_str!(sizes::INTERNAL_ROM_NAME), String::from)
            .map_err(|_: ParseErr| RomParseError::InternalHeader("Reading Internal ROM Name"))?;
        let (input, map_mode) = map_res(le_u8, MapMode::try_from)(input)
            .map_err(|_: ParseErr| RomParseError::InternalHeader("Reading Map Mode"))?;
        let (input, rom_type) = map_res(le_u8, RomType::try_from)(input)
            .map_err(|_: ParseErr| RomParseError::InternalHeader("Reading ROM Type"))?;
        let (input, rom_size) =
            le_u8(input).map_err(|_: ParseErr| RomParseError::InternalHeader("Reading ROM Size"))?;
        let (input, sram_size) =
            le_u8(input).map_err(|_: ParseErr| RomParseError::InternalHeader("Reading SRAM Size"))?;
        let (input, region_code) = map_res(le_u8, RegionCode::try_from)(input)
            .map_err(|_: ParseErr| RomParseError::InternalHeader("Reading Region Code"))?;
        let (input, developer_id) =
            le_u8(input).map_err(|_: ParseErr| RomParseError::InternalHeader("Reading Developer ID"))?;
        let (_, version_number) =
            le_u8(input).map_err(|_: ParseErr| RomParseError::InternalHeader("Reading Version Number"))?;

        Ok(Self {
            internal_rom_name,
            map_mode,
            rom_type,
            rom_size,
            sram_size,
            region_code,
            developer_id,
            version_number,
        })
    }

    fn find(rom_data: &[u8]) -> Result<AddrPc, RomParseError> {
        let lo_cpl_idx: usize = (*HEADER_LOROM.start() + offsets::COMPLEMENT_CHECK).into();
        let hi_cpl_idx: usize = (*HEADER_HIROM.start() + offsets::COMPLEMENT_CHECK).into();

        let (_, (lo_cpl, lo_csm)) =
            preceded(take(lo_cpl_idx), pair(le_u16, le_u16))(rom_data).map_err(|_: ParseErr| {
                RomParseError::InternalHeader("Reading checksum and complement at LoROM location")
            })?;
        let (_, (hi_cpl, hi_csm)) =
            preceded(take(hi_cpl_idx), pair(le_u16, le_u16))(rom_data).map_err(|_: ParseErr| {
                RomParseError::InternalHeader("Reading checksum and complement at HiROM location")
            })?;

        if (lo_csm ^ lo_cpl) == 0xFFFF {
            log::info!("Internal ROM header found at LoROM location: {:#X}", *HEADER_LOROM.start());
            Ok(*HEADER_LOROM.start())
        } else if (hi_csm ^ hi_cpl) == 0xFFFF {
            log::info!("Internal ROM header found at HiROM location: {:#X}", *HEADER_HIROM.start());
            Ok(*HEADER_HIROM.start())
        } else {
            log::error!("Couldn't find internal ROM header due to invalid checksums");
            log::error!("(LoROM: {:X}^{:X}, HiROM: {:X}^{:X})", lo_cpl, lo_csm, hi_cpl, hi_csm);
            Err(RomParseError::InternalHeader("Couldn't find internal ROM header"))
        }
    }

    pub fn rom_size_in_kb(&self) -> u32 {
        let exponent = self.rom_size as u32;
        2u32.pow(exponent)
    }

    pub fn sram_size_in_kb(&self) -> u32 {
        match self.sram_size as u32 {
            0 => 0,
            exponent => 2u32.pow(exponent),
        }
    }
}

impl fmt::Display for MapMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use MapMode::*;
        write!(f, "{}", match self {
            SlowLoRom => "LoROM",
            SlowHiRom => "HiROM",
            SlowExLoRom => "ExLoROM",
            SlowExHiRom => "ExHiROM",
            FastLoRom => "Fast LoROM",
            FastHiRom => "Fast HiROM",
            FastExLoRom => "Fast ExLoROM",
            FastExHiRom => "Fast ExHiROM",
        })
    }
}

#[rustfmt::skip]
impl MapMode {
    pub fn as_u8(&self) -> u8 { (*self).into() }
    pub fn is_slow(&self)    -> bool { (self.as_u8() & 0b010000) == 0 }
    pub fn is_fast(&self)    -> bool { !self.is_slow() }
    pub fn is_lorom(&self)   -> bool { (self.as_u8() & 0b000001) == 0 }
    pub fn is_hirom(&self)   -> bool { (self.as_u8() & 0b000001) != 0 }
    pub fn is_exlorom(&self) -> bool { (self.as_u8() & 0b000010) != 0 }
    pub fn is_exhirom(&self) -> bool { (self.as_u8() & 0b000100) != 0 }
}

impl fmt::Display for RomType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use RomType::*;
        let self_as_byte: u8 = (*self).into();
        write!(f, "{}", match self {
            Rom => String::from("ROM"),
            RomRam => String::from("ROM + RAM"),
            RomRamSram => String::from("ROM + RAM + SRAM"),
            _ => format!("ROM + {}", {
                let coprocessor = match self_as_byte & 0xF0 {
                    0x00 => "DSP",
                    0x10 => "SuperFX",
                    0x20 => "OBC-1",
                    0x30 => "SA-1",
                    0x40 => "SDD-1",
                    0x50 => "S-RTC",
                    0xE0 => "Other expansion chip",
                    0xF0 => "Custom expansion chip",
                    _ => "Unknown expansion chip",
                };
                let memory = self_as_byte & 0xF;
                if memory == 0x3 {
                    coprocessor.to_string()
                } else {
                    format!("{} + {}", coprocessor, match memory {
                        0x4 => "RAM",
                        0x5 => "RAM + SRAM",
                        0x6 => "SRAM",
                        _ => "Unknown memory chip",
                    })
                }
            }),
        })
    }
}

impl fmt::Display for RegionCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use RegionCode::*;
        write!(f, "{}", match self {
            Japan => "Japan",
            NorthAmerica => "North America",
            Europe => "Europe",
            Sweden => "Sweden",
            Finland => "Finland",
            Denmark => "Denmark",
            France => "France",
            Netherlands => "Netherlands",
            Spain => "Spain",
            Germany => "Germany",
            Italy => "Italy",
            China => "China",
            Indonesia => "Indonesia",
            Korea => "Korea",
            Global => "Global",
            Canada => "Canada",
            Brazil => "Brazil",
            Australia => "Australia",
            Other1 => "Other (1)",
            Other2 => "Other (2)",
            Other3 => "Other (3)",
        })
    }
}
