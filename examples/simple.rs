use dex_parser::DexBuilder;

use std::io;

fn main() -> io::Result<()> {
    let dex = DexBuilder::from_file("resources/classes.dex").unwrap();
    let s = dex.get_string(2).unwrap();
    //let t = dex.get_type(1).unwrap();

    println!("string: {:?}", s);
    //println!("type: {:?}", t);
    Ok(())
}
