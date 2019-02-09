mod action;
mod copy;
mod git;
mod toolchains;

use self::toolchains::get_toolchains_from_travis;
use crossbeam::channel::bounded;
use crossbeam::scope;
use crossbeam::thread::ScopedJoinHandle;
use failure::{bail, format_err, Error, ResultExt};
use git2::build::CheckoutBuilder;
use git2::Repository;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::mem::drop;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "cargo pando", author = "")]
struct Opts {
    #[structopt(short, long)]
    /// Be verbose about the tasks run.
    verbose: bool,

    /// checkout the index
    #[structopt(short, long, conflicts_with = "copy", conflicts_with = "no_copy")]
    index: bool,

    /// copy the working dir
    #[structopt(short, long)]
    copy: bool,

    /// don't copy
    #[structopt(long)]
    no_copy: bool,

    /// which toolchains to test against. Reads from .travis.yml by default
    #[structopt(short, long)]
    toolchain: Vec<String>,

    /// test against all installed toolchains
    #[structopt(short, long, conflicts_with = "toolchain")]
    all: bool,

    #[structopt(subcommand)]
    action: ActionOpt,
}

enum CheckoutSource {
    Copy,
    Index,
    None,
}

impl From<&Opts> for CheckoutSource {
    fn from(opts: &Opts) -> Self {
        // TODO: panic when invariants are violated
        if opts.index {
            CheckoutSource::Index
        } else if opts.no_copy {
            CheckoutSource::None
        } else {
            CheckoutSource::Copy
        }
    }
}

#[derive(StructOpt, Debug)]
#[structopt(author = "")]
enum ActionOpt {
    #[structopt(name = "test")]
    /// runs cargo test on each checkout, with the applicable toolchain.
    CargoTest {
        #[structopt(short, long)]
        /// How many active tasks should there be at once? Defaults to number of logical CPUs.
        jobs: Option<usize>,

        test_args: Vec<String>,
    },
}

/// A checkout represents a ready-to-go copy of the repository
/// with relevant metadata (e.g. the toolchain it represents)
pub struct Checkout {
    toolchain: String,
    working_dir: PathBuf,
    output: PathBuf,
    progress: ProgressBar,
    // TODO: allowed to fail?
}

/// Determine worker count based on number of intended checkouts,
/// type of action, job limit specified for the action, and
/// number of CPU cores.
// TODO: replace toolchains with some sort of `CheckoutIntent`
fn worker_count(checkout_count: usize, opt: &Opts) -> usize {
    if checkout_count == 1 {
        return 1;
    }

    match opt.action {
        ActionOpt::CargoTest { jobs, .. } => {
            if let Some(job_arg) = jobs {
                std::cmp::min(job_arg, checkout_count)
            } else {
                std::cmp::min(num_cpus::get(), checkout_count)
            }
        }
    }
}

fn main() -> Result<(), Error> {
    let args = std::env::args().enumerate().filter_map(|(i, arg)| {
        // handle being invoked as a cargo subcommand (will have pando passed as arg 1)
        // as well as on our own (e.g. cargo run in this dir, no extra arg 1)
        if i == 1 && arg == "pando" {
            None
        } else {
            Some(arg)
        }
    });

    let opts = Opts::from_iter(args);

    // let toolchains = get_toolchains()?;
    let toolchains = get_toolchains_from_travis()?;

    if toolchains.is_empty() {
        bail!("no toolchains found in travis"); // TODO handle this better. Is this an error condition or just a no-op?
    } else if opts.verbose {
        eprintln!("Loaded {} toolchains", toolchains.len())
    }

    let longest_tchain_name = toolchains.iter().map(String::len).max().unwrap();

    let template = format!(
        "{{prefix:<{}}} {{pos}}/{{len}} {{bar}} {{elapsed_precise}} {{msg}} ",
        longest_tchain_name
    );

    let style: ProgressStyle = ProgressStyle::default_bar().template(&template);

    let multi = MultiProgress::new();

    // TODO: get this from cargo metadata instead
    let mut all_checkouts = std::env::current_dir()?;
    all_checkouts.push("target");
    all_checkouts.push("pando");

    let checkouts = toolchains
        .into_iter()
        .map(|toolchain| {
            // 0: waiting for checkout
            // 1: checked out, waiting on test
            // 2: tested
            // experiemnting with 3 for total time thing
            let progress = multi.add(ProgressBar::new(2));
            progress.set_style(style.clone());
            progress.set_prefix(&toolchain);
            progress.set_message("waiting to be copied");

            let checkout = all_checkouts.join(&toolchain);

            Checkout {
                toolchain,
                working_dir: checkout.join("working_dir"),
                output: checkout.join("output"),
                progress: progress.into(),
            }
        })
        .collect::<Vec<Checkout>>();

    let multi_handle = std::thread::spawn(move || {
        multi.join().unwrap();
    });

    let worker_count = worker_count(checkouts.len(), &opts);

    let result = scope(|scope| -> Result<bool, Error> {
        let (tx, rx) = bounded::<&Checkout>(checkouts.len());

        let worker_handles = (0..worker_count)
            .map(|i| {
                let rx = rx.clone();
                scope
                    .builder()
                    .name(format!("worker {}", i))
                    .spawn(move |scope| -> bool {
                        rx.iter()
                            .map(|checkout| action::run_cmd(scope, &checkout))
                            .fold(true, |x, y| x && y)
                    })
                    .with_context(|_| format!("failed to spawn worker {}", i))
            })
            .collect::<Result<Vec<ScopedJoinHandle<'_, bool>>, _>>()?;

        let checkout_success = match CheckoutSource::from(&opts) {
            CheckoutSource::Index => git::checkout_index(&checkouts, tx),
            CheckoutSource::Copy => copy::copy_repo(&checkouts, tx),
            CheckoutSource::None => {
                for checkout in &checkouts {
                    // TODO: message
                    tx.send(checkout).unwrap();
                }
                drop(tx);
                Ok(true)
            }
        }?;

        Ok(checkout_success
            && worker_handles
                .into_iter()
                .map(|x| x.join().unwrap())
                .fold(true, |x, y| x && y))
    })
    .map_err(|_| format_err!("panicked"))??;

    multi_handle.join().unwrap();

    if !result {
        std::process::exit(1);
    } else {
        Ok(())
    }
}
