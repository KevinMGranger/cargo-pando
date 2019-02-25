use serde_derive::Deserialize;
use serde_yaml::from_reader;
use std::env::var_os;
use std::process::Command;

#[derive(Deserialize, Debug)]
pub struct CargoMetadata {
    pub target_directory: String,
}

pub fn get_cargo_metadata() -> CargoMetadata {
    let mut child = Command::new(var_os("CARGO").unwrap())
        .arg("metadata")
        .spawn()
        .unwrap();

    from_reader(child.stdout.take().unwrap()).unwrap()
}