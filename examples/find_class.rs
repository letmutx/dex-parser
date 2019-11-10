use dex_parser::DexReader;

use std::io;

fn main() -> io::Result<()> {
    env_logger::init();
    let dex = DexReader::from_file("resources/classes.dex").unwrap();
    let class = dex
        .find_class_by_name("La/a/a/a/d;")
        .expect("Failed to load class")
        .expect("class not found");
    let static_fields = class.static_fields();
    Ok(())
}
