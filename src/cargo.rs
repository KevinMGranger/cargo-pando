use serde_derive::Deserialize;
use serde_yaml::from_reader;
use std::env::var_os;
use std::process::{Command, Stdio};

#[derive(Deserialize, Debug)]
pub struct CargoMetadata {
    pub target_directory: String,
}

pub fn get_cargo_metadata() -> CargoMetadata {
    let mut child = Command::new(var_os("CARGO").unwrap())
        .args(&["metadata", "--format-version", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    from_reader(child.stdout.take().unwrap()).unwrap()
}
