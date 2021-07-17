use std::{
    convert::TryInto,
    fmt,
    fmt::{Display, Formatter},
};

use nom::{bytes::complete::take, combinator::map_parser, multi::count, IResult};

use crate::{
    compression::lc_lz2,
    error::{GfxFileParseError, ParseErr},
    graphics::color::{Abgr1555, Rgba32},
    snes_utils::{addr::AddrSnes, rom::Rom, rom_slice::SnesSlice},
};

#[derive(Copy, Clone, Debug)]
pub enum TileFormat {
    Tile2bpp,
    Tile4bpp,
    Tile8bpp,
    TileMode7,
}

#[derive(Clone)]
pub struct Tile {
    color_indices: Box<[u8]>,
}

#[derive(Clone)]
pub struct GfxFile {
    pub tile_format: TileFormat,
    pub tiles:       Vec<Tile>,
}

// -------------------------------------------------------------------------------------------------

impl Display for TileFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use TileFormat::*;
        f.write_str(match self {
            Tile2bpp => "2BPP",
            Tile4bpp => "4BPP",
            Tile8bpp => "8BPP",
            TileMode7 => "Mode7",
        })
    }
}

impl Tile {
    pub fn from_2bpp(input: &[u8]) -> IResult<&[u8], Self> {
        Self::from_xbpp(input, 2)
    }

    pub fn from_4bpp(input: &[u8]) -> IResult<&[u8], Self> {
        Self::from_xbpp(input, 4)
    }

    pub fn from_8bpp(input: &[u8]) -> IResult<&[u8], Self> {
        Self::from_xbpp(input, 8)
    }

    fn from_xbpp(input: &[u8], x: usize) -> IResult<&[u8], Self> {
        debug_assert!([2, 4, 8].contains(&x));
        let (input, bytes) = take(x * 8)(input)?;
        let mut tile = Tile { color_indices: [0; N_PIXELS_IN_TILE].into() };

        for i in 0..N_PIXELS_IN_TILE {
            let (row, col) = (i / 8, 7 - (i % 8));
            let mut color_idx = 0;
            for bit_idx in 0..x {
                let byte_idx = (2 * row) + (16 * (bit_idx / 2)) + (bit_idx % 2);
                let color_idx_bit = (bytes[byte_idx] >> col) & 1;
                color_idx |= color_idx_bit << bit_idx;
            }
            tile.color_indices[i] = color_idx;
        }

        Ok((input, tile))
    }

    pub fn from_mode7(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, bytes) = take(64usize)(input)?;
        let tile = Tile { color_indices: bytes.try_into().unwrap() };
        Ok((input, tile))
    }

    pub fn to_bgr555(&self, palette: &[Abgr1555]) -> Box<[Abgr1555]> {
        self.color_indices
            .iter()
            .copied()
            .map(|color_index| palette.get(color_index as usize).copied().unwrap_or(Abgr1555::MAGENTA))
            .collect()
    }

    pub fn to_rgba(&self, palette: &[Abgr1555]) -> Box<[Rgba32]> {
        self.to_bgr555(palette).iter().copied().map(Rgba32::from).collect()
    }
}

impl GfxFile {
    pub fn new(rom: &Rom, file_num: usize) -> Result<Self, GfxFileParseError> {
        debug_assert!(file_num < GFX_FILES_META.len());

        use TileFormat::*;
        type ParserFn = fn(&[u8]) -> IResult<&[u8], Tile>;

        let (tile_format, slice) = GFX_FILES_META[file_num];
        let (parser, tile_size_bytes): (ParserFn, usize) = match tile_format {
            Tile2bpp => (Tile::from_2bpp, 2 * 8),
            Tile4bpp => (Tile::from_4bpp, 4 * 8),
            Tile8bpp => (Tile::from_8bpp, 8 * 8),
            TileMode7 => (Tile::from_mode7, 8 * 8),
        };

        let bytes = rom.slice_lorom(slice).map_err(GfxFileParseError::IsolatingData)?;
        let decomp_bytes = lc_lz2::decompress(bytes).map_err(GfxFileParseError::DecompressingData)?;
        assert_eq!(0, decomp_bytes.len() % tile_size_bytes);

        if file_num == 0 {
            for (i, byte) in bytes.iter().enumerate() {
                print!("{:08b}{}", byte, if i % 8 == 7 { '\n' } else { ' ' }); // 47
            }
        }

        let tile_count = decomp_bytes.len() / tile_size_bytes;
        let mut read_tiles = count(map_parser(take(tile_size_bytes), parser), tile_count);

        let (_, tiles) = read_tiles(&decomp_bytes).map_err(|_: ParseErr| GfxFileParseError::ParsingTile)?;
        Ok(Self { tile_format, tiles })
    }

    pub fn n_pixels(&self) -> usize {
        self.tiles.len() * N_PIXELS_IN_TILE
    }
}

// -------------------------------------------------------------------------------------------------

pub const N_PIXELS_IN_TILE: usize = 8 * 8;
#[rustfmt::skip]
pub(crate) static GFX_FILES_META: [(TileFormat, SnesSlice); 0x34] = [
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x08D9F9), 2104)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x08E231), 2698)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x08ECBB), 2199)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x08F552), 2603)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x08FF7D), 2534)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x098963), 2569)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09936C), 2468)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x099D10), 2375)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09A657), 2378)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09AFA1), 2676)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09BA15), 2439)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09C39C), 2503)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09CD63), 2159)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09D5D2), 2041)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09DDCB), 2330)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09E6E5), 2105)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09EF1E), 2193)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09F7AF), 2062)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x09FFBD), 2387)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0A8910), 2616)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0A9348), 1952)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0A9AE8), 2188)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0AA374), 1600)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0AA9B4), 2297)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0AB2AD), 2359)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0ABBE4), 1948)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0AC380), 2278)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0ACC66), 2072)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0AD47E), 2058)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0ADC88), 2551)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0AE67F), 1988)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0AEE43), 2142)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0AF6A1), 2244)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0AFF65), 2408)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0B88CD), 2301)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0B91CA), 2331)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0B9AE5), 2256)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0BA3B5), 2668)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0BAE21), 2339)),
    (TileFormat::TileMode7, SnesSlice::new(AddrSnes(0x0BB744), 2344)),
    (TileFormat::Tile2bpp,  SnesSlice::new(AddrSnes(0x0BC06C), 1591)),
    (TileFormat::Tile2bpp,  SnesSlice::new(AddrSnes(0x0BC6A3), 1240)),
    (TileFormat::Tile2bpp,  SnesSlice::new(AddrSnes(0x0BCB7B), 1397)),
    (TileFormat::Tile2bpp,  SnesSlice::new(AddrSnes(0x0BD0F0), 1737)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0BD7B9), 2125)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0BE006), 2352)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0BE936), 2127)),
    (TileFormat::Tile2bpp,  SnesSlice::new(AddrSnes(0x0BF185), 566)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0BF3BB), 1093)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x0BF800), 1293)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x088000), 16320)),
    (TileFormat::Tile4bpp,  SnesSlice::new(AddrSnes(0x08BFC0), 6713)),
];
