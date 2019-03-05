//! Various toolchain list sources.
pub use self::travis::get_toolchains_from_travis;

use failure::{bail, Error, ResultExt};
use std::process::Command;

mod travis {
    use failure::*;
    use serde_derive::Deserialize;
    use serde_yaml::from_reader;
    use std::fs::File;

    #[derive(Deserialize, Debug)]
    struct TravisConfig {
        language: String,
        rust: Vec<String>,
    }

    /// Get all of the toolchains listed in `.travis.yml`.
    /// 
    /// # Failures
    /// 
    /// If `.travis.yml` is missing, doesn't match the expected structure,
    /// or the language isn't `rust`, there will be an error.
    pub fn get_toolchains_from_travis() -> Result<Vec<String>, Error> {
        let cwd = std::env::current_dir()
            .context("could not determine current dir when trying to open travis")?;
        let file = File::open(cwd.join(".travis.yml")).context("Could not open travis.yml")?;
        let config: TravisConfig = from_reader(file).context(".travis.yml was malformed")?;
        if config.language != "rust" {
            bail!("travis config was for '{}', not 'rust'", config.language)
        }

        Ok(config.rust)
    }

}

/// Get a list of installed rust toolchains, excluding the current default
pub fn get_installed_toolchains() -> Result<Vec<String>, Error> {
    let output = Command::new("rustup")
        .args(&["toolchain", "list"])
        .output()
        .context("could not execute rustup to list toolchains")?;

    if !output.status.success() {
        bail!("couldn't list toolchains");
    }

    let output =
        String::from_utf8(output.stdout).context("rustup output contained invalid utf-8")?;

    Ok(output
        .lines()
        .filter(|x| !x.ends_with("(default)"))
        .map(String::from)
        .collect())
}
