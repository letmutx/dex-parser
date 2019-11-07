use std::io;

use dex_parser::DexReader;

fn main() -> io::Result<()> {
    let dex = DexReader::from_file("resources/classes.dex").unwrap();
    for _class in dex.classes() {
        //println!("class: {:?}", class.unwrap());
    }
    Ok(())
}
