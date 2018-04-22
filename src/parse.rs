use std::io::{BufReader, Read};
use std::path::Path;
use xml::reader::{EventReader, XmlEvent};
use xml::attribute::OwnedAttribute;
use flate2::read::{GzDecoder, ZlibDecoder};
use base64::u8de as decode_base64;

use TiledError;
use Map;
use Frame;

pub(crate) fn parse_animation<R: Read>(
    parser: &mut EventReader<R>,
) -> Result<Vec<Frame>, TiledError> {
    let mut animation = Vec::new();
    parse_tag!(parser, "animation",
                   "frame" => |attrs| {
                        animation.push(try!(Frame::new(attrs)));
                        Ok(())
                   });
    Ok(animation)
}

pub(crate) fn parse_data<R: Read>(
    parser: &mut EventReader<R>,
    attrs: Vec<OwnedAttribute>,
    width: u32,
) -> Result<Vec<Vec<u32>>, TiledError> {
    let ((e, c), ()) = get_attrs!(
        attrs,
        optionals: [("encoding", encoding, |v| Some(v)),
                   ("compression", compression, |v| Some(v))],
        required: [],
        TiledError::MalformedAttributes("data must have an encoding and a compression".to_string()));

    match (e, c) {
        (None, None) => {
            return Err(TiledError::Other(
                "XML format is currently not supported".to_string(),
            ))
        }
        (Some(e), None) => match e.as_ref() {
            "base64" => return parse_base64(parser).map(|v| convert_to_u32(&v, width)),
            "csv" => return decode_csv(parser),
            e => return Err(TiledError::Other(format!("Unknown encoding format {}", e))),
        },
        (Some(e), Some(c)) => match (e.as_ref(), c.as_ref()) {
            ("base64", "zlib") => {
                return parse_base64(parser)
                    .and_then(decode_zlib)
                    .map(|v| convert_to_u32(&v, width))
            }
            ("base64", "gzip") => {
                return parse_base64(parser)
                    .and_then(decode_gzip)
                    .map(|v| convert_to_u32(&v, width))
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

pub(crate) fn parse_base64<R: Read>(parser: &mut EventReader<R>) -> Result<Vec<u8>, TiledError> {
    loop {
        match try!(parser.next().map_err(TiledError::XmlDecodingError)) {
            XmlEvent::Characters(s) => {
                return decode_base64(s.trim().as_bytes()).map_err(TiledError::Base64DecodingError)
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

pub(crate) fn decode_zlib(data: Vec<u8>) -> Result<Vec<u8>, TiledError> {
    let mut zd = ZlibDecoder::new(BufReader::new(&data[..]));
    let mut data = Vec::new();
    match zd.read_to_end(&mut data) {
        Ok(_v) => {}
        Err(e) => return Err(TiledError::DecompressingError(e)),
    }
    Ok(data)
}

pub(crate) fn decode_gzip(data: Vec<u8>) -> Result<Vec<u8>, TiledError> {
    let mut gzd = GzDecoder::new(BufReader::new(&data[..]));
    let mut data = Vec::new();
    gzd.read_to_end(&mut data)
        .map_err(|e| TiledError::DecompressingError(e))?;
    Ok(data)
}

pub(crate) fn decode_csv<R: Read>(
    parser: &mut EventReader<R>,
) -> Result<Vec<Vec<u32>>, TiledError> {
    loop {
        match try!(parser.next().map_err(TiledError::XmlDecodingError)) {
            XmlEvent::Characters(s) => {
                let mut rows: Vec<Vec<u32>> = Vec::new();
                for row in s.split('\n') {
                    if row.trim() == "" {
                        continue;
                    }
                    rows.push(
                        row.split(',')
                            .filter(|v| v.trim() != "")
                            .map(|v| v.replace('\r', "").parse().unwrap())
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

pub(crate) fn convert_to_u32(all: &Vec<u8>, width: u32) -> Vec<Vec<u32>> {
    let mut data = Vec::new();
    for chunk in all.chunks((width * 4) as usize) {
        let mut row = Vec::new();
        for i in 0..width {
            let start: usize = i as usize * 4;
            let n = ((chunk[start + 3] as u32) << 24) + ((chunk[start + 2] as u32) << 16)
                + ((chunk[start + 1] as u32) << 8) + chunk[start] as u32;
            row.push(n);
        }
        data.push(row);
    }
    data
}

pub(crate) fn parse_impl<R: Read, P: AsRef<Path>>(
    reader: R,
    map_path: Option<P>,
) -> Result<Map, TiledError> {
    let mut parser = EventReader::new(reader);
    loop {
        match try!(parser.next().map_err(TiledError::XmlDecodingError)) {
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
