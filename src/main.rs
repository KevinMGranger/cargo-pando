use failure::*;
use git2::build::CheckoutBuilder;
use git2::Repository;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::fs::{create_dir_all, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::str::FromStr;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "cargo checkout", author = "")]
struct Opts {
    #[structopt(short = "v", long = "verbose")]
    /// Print more information about what's happening to stderr.
    verbose: bool,
    #[structopt(short = "s", long = "single")]
    /// Check out just one copy and run the command with the default toolchain.
    single: bool,

    #[structopt(short = "j")]
    /// How many active tasks should there be at once? Defaults to number of logical CPUs.
    jobs: Option<usize>,
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
    fn new(all_checkouts_dir: &Path, toolchain: Option<String>) -> Checkout {
        let mut work_dir = all_checkouts_dir.to_path_buf();
        if let Some(toolchain) = &toolchain {
            work_dir.push(format!("index-{}", &toolchain));
        } else {
            work_dir.push("index");
        }
        work_dir.join("workdir");
        Checkout {
            work_dir,
            toolchain,
        }
    }

    // will be used for target moving and/or appending log
    #[allow(dead_code)]
    fn root(&self) -> &Path {
        self.work_dir.parent().unwrap()
    }

    fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    fn log_for(&self, task: &str) -> PathBuf {
        self.root().join(task)
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
    fn create_cmd(&self, checkout: &Checkout) -> Result<Command, Error> {
        match self {
            RunCmd::CargoTest => {
                let mut cmd = Command::new("cargo");
                if let Some(toolchain) = &checkout.toolchain {
                    cmd.arg(format!("+{}", toolchain));

                    let file = File::create(checkout.log_for("test"))?;
                    cmd.stdout(file.try_clone()?);
                    cmd.stderr(file);
                }
                cmd.arg("test").current_dir(checkout.work_dir());
                Ok(cmd)
            }
            RunCmd::Debug => {
                println!("{}:", checkout.work_dir().display());
                let mut cmd = Command::new("ls");
                cmd.arg("-al").current_dir(checkout.work_dir());
                Ok(cmd)
            }
        }
    }
}

fn run(repo: Repository, opts: Opts) -> Result<(), Error> {
    let all_checkouts_dir = repo.path().join("cargo-checkout");

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

    let checkouts: Vec<Checkout> = if !opts.single {
        let toolchains = get_toolchains()?;
        toolchains
            .into_iter()
            .map(|toolchain| Checkout::new(&all_checkouts_dir, Some(toolchain)))
            .collect()
    } else {
        vec![Checkout::new(&all_checkouts_dir, None)]
    };

    for checkout in &checkouts {
        create_dir_all(checkout.work_dir())?;

        let mut ckopt = CheckoutBuilder::new();
        ckopt.target_dir(checkout.work_dir());
        ckopt.recreate_missing(true);

        if opts.verbose {
            eprintln!("checking out into {}", checkout.work_dir().display());
        }

        repo.checkout_index(None, Some(&mut ckopt))?;
    }

    if let Some(num) = opts.jobs {
        ThreadPoolBuilder::new().num_threads(num).build_global()?;
    }

    let run_cmd = &opts.action;
    let verbose = opts.verbose;

    let mut statuses = Vec::new();
    checkouts
        .par_iter()
        .map(|checkout| -> Result<ExitStatus, Error> {
            // TODO: errors should be printed immediately, also simplifies exit
            let mut cmd = run_cmd.create_cmd(&checkout)?;

            if verbose {
                eprintln!("running {:?} in {}", cmd, checkout.work_dir().display());
            }

            let status = cmd.status()?;

            if status.success() {
                println!(
                    "{:?} in {} exited successfully",
                    cmd,
                    checkout.work_dir().display()
                );
            } else {
                println!(
                    "{:?} in {} exited with {}",
                    cmd,
                    checkout.work_dir().display(),
                    status.code().unwrap()
                );
            }

            Ok(status)
        })
        .collect_into_vec(&mut statuses);

    // only exit successfully if all subtasks were successful
    let mut exit = Ok(());
    for status in statuses {
        match status {
            Ok(status) if !status.success() => {
                if exit.is_ok() {
                    exit = Err(format_err!("A command did not exit successfully"));
                }
            }
            Err(err) => {
                if exit.is_ok() {
                    exit = Err(err);
                }
            }
            _ => {}
        }
    }

    exit
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

    run(Repository::open_from_env()?, opts)
}
