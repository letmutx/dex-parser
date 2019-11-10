use dex_parser::DexReader;

use std::io;

fn main() -> io::Result<()> {
    let dex = DexReader::from_file("resources/classes.dex").unwrap();
    let s = dex.get_string(2).unwrap();
    println!("string: {:?}", s);
    Ok(())
}
