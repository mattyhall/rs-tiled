use std::io::Read;
use std::collections::HashMap;
use xml::reader::{EventReader, XmlEvent};
use xml::attribute::OwnedAttribute;

use TiledError;
use Colour;
use Properties;
use parse_properties;

#[derive(Debug, PartialEq, Clone)]
pub struct ObjectGroup {
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    pub objects: Vec<Object>,
    pub colour: Option<Colour>,
}

impl ObjectGroup {
    pub(crate) fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
    ) -> Result<ObjectGroup, TiledError> {
        let ((o, v, c, n), ()) = get_attrs!(
            attrs,
            optionals: [("opacity", opacity, |v:String| v.parse().ok()),
                        ("visible", visible, |v:String| v.parse().ok().map(|x:i32| x == 1)),
                        ("color", colour, |v:String| v.parse().ok()),
                        ("name", name, |v:String| v.into())],
            required: [],
            TiledError::MalformedAttributes("object groups must have a name".to_string()));
        let mut objects = Vec::new();
        parse_tag!(parser, "objectgroup",
                   "object" => |attrs| {
                        objects.push(try!(Object::new(parser, attrs)));
                        Ok(())
                   });
        Ok(ObjectGroup {
            name: n.unwrap_or(String::new()),
            opacity: o.unwrap_or(1.0),
            visible: v.unwrap_or(true),
            objects: objects,
            colour: c,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ObjectShape {
    Rect { width: f32, height: f32 },
    Ellipse { width: f32, height: f32 },
    Polyline { points: Vec<(f32, f32)> },
    Polygon { points: Vec<(f32, f32)> },
}

#[derive(Debug, PartialEq, Clone)]
pub struct Object {
    pub id: u32,
    pub gid: u32,
    pub name: String,
    pub obj_type: String,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub visible: bool,
    pub shape: ObjectShape,
    pub properties: Properties,
}

impl Object {
    pub(crate) fn new<R: Read>(
        parser: &mut EventReader<R>,
        attrs: Vec<OwnedAttribute>,
    ) -> Result<Object, TiledError> {
        let ((id, gid, n, t, w, h, v, r), (x, y)) = get_attrs!(
            attrs,
            optionals: [("id", id, |v:String| v.parse().ok()),
                        ("gid", gid, |v:String| v.parse().ok()),
                        ("name", name, |v:String| v.parse().ok()),
                        ("type", obj_type, |v:String| v.parse().ok()),
                        ("width", width, |v:String| v.parse().ok()),
                        ("height", height, |v:String| v.parse().ok()),
                        ("visible", visible, |v:String| v.parse().ok()),
                        ("rotation", rotation, |v:String| v.parse().ok())],
            required: [("x", x, |v:String| v.parse().ok()),
                       ("y", y, |v:String| v.parse().ok())],
            TiledError::MalformedAttributes("objects must have an x and a y number".to_string()));
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

        parse_tag!(
            parser, "object",
            "ellipse" => |_| {
                shape = Some(ObjectShape::Ellipse {
                    width: w,
                    height: h,
                });
                Ok(())
            },
            "polyline" => |attrs| {
                shape = Some(try!(Object::new_polyline(attrs)));
                Ok(())
            },
            "polygon" => |attrs| {
                shape = Some(try!(Object::new_polygon(attrs)));
                Ok(())
            },
            "properties" => |_| {
                properties = try!(parse_properties(parser));
                Ok(())
            }
        );

        let shape = shape.unwrap_or(ObjectShape::Rect {
            width: w,
            height: h,
        });

        Ok(Object {
            id: id,
            gid: gid,
            name: n.clone(),
            obj_type: t.clone(),
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
            required: [("points", points, |v| Some(v))],
            TiledError::MalformedAttributes("A polyline must have points".to_string()));
        let points = try!(Object::parse_points(s));
        Ok(ObjectShape::Polyline { points: points })
    }

    fn new_polygon(attrs: Vec<OwnedAttribute>) -> Result<ObjectShape, TiledError> {
        let ((), s) = get_attrs!(
            attrs,
            optionals: [],
            required: [("points", points, |v| Some(v))],
            TiledError::MalformedAttributes("A polygon must have points".to_string()));
        let points = try!(Object::parse_points(s));
        Ok(ObjectShape::Polygon { points: points })
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
