use failure::*;
use git2::build::CheckoutBuilder;
use git2::Repository;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "cargo checkout", author = "")]
struct Opts {}

/// A checkout is the ref-specific checkout in .git/cargo-checkout
/// We keep the work dir since getting the parent is easy, and
/// we don't always use a log file, so we do that lazily.
struct Checkout {
    work_dir: PathBuf,
    /// The toolchain to use on this dir, if applicable
    toolchain: Option<String>,
}

impl Checkout {
    fn root(&self) -> &Path {
        self.work_dir.parent().unwrap()
    }

    fn work_dir(&self) -> &Path {
        &self.work_dir
    }
}

/// Where to check out from.
enum CheckoutSource {
    Index,
    Toolchains,
}

impl CheckoutSource {
    // TODO: make this an iterator for multi-checkouts?
    fn do_checkout(&self, repo: &Repository, checkouts: &Path) -> Result<Vec<Checkout>, Error> {
        match self {
            CheckoutSource::Index => {
                let target = Checkout {
                    work_dir: checkouts.join("index").join("workdir"),
                    toolchain: None,
                };

                create_dir_all(target.work_dir()).context("create workdir")?;

                let mut ckopt = CheckoutBuilder::new();
                ckopt.target_dir(target.work_dir());
                ckopt.recreate_missing(true);

                repo.checkout_index(None, Some(&mut ckopt))
                    .context("index checkout")?;

                Ok(vec![target])
            }
            CheckoutSource::Toolchains => {
                let toolchains = get_toolchains()?;

                toolchains
                    .into_iter()
                    .map(|toolchain| -> Result<Checkout, Error> {
                        let target = checkouts
                            .join(format!("index-{}", &toolchain))
                            .join("workdir");
                        create_dir_all(&target)?;

                        let mut ckopt = CheckoutBuilder::new();
                        ckopt.target_dir(&target);
                        ckopt.recreate_missing(true);

                        repo.checkout_index(None, Some(&mut ckopt))
                            .context("index checkout")?;

                        Ok(Checkout {
                            work_dir: target,
                            toolchain: Some(toolchain),
                        })
                    })
                    .collect()
            }
        }
    }
}

/// What command to run on each checked out directory.
enum RunCmd {
    CargoTest,
    Debug,
}

impl RunCmd {
    fn run_cmd(&self, checkout: &Checkout) -> Result<Child, Error> {
        match self {
            RunCmd::CargoTest => {
                let mut cmd = Command::new("cargo");
                if let Some(toolchain) = &checkout.toolchain {
                    cmd.arg(format!("+{}", toolchain));
                }
                cmd.arg("test").current_dir(checkout.work_dir());
                println!("{:?}", cmd);
                Ok(cmd.spawn()?)
            }
            RunCmd::Debug => {
                println!("{}:", checkout.work_dir().display());
                Ok(Command::new("ls")
                    .arg("-al")
                    .current_dir(checkout.work_dir())
                    .spawn()?)
            }
        }
    }
}

struct Program {
    repo: Repository,
    src: CheckoutSource,
    run_cmd: RunCmd,
}

impl Program {
    fn new(repo: Repository, _opts: Opts) -> Program {
        Program {
            repo,
            // TODO
            src: CheckoutSource::Toolchains,
            // TODO
            run_cmd: RunCmd::CargoTest,
        }
    }

    fn run(&self) -> Result<(), Error> {
        let all_checkouts_dir = self.repo.path().join("cargo-checkout");

        create_dir_all(&all_checkouts_dir).context("creating checkouts dir")?;
        if !all_checkouts_dir.is_dir() {
            if all_checkouts_dir.exists() {
                bail!(
                    "checkout directory {} exists but is not a directory",
                    all_checkouts_dir.display()
                );
            }

            create_dir_all(&all_checkouts_dir).context("couldn't create checkouts dir")?;
        }

        let target_dirs = self.src.do_checkout(&self.repo, &all_checkouts_dir)?;

        // TODO: job limiting / parallelism
        for checkout in target_dirs {
            self.run_cmd.run_cmd(&checkout)?.wait()?;
        }

        Ok(())
    }
}

/// Get a list of installed rust toolchains, excluding the current default
fn get_toolchains() -> Result<Vec<String>, Error> {
    let output = Command::new("rustup")
        .args(&["toolchain", "list"])
        .output()?;

    if !output.status.success() {
        bail!("couldn't list toolchains");
    }

    let output = String::from_utf8(output.stdout)?;

    Ok(output
        .lines()
        .filter(|x| !x.ends_with("(default)"))
        .map(String::from)
        .collect())
}

fn main() -> Result<(), Error> {
    let mut args = std::env::args().collect::<Vec<String>>();

    if args.len() >= 2 {
        if args[1] == "checkout" {
            args.remove(1);
        }
    }

    let opts = Opts::from_iter(args);

    let program = Program::new(Repository::open_from_env()?, opts);

    program.run()
}
