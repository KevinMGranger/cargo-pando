use failure::*;
use git2::build::CheckoutBuilder;
use git2::Repository;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::str::FromStr;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "cargo checkout", author = "")]
struct Opts {
    /// How to check out the code. (index|toolchains)
    ///
    /// `index` checks out only the current index and runs the command with the default toolchain.
    /// `toolchains` checks out the current index into one directory per installed toolchain (other than the default) and runs the command, once per directory/toolc
    source: CheckoutSource,
    /// What to run on the code (test|debug)
    ///
    /// `test` runs cargo test on each checkout, with the applicable toolchain.
    /// `debug` merely lists the contents and prints each directory name.
    action: RunCmd,
}

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
#[derive(Debug)]
enum CheckoutSource {
    Index,
    Toolchains,
}

impl FromStr for CheckoutSource {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "index" => CheckoutSource::Index,
            "toolchains" => CheckoutSource::Toolchains,
            _ => bail!("unknown checkout source {}", s),
        })
    }
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
#[derive(Debug)]
enum RunCmd {
    CargoTest,
    Debug,
}

impl FromStr for RunCmd {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "test" => RunCmd::CargoTest,
            "debug" => RunCmd::Debug,
            _ => bail!("unknown action {}", s),
        })
    }
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
    fn new(repo: Repository, opts: Opts) -> Program {
        Program {
            repo,

            src: opts.source,
            run_cmd: opts.action,
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

    // handle being invoked as `cargo checkout`
    if args.len() >= 2 && args[1] == "checkout" {
        args.remove(1);
    }

    let opts = Opts::from_iter(args);

    let program = Program::new(Repository::open_from_env()?, opts);

    program.run()
}
