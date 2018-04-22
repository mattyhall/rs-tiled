use std::io::Read;
use xml::reader::{EventReader, XmlEvent};
use xml::attribute::OwnedAttribute;
use std::collections::HashMap;

use TiledError;
use Properties;
use Image;
use {parse_data, parse_properties};

#[derive(Debug, PartialEq, Clone)]
pub struct Layer {
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    /// The tiles are arranged in rows. Each tile is a number which can be used
    ///  to find which tileset it belongs to and can then be rendered.
    pub tiles: Vec<Vec<u32>>,
    pub properties: Properties,
}

impl Layer {
    pub(crate) fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
        width: u32,
    ) -> Result<Layer, TiledError> {
        let ((o, v), n) = get_attrs!(
            attrs,
            optionals: [("opacity", opacity, |v:String| v.parse().ok()),
                        ("visible", visible, |v:String| v.parse().ok().map(|x:i32| x == 1))],
            required: [("name", name, |v| Some(v))],
            TiledError::MalformedAttributes("layer must have a name".to_string()));
        let mut tiles = Vec::new();
        let mut properties = HashMap::new();
        parse_tag!(parser, "layer",
                   "data" => |attrs| {
                        tiles = try!(parse_data(parser, attrs, width));
                        Ok(())
                   },
                   "properties" => |_| {
                        properties = try!(parse_properties(parser));
                        Ok(())
                   });
        Ok(Layer {
            name: n,
            opacity: o.unwrap_or(1.0),
            visible: v.unwrap_or(true),
            tiles: tiles,
            properties: properties,
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
}

impl ImageLayer {
    pub(crate) fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
    ) -> Result<ImageLayer, TiledError> {
        let ((o, v, ox, oy), n) = get_attrs!(
            attrs,
            optionals: [("opacity", opacity, |v:String| v.parse().ok()),
                        ("visible", visible, |v:String| v.parse().ok().map(|x:i32| x == 1)),
                        ("offset_x", offset_x, |v:String| v.parse().ok()),
                        ("offset_y", offset_y, |v:String| v.parse().ok())],
            required: [("name", name, |v| Some(v))],
            TiledError::MalformedAttributes("layer must have a name".to_string()));
        let mut properties = HashMap::new();
        let mut image: Option<Image> = None;
        parse_tag!(parser, "imagelayer",
                   "image" => |attrs| {
                       image = Some(Image::new(parser, attrs)?);
                       Ok(())
                   },
                   "properties" => |_| {
                       properties = parse_properties(parser)?;
                       Ok(())
                   });
        Ok(ImageLayer {
            name: n,
            opacity: o.unwrap_or(1.0),
            visible: v.unwrap_or(true),
            offset_x: ox.unwrap_or(0.0),
            offset_y: oy.unwrap_or(0.0),
            image,
            properties,
        })
    }
}
