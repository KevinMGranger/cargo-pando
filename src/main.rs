use crossbeam::channel::bounded;
use crossbeam::scope;
use failure::*;
use git2::build::CheckoutBuilder;
use git2::Repository;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use num_cpus;
use std::fs::{create_dir_all, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
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

    #[structopt(subcommand)]
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
        work_dir.push("workdir");
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
#[derive(StructOpt, Debug)]
#[structopt(author = "")]
enum RunCmd {
    #[structopt(name = "test")]
    /// runs cargo test on each checkout, with the applicable toolchain.
    CargoTest { test_args: Vec<String> },
    #[structopt(name = "debug")]
    /// lists the contents and prints each directory name.
    Debug,
}

impl RunCmd {
    fn create_cmd(&self, checkout: &Checkout) -> Result<Command, Error> {
        match self {
            RunCmd::CargoTest { test_args } => {
                let mut cmd = Command::new("cargo");
                if let Some(toolchain) = &checkout.toolchain {
                    cmd.arg(format!("+{}", toolchain));

                    let file = File::create(checkout.log_for("test"))?;
                    cmd.stdout(file.try_clone()?);
                    cmd.stderr(file);
                }
                cmd.arg("test")
                    .args(test_args)
                    .current_dir(checkout.work_dir());
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

fn run_action(checkout: &Checkout, opts: &Opts) -> Result<ExitStatus, Error> {
    // TODO: errors should be printed immediately, also simplifies exit
    let mut cmd = opts.action.create_cmd(&checkout)?;

    if opts.verbose {
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
}

fn run(repo: &Repository, opts: &Opts) -> Result<(), Error> {
    let jobs = opts.jobs.unwrap_or_else(num_cpus::get);
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

    let any_failure = std::sync::atomic::AtomicBool::new(false);
    scope(|scope| -> Result<(), Error> {
        let checkouts: Vec<Checkout> = if !opts.single {
            let toolchains = get_toolchains()?;
            toolchains
                .into_iter()
                .map(|toolchain| Checkout::new(&all_checkouts_dir, Some(toolchain)))
                .collect()
        } else {
            vec![Checkout::new(&all_checkouts_dir, None)]
        };

        let (sender, recv) = bounded::<Checkout>(checkouts.len());

        let task_count = std::cmp::min(checkouts.len(), jobs);

        for i in 0..task_count {
            if opts.verbose {
                eprintln!("spawning worker {}", i);
            }
            let my_recv = recv.clone();
            scope.spawn(|_| {
                for checkout in my_recv {
                    let status = run_action(&checkout, opts);
                    match status {
                        Ok(status) if !status.success() => {
                            any_failure.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                        Err(_) => {
                            any_failure.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                        _ => {}
                    }
                }
            });
        }

        for checkout in checkouts {
            create_dir_all(checkout.work_dir())?;

            let mut ckopt = CheckoutBuilder::new();
            ckopt.target_dir(checkout.work_dir());
            ckopt.recreate_missing(true);

            if opts.verbose {
                eprintln!("checking out into {}", checkout.work_dir().display());
            }

            repo.checkout_index(None, Some(&mut ckopt))?;

            sender.send(checkout)?;
        }
        std::mem::drop(sender);

        Ok(())
    })
    .map_err(|_| format_err!("parallelism error"))??;

    if any_failure.into_inner() {
        bail!("a sub task failed");
    } else {
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
    // let mut args = std::env::args().collect::<Vec<String>>();

    // // handle being invoked as `cargo checkout`
    // if args.len() >= 2 && args[1] == "checkout" {
    //     args.remove(1);
    // }

    // let opts = Opts::from_iter(args);

    // run(&Repository::open_from_env()?, &opts)

    let sleep = |s| std::thread::sleep(std::time::Duration::from_secs(s));

    let format = "{prefix} {pos}/{len} {bar} {msg} {elapsed}";

    let style = ProgressStyle::default_bar().template(format);

    let toolchains = get_toolchains()?;
    scope(|scope| {
        let multi = MultiProgress::new();

        // let iter = toolchains
        //     .iter()
        //     .map(|tc| {
        //         let bar = ProgressBar::new(3);
        //         bar.set_style(style.clone());
        //         bar.set_prefix(&tc);
        //         bar.set_message("checking out");
        //         (tc, multi.add(bar))
        //     })
        //     .collect::<Vec<(&String, ProgressBar)>>();
        //multi.set_move_cursor(true);
        println!("in between");
        sleep(2);
        for toolchain in toolchains {
            let bar = multi.add(ProgressBar::new(3));
            bar.set_style(style.clone());
            bar.set_prefix(&toolchain);
            bar.set_message("checking out");

            scope.spawn(move |_| {
                //println!("spawnt a thread for {}", toolchain);
                bar.tick();
                sleep(1);
                bar.inc(1); // 1
                bar.set_message("building");
                sleep(3);
                bar.inc(1); // 2
                bar.set_message("testing");
                sleep(2);
                bar.finish_with_message("status: ");
            });
        }
        multi.join().unwrap();
    })
    .unwrap();

    Ok(())
}
