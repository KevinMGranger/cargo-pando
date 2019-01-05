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
}

impl CheckoutSource {
    // TODO: make this an iterator for multi-checkouts?
    fn do_checkout(&self, repo: &Repository, checkouts: &Path) -> Result<Vec<Checkout>, Error> {
        let target = Checkout {
            work_dir: checkouts.join("index").join("workdir"),
        };

        create_dir_all(target.work_dir()).context("create workdir")?;

        let mut ckopt = CheckoutBuilder::new();
        ckopt.target_dir(target.work_dir());
        ckopt.recreate_missing(true);

        repo.checkout_index(None, Some(&mut ckopt))
            .context("index checkout")?;

        Ok(vec![target])
    }
}

/// What command to run on each checked out directory.
enum RunCmd {
    CargoTest,
}

impl RunCmd {
    fn run_cmd(&self, dir: &Path) -> Result<Child, Error> {
        Ok(Command::new("cargo").arg("test").current_dir(dir).spawn()?)
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
            src: CheckoutSource::Index,
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
            self.run_cmd.run_cmd(checkout.work_dir())?.wait()?;
        }

        Ok(())
    }
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
