use git2::build::CheckoutBuilder;
use git2::{DiffOptions, Repository};
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::process::Command;
use tempdir::TempDir;

struct CargoTestErr(i32);

impl Debug for CargoTestErr {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "`cargo test` returned {}", self.0)
    }
}

impl Display for CargoTestErr {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        Debug::fmt(self, f)
    }
}

impl Error for CargoTestErr {}

fn main() -> Result<(), Box<std::error::Error>> {
    let repo = Repository::open_from_env()?;

    let mut diffopt = DiffOptions::new();
    diffopt.include_untracked(true);

    let num_changed = repo
        .diff_index_to_workdir(None, Some(&mut diffopt))?
        .stats()?
        .files_changed();

    eprintln!("{} changed files", num_changed);
    let target_dir = if num_changed > 0 {
        let tmp = TempDir::new("rust-repo")?.into_path();

        let mut ckopt = CheckoutBuilder::new();
        ckopt.target_dir(&tmp);

        repo.checkout_index(None, Some(&mut ckopt))?;

        tmp
    } else {
        repo.workdir().unwrap().to_path_buf()
    };

    eprintln!("testing in {}", target_dir.display());

    let status = Command::new("cargo")
        .arg("test")
        .current_dir(&target_dir)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(Box::new(CargoTestErr(status.code().unwrap())))
    }
}
