use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use xml::reader::EventReader;
use xml::reader::XmlEvent;
use xml::attribute::OwnedAttribute;

use {Colour, ImageLayer, Layer, ObjectGroup, Orientation, Tileset};
use properties::{parse_properties, Properties};
use error::TiledError;

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
}

impl Map {
    pub(crate) fn new<R: Read, P: AsRef<Path>>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
        map_path: Option<P>,
    ) -> Result<Map, TiledError> {
        let (c, (v, o, w, h, tw, th)) = get_attrs!(
            attrs,
            optionals: [("backgroundcolor", colour, |v:String| v.parse().ok())],
            required: [("version", version, |v| Some(v)),
                       ("orientation", orientation, |v:String| v.parse().ok()),
                       ("width", width, |v:String| v.parse().ok()),
                       ("height", height, |v:String| v.parse().ok()),
                       ("tilewidth", tile_width, |v:String| v.parse().ok()),
                       ("tileheight", tile_height, |v:String| v.parse().ok())],
            TiledError::MalformedAttributes("map must have a version, width and height with correct types".to_string()));

        let mut tilesets = Vec::new();
        let mut layers = Vec::new();
        let mut image_layers = Vec::new();
        let mut properties = HashMap::new();
        let mut object_groups = Vec::new();
        parse_tag!(parser, "map",
                   "tileset" => | attrs| {
                        tilesets.push(try!(Tileset::new(parser, attrs, map_path.as_ref())));
                        Ok(())
                   },
                   "layer" => |attrs| {
                        layers.push(try!(Layer::new(parser, attrs, w)));
                        Ok(())
                   },
                   "imagelayer" => |attrs| {
                        image_layers.push(try!(ImageLayer::new(parser, attrs)));
                        Ok(())
                   },
                   "properties" => |_| {
                        properties = try!(parse_properties(parser));
                        Ok(())
                   },
                   "objectgroup" => |attrs| {
                       object_groups.push(try!(ObjectGroup::new(parser, attrs)));
                       Ok(())
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
