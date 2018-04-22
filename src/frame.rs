use xml::attribute::OwnedAttribute;

use TiledError;

#[derive(Debug, PartialEq, Clone)]
pub struct Frame {
    tile_id: u32,
    duration: u32,
}

impl Frame {
    pub(crate) fn new(attrs: Vec<OwnedAttribute>) -> Result<Frame, TiledError> {
        let ((), (tile_id, duration)) = get_attrs!(
            attrs,
            optionals: [],
            required: [("tileid", tile_id, |v:String| v.parse().ok()),
            ("duration", duration, |v:String| v.parse().ok())],
            TiledError::MalformedAttributes("A frame must have tileid and duration".to_string()));
        Ok(Frame {
            tile_id: tile_id,
            duration: duration,
        })
    }
}
