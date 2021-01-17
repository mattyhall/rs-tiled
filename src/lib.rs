use base64;

use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Error, Read};
use std::path::Path;
use std::str::FromStr;
use xml::attribute::OwnedAttribute;
use xml::reader::XmlEvent;
use xml::reader::{Error as XmlError, EventReader};

#[derive(Debug, Copy, Clone)]
pub enum ParseTileError {
    ColourError,
    OrientationError,
}

// Loops through the attributes once and pulls out the ones we ask it to. It
// will check that the required ones are there. This could have been done with
// attrs.find but that would be inefficient.
//
// This is probably a really terrible way to do this. It does cut down on lines
// though which is nice.
macro_rules! get_attrs {
    ($attrs:expr, optionals: [$(($oName:pat, $oVar:ident, $oMethod:expr)),* $(,)*],
     required: [$(($name:pat, $var:ident, $method:expr)),* $(,)*], $err:expr) => {
        {
            $(let mut $oVar = None;)*
            $(let mut $var = None;)*
            for attr in $attrs.iter() {
                match attr.name.local_name.as_ref() {
                    $($oName => $oVar = $oMethod(attr.value.clone()),)*
                    $($name => $var = $method(attr.value.clone()),)*
                    _ => {}
                }
            }
            if !(true $(&& $var.is_some())*) {
                return Err($err);
            }
            (($($oVar),*), ($($var.unwrap()),*))
        }
    }
}

// Goes through the children of the tag and will call the correct function for
// that child. Closes the tag
//
// Not quite as bad.
macro_rules! parse_tag {
    ($parser:expr, $close_tag:expr, {$($open_tag:expr => $open_method:expr),* $(,)*}) => {
        loop {
            match $parser.next().map_err(TiledError::XmlDecodingError)? {
                XmlEvent::StartElement {name, attributes, ..} => {
                    if false {}
                    $(else if name.local_name == $open_tag {
                        match $open_method(attributes) {
                            Ok(()) => {},
                            Err(e) => return Err(e)
                        };
                    })*
                }
                XmlEvent::EndElement {name, ..} => {
                    if name.local_name == $close_tag {
                        break;
                    }
                }
                XmlEvent::EndDocument => return Err(TiledError::PrematureEnd("Document ended before we expected.".to_string())),
                _ => {}
            }
        }
    }
}

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

/// Errors which occured when parsing the file
#[derive(Debug)]
pub enum TiledError {
    /// A attribute was missing, had the wrong type of wasn't formated
    /// correctly.
    MalformedAttributes(String),
    /// An error occured when decompressing using the
    /// [flate2](https://github.com/alexcrichton/flate2-rs) crate.
    DecompressingError(Error),
    Base64DecodingError(base64::DecodeError),
    XmlDecodingError(XmlError),
    PrematureEnd(String),
    Other(String),
}

impl fmt::Display for TiledError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match *self {
            TiledError::MalformedAttributes(ref s) => write!(fmt, "{}", s),
            TiledError::DecompressingError(ref e) => write!(fmt, "{}", e),
            TiledError::Base64DecodingError(ref e) => write!(fmt, "{}", e),
            TiledError::XmlDecodingError(ref e) => write!(fmt, "{}", e),
            TiledError::PrematureEnd(ref e) => write!(fmt, "{}", e),
            TiledError::Other(ref s) => write!(fmt, "{}", s),
        }
    }
}

// This is a skeleton implementation, which should probably be extended in the future.
impl std::error::Error for TiledError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            TiledError::MalformedAttributes(_) => None,
            TiledError::DecompressingError(ref e) => Some(e as &dyn std::error::Error),
            TiledError::Base64DecodingError(ref e) => Some(e as &dyn std::error::Error),
            TiledError::XmlDecodingError(ref e) => Some(e as &dyn std::error::Error),
            TiledError::PrematureEnd(_) => None,
            TiledError::Other(_) => None,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum PropertyValue {
    BoolValue(bool),
    FloatValue(f32),
    IntValue(i32),
    ColorValue(u32),
    StringValue(String),
}

impl PropertyValue {
    fn new(property_type: String, value: String) -> Result<PropertyValue, TiledError> {
        // Check the property type against the value.
        match property_type.as_str() {
            "bool" => match value.parse() {
                Ok(val) => Ok(PropertyValue::BoolValue(val)),
                Err(err) => Err(TiledError::Other(err.to_string())),
            },
            "float" => match value.parse() {
                Ok(val) => Ok(PropertyValue::FloatValue(val)),
                Err(err) => Err(TiledError::Other(err.to_string())),
            },
            "int" => match value.parse() {
                Ok(val) => Ok(PropertyValue::IntValue(val)),
                Err(err) => Err(TiledError::Other(err.to_string())),
            },
            "color" if value.len() > 1 => match u32::from_str_radix(&value[1..], 16) {
                Ok(color) => Ok(PropertyValue::ColorValue(color)),
                Err(_) => Err(TiledError::Other(format!(
                    "Improperly formatted color property"
                ))),
            },
            "string" => Ok(PropertyValue::StringValue(value)),
            _ => Err(TiledError::Other(format!(
                "Unknown property type \"{}\"",
                property_type
            ))),
        }
    }
}

