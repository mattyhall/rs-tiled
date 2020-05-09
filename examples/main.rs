use std::fs::File;
use tiled::parse;

fn main() {
    let file = File::open("assets/tiled_base64_zlib.tmx").unwrap();
    let map = parse(file).unwrap();
    println!("{:?}", map);
    println!("{:?}", map.get_tileset_by_gid(22));
}
