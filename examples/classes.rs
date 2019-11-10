use std::io;

use dex_parser::DexReader;

fn main() -> io::Result<()> {
    env_logger::init();
    let dex = DexReader::from_file("resources/classes.dex").unwrap();
    for class in dex.classes() {
        let class = class.expect("Class failed");
        println!("class name: {:?}", class.jtype());
    }
    Ok(())
}