pub type Properties = HashMap<String, PropertyValue>;

fn parse_properties<R: Read>(parser: &mut EventReader<R>) -> Result<Properties, TiledError> {
    let mut p = HashMap::new();
    parse_tag!(parser, "properties", {
        "property" => |attrs:Vec<OwnedAttribute>| {
            let (t, (k, v)) = get_attrs!(
                attrs,
                optionals: [
                    ("type", property_type, |v| Some(v)),
                ],
                required: [
                    ("name", key, |v| Some(v)),
                    ("value", value, |v| Some(v)),
                ],
                TiledError::MalformedAttributes("property must have a name and a value".to_string())
            );
            let t = t.unwrap_or("string".into());

            p.insert(k, PropertyValue::new(t, v)?);
            Ok(())
        },
    });
    Ok(p)
}

/// All Tiled files will be parsed into this. Holds all the layers and tilesets
#[derive(Debug, PartialEq, Clone)]
pub struct Map {
    pub version: String,
    pub orientation: Orientation,
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub tilesets: Vec<Tileset>,
    pub layers: Vec<Layer>,
    pub image_layers: Vec<ImageLayer>,
    pub object_groups: Vec<ObjectGroup>,
    pub properties: Properties,
    pub background_colour: Option<Colour>,
    pub infinite: bool,
}

