extern crate base64;
extern crate flate2;
extern crate xml;

use std::str::FromStr;
use std::fs::File;
use std::io::Read;
use std::path::Path;

mod error;
pub use error::{ParseTileError, TiledError};

mod macros;

mod properties;
pub use properties::{Properties, PropertyValue};
use properties::parse_properties;

mod map;
pub use map::Map;

mod tileset;
pub use tileset::Tileset;

mod tile;
pub use tile::Tile;

mod image;
pub use image::Image;

mod layer;
pub use layer::{ImageLayer, Layer};

mod object;
pub use object::{Object, ObjectGroup, ObjectShape};

mod frame;
pub use frame::Frame;

mod parse;
use parse::{parse_animation, parse_data, parse_impl};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Colour {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl FromStr for Colour {
    type Err = ParseTileError;

    fn from_str(s: &str) -> Result<Colour, ParseTileError> {
        let s = if s.starts_with("#") { &s[1..] } else { s };
        if s.len() != 6 {
            return Err(ParseTileError::ColourError);
        }
        let r = u8::from_str_radix(&s[0..2], 16);
        let g = u8::from_str_radix(&s[2..4], 16);
        let b = u8::from_str_radix(&s[4..6], 16);
        if r.is_ok() && g.is_ok() && b.is_ok() {
            return Ok(Colour {
                red: r.unwrap(),
                green: g.unwrap(),
                blue: b.unwrap(),
            });
        }
        Err(ParseTileError::ColourError)
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Orientation {
    Orthogonal,
    Isometric,
    Staggered,
    Hexagonal,
}

impl FromStr for Orientation {
    type Err = ParseTileError;

    fn from_str(s: &str) -> Result<Orientation, ParseTileError> {
        match s {
            "orthogonal" => Ok(Orientation::Orthogonal),
            "isometric" => Ok(Orientation::Isometric),
            "staggered" => Ok(Orientation::Staggered),
            "hexagonal" => Ok(Orientation::Hexagonal),
            _ => Err(ParseTileError::OrientationError),
        }
    }
}

/// Parse a file hopefully containing a Tiled map and try to parse it.  If the
/// file has an external tileset, the tileset file will be loaded using a path
/// relative to the map file's path.
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Map, TiledError> {
    let file = File::open(path.as_ref())
        .map_err(|_| TiledError::Other(format!("Map file not found: {:?}", path.as_ref())))?;
    parse_impl(file, Some(path))
}

/// Parse a buffer hopefully containing the contents of a Tiled file and try to
/// parse it.
pub fn parse<R: Read>(reader: R) -> Result<Map, TiledError> {
    parse_impl::<_, &Path>(reader, None)
}

/// Parse a buffer hopefully containing the contents of a Tiled tileset.
///
/// External tilesets do not have a firstgid attribute.  That lives in the
/// map. You must pass in `first_gid`.  If you do not need to use gids for anything,
/// passing in 1 will work fine.
pub fn parse_tileset<R: Read>(reader: R, first_gid: u32) -> Result<Tileset, TiledError> {
    Tileset::new_external(reader, first_gid)
}
