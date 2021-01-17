# rs-tiled

[![Travis](https://travis-ci.org/mattyhall/rs-tiled.svg?branch=master)](https://travis-ci.org/mattyhall/rs-tiled)
[![Crates.io](https://img.shields.io/crates/v/tiled.svg)](https://crates.io/crates/tiled)

Read maps from the [Tiled Map Editor](http://www.mapeditor.org/) into rust for use in video games. It is game engine agnostic and pretty barebones at the moment. Documentation is available [on docs.rs](https://docs.rs/tiled/).

Code contributions are welcome as are bug reports, documentation, suggestions and criticism.

[There is a package on crates.io](https://crates.io/crates/tiled), to use simply add:

```
tiled = "0.9.3"
```

to the dependencies section of your Cargo.toml.

### Example

```rust
extern crate serialize;
extern crate tiled;

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use tiled::parse;

fn main() {
    let file = File::open(&Path::new("assets/tiled_base64_zlib.tmx")).unwrap();
    println!("Opened file");
    let reader = BufReader::new(file);
    let map = parse(reader).unwrap();
    println!("{:?}", map);
    println!("{:?}", map.get_tileset_by_gid(22));
}
```

### Things missing

There are a few things missing at the moment:

- Tile flipping
- Image layers
- A nice API. At the moment you can access attributes and properties, find tilesets by GID and loop through the tiles. This leaves a user of the library with a bit to do.

### Licences

assets/tilesheet.png by Buch (http://blog-buch.rhcloud.com/)

Licenced under MIT