impl Map {
    fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
        map_path: Option<&Path>,
    ) -> Result<Map, TiledError> {
        let ((c, infinite), (v, o, w, h, tw, th)) = get_attrs!(
            attrs,
            optionals: [
                ("backgroundcolor", colour, |v:String| v.parse().ok()),
                ("infinite", infinite, |v:String| Some(v == "1")),
            ],
            required: [
                ("version", version, |v| Some(v)),
                ("orientation", orientation, |v:String| v.parse().ok()),
                ("width", width, |v:String| v.parse().ok()),
                ("height", height, |v:String| v.parse().ok()),
                ("tilewidth", tile_width, |v:String| v.parse().ok()),
                ("tileheight", tile_height, |v:String| v.parse().ok()),
            ],
            TiledError::MalformedAttributes("map must have a version, width and height with correct types".to_string())
        );

        let mut tilesets = Vec::new();
        let mut layers = Vec::new();
        let mut image_layers = Vec::new();
        let mut properties = HashMap::new();
        let mut object_groups = Vec::new();
        let mut layer_index = 0;
        parse_tag!(parser, "map", {
            "tileset" => | attrs| {
                tilesets.push(Tileset::new(parser, attrs, map_path)?);
                Ok(())
            },
            "layer" => |attrs| {
                layers.push(Layer::new(parser, attrs, w, layer_index, infinite.unwrap_or(false))?);
                layer_index += 1;
                Ok(())
            },
            "imagelayer" => |attrs| {
                image_layers.push(ImageLayer::new(parser, attrs, layer_index)?);
                layer_index += 1;
                Ok(())
            },
            "properties" => |_| {
                properties = parse_properties(parser)?;
                Ok(())
            },
            "objectgroup" => |attrs| {
                object_groups.push(ObjectGroup::new(parser, attrs, Some(layer_index))?);
                layer_index += 1;
                Ok(())
            },
        });
        Ok(Map {
            version: v,
            orientation: o,
            width: w,
            height: h,
            tile_width: tw,
            tile_height: th,
            tilesets,
            layers,
            image_layers,
            object_groups,
            properties,
            background_colour: c,
            infinite: infinite.unwrap_or(false)
        })
    }

    /// This function will return the correct Tileset given a GID.
    pub fn get_tileset_by_gid(&self, gid: u32) -> Option<&Tileset> {
        let mut maximum_gid: i32 = -1;
        let mut maximum_ts = None;
        for tileset in self.tilesets.iter() {
            if tileset.first_gid as i32 > maximum_gid && tileset.first_gid <= gid {
                maximum_gid = tileset.first_gid as i32;
                maximum_ts = Some(tileset);
            }
        }
        maximum_ts
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

/// A tileset, usually the tilesheet image.
#[derive(Debug, PartialEq, Clone)]
pub struct Tileset {
    /// The GID of the first tile stored
    pub first_gid: u32,
    pub name: String,
    pub tile_width: u32,
    pub tile_height: u32,
    pub spacing: u32,
    pub margin: u32,
    pub tilecount: Option<u32>,
    pub columns: Option<u32>,
    /// The Tiled spec says that a tileset can have mutliple images so a `Vec`
    /// is used. Usually you will only use one.
    pub images: Vec<Image>,
    pub terrain_types: Option<TerrainTypes>,
    pub tiles: Vec<Tile>,
    pub properties: Properties,
}

impl Tileset {
    fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
        map_path: Option<&Path>,
    ) -> Result<Tileset, TiledError> {
        Tileset::new_internal(parser, &attrs).or_else(|_| Tileset::new_reference(&attrs, map_path))
    }

    fn new_internal<R: Read>(
        parser: &mut EventReader<R>,
        attrs: &Vec<OwnedAttribute>,
    ) -> Result<Tileset, TiledError> {
        let ((spacing, margin, tilecount, columns), (first_gid, name, width, height)) = get_attrs!(
           attrs,
           optionals: [
                ("spacing", spacing, |v:String| v.parse().ok()),
                ("margin", margin, |v:String| v.parse().ok()),
                ("tilecount", tilecount, |v:String| v.parse().ok()),
                ("columns", columns, |v:String| v.parse().ok()),
            ],
           required: [
                ("firstgid", first_gid, |v:String| v.parse().ok()),
                ("name", name, |v| Some(v)),
                ("tilewidth", width, |v:String| v.parse().ok()),
                ("tileheight", height, |v:String| v.parse().ok()),
            ],
            TiledError::MalformedAttributes("tileset must have a firstgid, name tile width and height with correct types".to_string())
        );

        let mut images = Vec::new();
        let mut terrain_types = None;
        let mut tiles = Vec::new();
        let mut properties = HashMap::new();
        parse_tag!(parser, "tileset", {
            "image" => |attrs| {
                images.push(Image::new(parser, attrs)?);
                Ok(())
            },
            "terraintypes" => |attrs| {
                terrain_types = Some(TerrainTypes::new(parser, attrs)?);
                Ok(())
            },
            "properties" => |_| {
                properties = parse_properties(parser)?;
                Ok(())
            },
            "tile" => |attrs| {
                tiles.push(Tile::new(parser, attrs)?);
                Ok(())
            },
        });

        Ok(Tileset {
            tile_width: width,
            tile_height: height,
            spacing: spacing.unwrap_or(0),
            margin: margin.unwrap_or(0),
            first_gid,
            name,
            tilecount,
            columns,
            images,
            terrain_types,
            tiles,
            properties,
        })
    }

    fn new_reference(
        attrs: &Vec<OwnedAttribute>,
        map_path: Option<&Path>,
    ) -> Result<Tileset, TiledError> {
        let ((), (first_gid, source)) = get_attrs!(
            attrs,
            optionals: [],
            required: [
                ("firstgid", first_gid, |v:String| v.parse().ok()),
                ("source", name, |v| Some(v)),
            ],
            TiledError::MalformedAttributes("tileset must have a firstgid, name tile width and height with correct types".to_string())
        );

        let tileset_path = map_path.ok_or(TiledError::Other("Maps with external tilesets must know their file location.  See parse_with_path(Path).".to_string()))?.with_file_name(source);
        let file = File::open(&tileset_path).map_err(|_| {
            TiledError::Other(format!(
                "External tileset file not found: {:?}",
                tileset_path
            ))
        })?;
        Tileset::new_external(file, first_gid)
    }

    fn new_external<R: Read>(file: R, first_gid: u32) -> Result<Tileset, TiledError> {
        let mut tileset_parser = EventReader::new(file);
        loop {
            match tileset_parser
                .next()
                .map_err(TiledError::XmlDecodingError)?
            {
                XmlEvent::StartElement {
                    name, attributes, ..
                } => {
                    if name.local_name == "tileset" {
                        return Tileset::parse_external_tileset(
                            first_gid,
                            &mut tileset_parser,
                            &attributes,
                        );
                    }
                }
                XmlEvent::EndDocument => {
                    return Err(TiledError::PrematureEnd(
                        "Tileset Document ended before map was parsed".to_string(),
                    ))
                }
                _ => {}
            }
        }
    }

    fn parse_external_tileset<R: Read>(
        first_gid: u32,
        parser: &mut EventReader<R>,
        attrs: &Vec<OwnedAttribute>,
    ) -> Result<Tileset, TiledError> {
        let ((spacing, margin, tilecount, columns), (name, width, height)) = get_attrs!(
            attrs,
            optionals: [
                ("spacing", spacing, |v:String| v.parse().ok()),
                ("margin", margin, |v:String| v.parse().ok()),
                ("tilecount", tilecount, |v:String| v.parse().ok()),
                ("columns", columns, |v:String| v.parse().ok()),
            ],
            required: [
                ("name", name, |v| Some(v)),
                ("tilewidth", width, |v:String| v.parse().ok()),
                ("tileheight", height, |v:String| v.parse().ok()),
            ],
            TiledError::MalformedAttributes("tileset must have a firstgid, name tile width and height with correct types".to_string())
        );

        let mut images = Vec::new();
        let mut terrain_types = None;
        let mut tiles = Vec::new();
        let mut properties = HashMap::new();
        parse_tag!(parser, "tileset", {
            "image" => |attrs| {
                images.push(Image::new(parser, attrs)?);
                Ok(())
            },
            "terraintypes" => |attrs| {
                terrain_types = Some(TerrainTypes::new(parser, attrs)?);
                Ok(())
            },
            "tile" => |attrs| {
                tiles.push(Tile::new(parser, attrs)?);
                Ok(())
            },
            "properties" => |_| {
                properties = parse_properties(parser)?;
                Ok(())
            },
        });

        Ok(Tileset {
            first_gid,
            name,
            tile_width: width,
            tile_height: height,
            spacing: spacing.unwrap_or(0),
            margin: margin.unwrap_or(0),
            tilecount,
            columns,
            images,
            terrain_types,
            tiles,
            properties,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Image {
    /// The filepath of the image
    pub source: String,
    pub width: i32,
    pub height: i32,
    pub transparent_colour: Option<Colour>,
}

impl Image {
    fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
    ) -> Result<Image, TiledError> {
        let (c, (s, w, h)) = get_attrs!(
            attrs,
            optionals: [
                ("trans", trans, |v:String| v.parse().ok()),
            ],
            required: [
                ("source", source, |v| Some(v)),
                ("width", width, |v:String| v.parse().ok()),
                ("height", height, |v:String| v.parse().ok()),
            ],
            TiledError::MalformedAttributes("image must have a source, width and
height with correct types".to_string())
        );

        parse_tag!(parser, "image", { "" => |_| Ok(()) });
        Ok(Image {
            source: s,
            width: w,
            height: h,
            transparent_colour: c,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Terrain {
    pub name: String,
    pub tile: u32,
    pub properties: Properties,
}

impl Terrain {
    fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
    ) -> Result<Terrain, TiledError> {
        let (_, (n, t)) = get_attrs!(
            attrs,
            optionals: [],
            required: [
                ("name", n, |v:String| v.parse().ok()),
                ("tile", t, |v:String| v.parse::<u32>().ok()),
            ],
            TiledError::MalformedAttributes("terrain must have a name and a tile".to_string())
        );
        let mut p = HashMap::new();
        parse_tag!(parser, "terrain", {
            "properties" => |_| {
                p = parse_properties(parser)?;
                Ok(())
            },
        });
        Ok(Terrain {
            name: n,
            tile: t,
            properties: p,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TerrainTypes {
    pub terrains: Vec<Terrain>,
}

impl TerrainTypes {
    fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
    ) -> Result<TerrainTypes, TiledError> {
        let mut terrains = Vec::new();
        parse_tag!(parser, "terraintypes", {
            "terrain" => |attrs| {
                terrains.push(Terrain::new(parser, attrs)?);
                Ok(())
            },
        });
        Ok(TerrainTypes {
            terrains,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Tile {
    pub id: u32,
    pub images: Vec<Image>,
    pub properties: Properties,
    pub objectgroup: Option<ObjectGroup>,
    pub animation: Option<Vec<Frame>>,
    pub tile_type: Option<String>,
    pub probability: f32,
    pub terrain: Vec<usize>,
}

impl Tile {
    fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
    ) -> Result<Tile, TiledError> {
        let ((tile_type, probability, terrain), id) = get_attrs!(
            attrs,
            optionals: [
                ("type", tile_type, |v:String| v.parse().ok()),
                ("probability", probability, |v:String| v.parse().ok()),
                ("terrain", terrain, |v:String| v.split(',').map(|s| {
                    s.parse::<usize>().ok()}).collect()),
            ],
            required: [
                ("id", id, |v:String| v.parse::<u32>().ok()),
            ],
            TiledError::MalformedAttributes("tile must have an id with the correct type".to_string())
        );

        let mut images = Vec::new();
        let mut properties = HashMap::new();
        let mut objectgroup = None;
        let mut animation = None;
        parse_tag!(parser, "tile", {
            "image" => |attrs| {
                images.push(Image::new(parser, attrs)?);
                Ok(())
            },
            "properties" => |_| {
                properties = parse_properties(parser)?;
                Ok(())
            },
            "objectgroup" => |attrs| {
                objectgroup = Some(ObjectGroup::new(parser, attrs, None)?);
                Ok(())
            },
            "animation" => |_| {
                animation = Some(parse_animation(parser)?);
                Ok(())
            },
        });
        Ok(Tile {
            id,
            images,
            properties,
            objectgroup,
            animation,
            tile_type,
            probability: probability.unwrap_or(1.0),
            terrain: terrain.unwrap_or(Vec::new()),
        })
    }
}

/// Stores the proper tile gid, along with how it is flipped.
// Maybe PartialEq and Eq should be custom, so that it ignores tile-flipping?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerTile {
    pub gid: u32,
    pub flip_h: bool,
    pub flip_v: bool,
    pub flip_d: bool,
}

const FLIPPED_HORIZONTALLY_FLAG: u32 = 0x80000000;
const FLIPPED_VERTICALLY_FLAG: u32 = 0x40000000;
const FLIPPED_DIAGONALLY_FLAG: u32 = 0x20000000;
const ALL_FLIP_FLAGS: u32 =
    FLIPPED_HORIZONTALLY_FLAG | FLIPPED_VERTICALLY_FLAG | FLIPPED_DIAGONALLY_FLAG;

impl LayerTile {
    pub fn new(id: u32) -> LayerTile {
        let flags = id & ALL_FLIP_FLAGS;
        let gid = id & !ALL_FLIP_FLAGS;
        let flip_d = flags & FLIPPED_DIAGONALLY_FLAG == FLIPPED_DIAGONALLY_FLAG; // Swap x and y axis (anti-diagonally) [flips over y = -x line]
        let flip_h = flags & FLIPPED_HORIZONTALLY_FLAG == FLIPPED_HORIZONTALLY_FLAG; // Flip tile over y axis
        let flip_v = flags & FLIPPED_VERTICALLY_FLAG == FLIPPED_VERTICALLY_FLAG; // Flip tile over x axis

        LayerTile {
            gid,
            flip_h,
            flip_v,
            flip_d,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Layer {
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    /// The tiles are arranged in rows. Each tile is a number which can be used
    ///  to find which tileset it belongs to and can then be rendered.
    pub tiles: LayerData,
    pub properties: Properties,
    pub layer_index: u32,
}

impl Layer {
    fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
        width: u32,
        layer_index: u32,
        infinite: bool,
    ) -> Result<Layer, TiledError> {
        let ((o, v), n) = get_attrs!(
            attrs,
            optionals: [
                ("opacity", opacity, |v:String| v.parse().ok()),
                ("visible", visible, |v:String| v.parse().ok().map(|x:i32| x == 1)),
            ],
            required: [
                ("name", name, |v| Some(v)),
            ],
            TiledError::MalformedAttributes("layer must have a name".to_string())
        );
        let mut tiles: LayerData = LayerData::Finite(Default::default());
        let mut properties = HashMap::new();
        parse_tag!(parser, "layer", {
            "data" => |attrs| {
                if infinite {
                    tiles = parse_infinite_data(parser, attrs, width)?;
                } else {
                    tiles = parse_data(parser, attrs, width)?;
                }
                Ok(())
            },
            "properties" => |_| {
                properties = parse_properties(parser)?;
                Ok(())
            },
        });

        Ok(Layer {
            name: n,
            opacity: o.unwrap_or(1.0),
            visible: v.unwrap_or(true),
            tiles: tiles,
            properties: properties,
            layer_index,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum LayerData {
    Finite(Vec<Vec<LayerTile>>),
    Infinite(HashMap<(i32, i32), Chunk>)
}

#[derive(Debug, PartialEq, Clone)]
pub struct Chunk {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Vec<LayerTile>>,
}


impl Chunk {
    pub(crate) fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
        encoding: Option<String>,
        compression: Option<String>,
    ) -> Result<Chunk, TiledError> {
        let ((), (x, y, width, height)) = get_attrs!(
            attrs,
            optionals: [],
            required: [
                ("x", x, |v: String| v.parse().ok()),
                ("y", y, |v: String| v.parse().ok()),
                ("width", width, |v: String| v.parse().ok()),
                ("height", height, |v: String| v.parse().ok()),
            ],
            TiledError::MalformedAttributes("layer must have a name".to_string())
        );



        let tiles = parse_data_line(encoding, compression, parser, width)?;

        Ok(Chunk {
                x,
                y,
                width,
                height,
                tiles,
            })
    }
}


#[derive(Debug, PartialEq, Clone)]
pub struct ImageLayer {
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    pub offset_x: f32,
    pub offset_y: f32,
    pub image: Option<Image>,
    pub properties: Properties,
    pub layer_index: u32,
}

impl ImageLayer {
    fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
        layer_index: u32,
    ) -> Result<ImageLayer, TiledError> {
        let ((o, v, ox, oy), n) = get_attrs!(
            attrs,
            optionals: [
                ("opacity", opacity, |v:String| v.parse().ok()),
                ("visible", visible, |v:String| v.parse().ok().map(|x:i32| x == 1)),
                ("offsetx", offset_x, |v:String| v.parse().ok()),
                ("offsety", offset_y, |v:String| v.parse().ok()),
            ],
            required: [
                ("name", name, |v| Some(v)),
            ],
            TiledError::MalformedAttributes("layer must have a name".to_string()));
        let mut properties = HashMap::new();
        let mut image: Option<Image> = None;
        parse_tag!(parser, "imagelayer", {
            "image" => |attrs| {
                image = Some(Image::new(parser, attrs)?);
                Ok(())
            },
            "properties" => |_| {
                properties = parse_properties(parser)?;
                Ok(())
            },
        });
        Ok(ImageLayer {
            name: n,
            opacity: o.unwrap_or(1.0),
            visible: v.unwrap_or(true),
            offset_x: ox.unwrap_or(0.0),
            offset_y: oy.unwrap_or(0.0),
            image,
            properties,
            layer_index,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ObjectGroup {
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    pub objects: Vec<Object>,
    pub colour: Option<Colour>,
    /**
     * Layer index is not preset for tile collision boxes
     */
    pub layer_index: Option<u32>,
    pub properties: Properties,
}

impl ObjectGroup {
    fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
        layer_index: Option<u32>,
    ) -> Result<ObjectGroup, TiledError> {
        let ((o, v, c, n), ()) = get_attrs!(
            attrs,
            optionals: [
                ("opacity", opacity, |v:String| v.parse().ok()),
                ("visible", visible, |v:String| v.parse().ok().map(|x:i32| x == 1)),
                ("color", colour, |v:String| v.parse().ok()),
                ("name", name, |v:String| v.into()),
            ],
            required: [],
            TiledError::MalformedAttributes("object groups must have a name".to_string())
        );
        let mut objects = Vec::new();
        let mut properties = HashMap::new();
        parse_tag!(parser, "objectgroup", {
            "object" => |attrs| {
                objects.push(Object::new(parser, attrs)?);
                Ok(())
            },
            "properties" => |_| {
                properties = parse_properties(parser)?;
                Ok(())
            },
        });
        Ok(ObjectGroup {
            name: n.unwrap_or(String::new()),
            opacity: o.unwrap_or(1.0),
            visible: v.unwrap_or(true),
            objects: objects,
            colour: c,
            layer_index,
            properties,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ObjectShape {
    Rect { width: f32, height: f32 },
    Ellipse { width: f32, height: f32 },
    Polyline { points: Vec<(f32, f32)> },
    Polygon { points: Vec<(f32, f32)> },
    Point(f32, f32),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Object {
    pub id: u32,
    pub gid: u32,
    pub name: String,
    pub obj_type: String,
    pub width: f32,
    pub height: f32,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub visible: bool,
    pub shape: ObjectShape,
    pub properties: Properties,
}

impl Object {
    fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
    ) -> Result<Object, TiledError> {
        let ((id, gid, n, t, w, h, v, r), (x, y)) = get_attrs!(
            attrs,
            optionals: [
                ("id", id, |v:String| v.parse().ok()),
                ("gid", gid, |v:String| v.parse().ok()),
                ("name", name, |v:String| v.parse().ok()),
                ("type", obj_type, |v:String| v.parse().ok()),
                ("width", width, |v:String| v.parse().ok()),
                ("height", height, |v:String| v.parse().ok()),
                ("visible", visible, |v:String| v.parse().ok()),
                ("rotation", rotation, |v:String| v.parse().ok()),
            ],
            required: [
                ("x", x, |v:String| v.parse().ok()),
                ("y", y, |v:String| v.parse().ok()),
            ],
            TiledError::MalformedAttributes("objects must have an x and a y number".to_string())
        );
        let v = v.unwrap_or(true);
        let w = w.unwrap_or(0f32);
        let h = h.unwrap_or(0f32);
        let r = r.unwrap_or(0f32);
        let id = id.unwrap_or(0u32);
        let gid = gid.unwrap_or(0u32);
        let n = n.unwrap_or(String::new());
        let t = t.unwrap_or(String::new());
        let mut shape = None;
        let mut properties = HashMap::new();

        parse_tag!(parser, "object", {
            "ellipse" => |_| {
                shape = Some(ObjectShape::Ellipse {
                    width: w,
                    height: h,
                });
                Ok(())
            },
            "polyline" => |attrs| {
                shape = Some(Object::new_polyline(attrs)?);
                Ok(())
            },
            "polygon" => |attrs| {
                shape = Some(Object::new_polygon(attrs)?);
                Ok(())
            },
            "point" => |_| {
                shape = Some(Object::new_point(x, y)?);
                Ok(())
            },
            "properties" => |_| {
                properties = parse_properties(parser)?;
                Ok(())
            },
        });

        let shape = shape.unwrap_or(ObjectShape::Rect {
            width: w,
            height: h,
        });

        Ok(Object {
            id: id,
            gid: gid,
            name: n.clone(),
            obj_type: t.clone(),
            width: w,
            height: h,
            x: x,
            y: y,
            rotation: r,
            visible: v,
            shape: shape,
            properties: properties,
        })
    }

    fn new_polyline(attrs: Vec<OwnedAttribute>) -> Result<ObjectShape, TiledError> {
        let ((), s) = get_attrs!(
            attrs,
            optionals: [],
            required: [
                ("points", points, |v| Some(v)),
            ],
            TiledError::MalformedAttributes("A polyline must have points".to_string())
        );
        let points = Object::parse_points(s)?;
        Ok(ObjectShape::Polyline { points: points })
    }

    fn new_polygon(attrs: Vec<OwnedAttribute>) -> Result<ObjectShape, TiledError> {
        let ((), s) = get_attrs!(
            attrs,
            optionals: [],
            required: [
                ("points", points, |v| Some(v)),
            ],
            TiledError::MalformedAttributes("A polygon must have points".to_string())
        );
        let points = Object::parse_points(s)?;
        Ok(ObjectShape::Polygon { points: points })
    }

    fn new_point(x: f32, y: f32) -> Result<ObjectShape, TiledError> {
        Ok(ObjectShape::Point(x, y))
    }

    fn parse_points(s: String) -> Result<Vec<(f32, f32)>, TiledError> {
        let pairs = s.split(' ');
        let mut points = Vec::new();
        for v in pairs.map(|p| p.split(',')) {
            let v: Vec<&str> = v.collect();
            if v.len() != 2 {
                return Err(TiledError::MalformedAttributes(
                    "one of a polyline's points does not have an x and y coordinate".to_string(),
                ));
            }
            let (x, y) = (v[0].parse().ok(), v[1].parse().ok());
            if x.is_none() || y.is_none() {
                return Err(TiledError::MalformedAttributes(
                    "one of polyline's points does not have i32eger coordinates".to_string(),
                ));
            }
            points.push((x.unwrap(), y.unwrap()));
        }
        Ok(points)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Frame {
    pub tile_id: u32,
    pub duration: u32,
}

impl Frame {
    fn new(attrs: Vec<OwnedAttribute>) -> Result<Frame, TiledError> {
        let ((), (tile_id, duration)) = get_attrs!(
            attrs,
            optionals: [],
            required: [
                ("tileid", tile_id, |v:String| v.parse().ok()),
                ("duration", duration, |v:String| v.parse().ok()),
            ],
            TiledError::MalformedAttributes("A frame must have tileid and duration".to_string())
        );
        Ok(Frame {
            tile_id: tile_id,
            duration: duration,
        })
    }
}

fn parse_animation<R: Read>(parser: &mut EventReader<R>) -> Result<Vec<Frame>, TiledError> {
    let mut animation = Vec::new();
    parse_tag!(parser, "animation", {
        "frame" => |attrs| {
            animation.push(Frame::new(attrs)?);
            Ok(())
        },
    });
    Ok(animation)
}

fn parse_infinite_data<R: Read>(
    parser: &mut EventReader<R>,
    attrs: Vec<OwnedAttribute>,
    width: u32,
) -> Result<LayerData, TiledError> {
    let ((e, c), ()) = get_attrs!(
        attrs,
        optionals: [
            ("encoding", encoding, |v| Some(v)),
            ("compression", compression, |v| Some(v)),
        ],
        required: [],
        TiledError::MalformedAttributes("data must have an encoding and a compression".to_string())
    );

    let mut chunks = HashMap::<(i32, i32), Chunk>::new();
    parse_tag!(parser, "data", {
        "chunk" => |attrs| {
            let chunk = Chunk::new(parser, attrs, e.clone(), c.clone())?;
            chunks.insert((chunk.x, chunk.y), chunk);
            Ok(())
        }
    });

    Ok(LayerData::Infinite(chunks))
}

fn parse_data<R: Read>(
    parser: &mut EventReader<R>,
    attrs: Vec<OwnedAttribute>,
    width: u32,
) -> Result<LayerData, TiledError> {
    let ((e, c), ()) = get_attrs!(
        attrs,
        optionals: [
            ("encoding", encoding, |v| Some(v)),
            ("compression", compression, |v| Some(v)),
        ],
        required: [],
        TiledError::MalformedAttributes("data must have an encoding and a compression".to_string())
    );

    let tiles = parse_data_line(e, c, parser, width)?;

    Ok(LayerData::Finite(tiles))
}

fn parse_data_line<R: Read>(encoding: Option<String>, compression: Option<String>, parser: &mut EventReader<R>, width: u32) -> Result<Vec<Vec<LayerTile>>, TiledError> {
    match (encoding, compression) {
        (None, None) => {
            return Err(TiledError::Other(
                "XML format is currently not supported".to_string(),
            ))
        }
        (Some(e), None) => match e.as_ref() {
            "base64" => return parse_base64(parser).map(|v| convert_to_tile(&v, width)),
            "csv" => return decode_csv(parser),
            e => return Err(TiledError::Other(format!("Unknown encoding format {}", e))),
        },
        (Some(e), Some(c)) => match (e.as_ref(), c.as_ref()) {
            ("base64", "zlib") => {
                return parse_base64(parser)
                    .and_then(decode_zlib)
                    .map(|v| convert_to_tile(&v, width))
            }
            ("base64", "gzip") => {
                return parse_base64(parser)
                    .and_then(decode_gzip)
                    .map(|v| convert_to_tile(&v, width))
            }
            ("base64", "zstd") => {
                return parse_base64(parser)
                    .and_then(decode_zstd)
                    .map(|v| convert_to_tile(&v, width))
            }
            (e, c) => {
                return Err(TiledError::Other(format!(
                    "Unknown combination of {} encoding and {} compression",
                    e, c
                )))
            }
        },
        _ => return Err(TiledError::Other("Missing encoding format".to_string())),
    };
}

fn parse_base64<R: Read>(parser: &mut EventReader<R>) -> Result<Vec<u8>, TiledError> {
    loop {
        match parser.next().map_err(TiledError::XmlDecodingError)? {
            XmlEvent::Characters(s) => {
                return base64::decode(s.trim().as_bytes()).map_err(TiledError::Base64DecodingError)
            }
            XmlEvent::EndElement { name, .. } => {
                if name.local_name == "data" {
                    return Ok(Vec::new());
                }
            }
            _ => {}
        }
    }
}

fn decode_zlib(data: Vec<u8>) -> Result<Vec<u8>, TiledError> {
    use libflate::zlib::Decoder;
    let mut zd =
        Decoder::new(BufReader::new(&data[..])).map_err(|e| TiledError::DecompressingError(e))?;
    let mut data = Vec::new();
    match zd.read_to_end(&mut data) {
        Ok(_v) => {}
        Err(e) => return Err(TiledError::DecompressingError(e)),
    }
    Ok(data)
}

fn decode_gzip(data: Vec<u8>) -> Result<Vec<u8>, TiledError> {
    use libflate::gzip::Decoder;
    let mut zd =
        Decoder::new(BufReader::new(&data[..])).map_err(|e| TiledError::DecompressingError(e))?;

    let mut data = Vec::new();
    zd.read_to_end(&mut data)
        .map_err(|e| TiledError::DecompressingError(e))?;
    Ok(data)
}

fn decode_zstd(data: Vec<u8>) -> Result<Vec<u8>, TiledError> {
    use std::io::Cursor;
    use zstd::stream::read::Decoder;

    let buff = Cursor::new(&data);
    let mut zd = Decoder::with_buffer(buff).map_err(|e| TiledError::DecompressingError(e))?;

    let mut data = Vec::new();
    zd.read_to_end(&mut data)
        .map_err(|e| TiledError::DecompressingError(e))?;
    Ok(data)
}

fn decode_csv<R: Read>(parser: &mut EventReader<R>) -> Result<Vec<Vec<LayerTile>>, TiledError> {
    loop {
        match parser.next().map_err(TiledError::XmlDecodingError)? {
            XmlEvent::Characters(s) => {
                let mut rows: Vec<Vec<LayerTile>> = Vec::new();
                for row in s.split('\n') {
                    if row.trim() == "" {
                        continue;
                    }
                    rows.push(
                        row.split(',')
                            .filter(|v| v.trim() != "")
                            .map(|v| v.replace('\r', "").parse().unwrap())
                            .map(|id| LayerTile::new(id))
                            .collect(),
                    );
                }
                return Ok(rows);
            }
            XmlEvent::EndElement { name, .. } => {
                if name.local_name == "data" {
                    return Ok(Vec::new());
                }
            }
            _ => {}
        }
    }
}

fn convert_to_tile(all: &Vec<u8>, width: u32) -> Vec<Vec<LayerTile>> {
    let mut data = Vec::new();
    for chunk in all.chunks((width * 4) as usize) {
        let mut row = Vec::new();
        for i in 0..width {
            let start: usize = i as usize * 4;
            let n = ((chunk[start + 3] as u32) << 24)
                + ((chunk[start + 2] as u32) << 16)
                + ((chunk[start + 1] as u32) << 8)
                + chunk[start] as u32;
            let n = LayerTile::new(n);
            row.push(n);
        }
        data.push(row);
    }
    data
}

fn parse_impl<R: Read>(reader: R, map_path: Option<&Path>) -> Result<Map, TiledError> {
    let mut parser = EventReader::new(reader);
    loop {
        match parser.next().map_err(TiledError::XmlDecodingError)? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                if name.local_name == "map" {
                    return Map::new(&mut parser, attributes, map_path);
                }
            }
            XmlEvent::EndDocument => {
                return Err(TiledError::PrematureEnd(
                    "Document ended before map was parsed".to_string(),
                ))
            }
            _ => {}
        }
    }
}

/// Parse a buffer hopefully containing the contents of a Tiled file and try to
/// parse it. This augments `parse` with a file location: some engines
/// (e.g. Amethyst) simply hand over a byte stream (and file location) for parsing,
/// in which case this function may be required.
pub fn parse_with_path<R: Read>(reader: R, path: &Path) -> Result<Map, TiledError> {
    parse_impl(reader, Some(path))
}

/// Parse a file hopefully containing a Tiled map and try to parse it.  If the
/// file has an external tileset, the tileset file will be loaded using a path
/// relative to the map file's path.
pub fn parse_file(path: &Path) -> Result<Map, TiledError> {
    let file = File::open(path)
        .map_err(|_| TiledError::Other(format!("Map file not found: {:?}", path)))?;
    parse_impl(file, Some(path))
}

/// Parse a buffer hopefully containing the contents of a Tiled file and try to
/// parse it.
pub fn parse<R: Read>(reader: R) -> Result<Map, TiledError> {
    parse_impl(reader, None)
}

/// Parse a buffer hopefully containing the contents of a Tiled tileset.
///
/// External tilesets do not have a firstgid attribute.  That lives in the
/// map. You must pass in `first_gid`.  If you do not need to use gids for anything,
/// passing in 1 will work fine.
pub fn parse_tileset<R: Read>(reader: R, first_gid: u32) -> Result<Tileset, TiledError> {
    Tileset::new_external(reader, first_gid)
}
