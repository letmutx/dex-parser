use dex::DexReader;

use std::io;

fn main() -> io::Result<()> {
    env_logger::init();
    let dex = DexReader::from_file("resources/classes.dex").unwrap();
    let class = dex
        .find_class_by_name("Lorg/adw/launcher/Launcher;")
        .expect("Failed to load class")
        .expect("class not found");
    println!("class type: {}", class.jtype());
    Ok(())
}
