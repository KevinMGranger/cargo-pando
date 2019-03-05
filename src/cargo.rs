//! Read applicable metadata from cargo.
use serde_derive::Deserialize;
use serde_yaml::from_reader;
use std::env::var_os;
use std::process::{Command, Stdio};
use std::ffi::OsString;

/// Metadata obtained from cargo.
#[derive(Deserialize, Debug)]
pub struct CargoMetadata {
    pub target_directory: String,
}

/// Retrieves metadata via `cargo metadata`. Respects the `CARGO` env var.
pub fn get_cargo_metadata() -> CargoMetadata {
    let mut child = Command::new(var_os("CARGO").unwrap_or(OsString::from("cargo")))
        .args(&["metadata", "--format-version", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    from_reader(child.stdout.take().unwrap()).unwrap()
}
