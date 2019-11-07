use std::io;

use dex_parser::DexReader;

fn main() -> io::Result<()> {
    env_logger::init();
    let dex = DexReader::from_file("resources/classes.dex").unwrap();
    for class in dex.classes() {
        if class.is_err() {
            println!("scroll-error: {:?}", class);
        }
        //        println!("class name: {:?}", class.expect("Class failed").get_type());
        //println!("class: {:?}", class.unwrap());
    }
    Ok(())
}
