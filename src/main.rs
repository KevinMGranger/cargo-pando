mod action;
mod cli;
mod copy;
mod git;
mod toolchains;

use cli::{ActionOpt, Opts};
use crossbeam::channel::bounded;
use crossbeam::scope;
use crossbeam::thread::ScopedJoinHandle;
use failure::{bail, format_err, Error, ResultExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::mem::drop;
use std::path::PathBuf;
use structopt::StructOpt;

// the parsed-and-proper program obtained from the structopt Opts.
struct Program {
    toolchains: Vec<String>,
    checkout_source: CheckoutSource,
    action: ActionOpt,
}

fn opts_to_program(opts: Opts) -> Result<Program, Error> {
    Ok(Program {
        toolchains: if opts.all {
            toolchains::get_installed_toolchains()?
        } else if !opts.toolchain.is_empty() {
            opts.toolchain
        } else {
            toolchains::get_toolchains_from_travis()?
        },
        checkout_source: if opts.index {
            CheckoutSource::Index
        } else if opts.no_copy {
            CheckoutSource::None
        } else {
            CheckoutSource::Copy
        },
        action: opts.action,
    })
}

enum CheckoutSource {
    Copy,
    Index,
    None,
}

impl std::fmt::Display for CheckoutSource {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CheckoutSource::Copy => write!(f, "Copying current directory"),
            CheckoutSource::Index => write!(f, "Checking out index"),
            CheckoutSource::None => write!(f, "Using existing checkouts"),
        }
    }
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

impl Program {
    fn run(self) -> Result<(), Error> {
        let Program {
            toolchains,
            checkout_source,
            action,
        } = self;

        if toolchains.is_empty() {
            bail!("no toolchains found");
        }

        // set up progress bars and determine what checkouts will happen
        let style = {
            let longest_tchain_name = toolchains.iter().map(String::len).max().unwrap();

            let template = format!(
                "{{prefix:<{}}} {{pos}}/{{len}} {{bar}} {{elapsed_precise}} {{msg}} ",
                longest_tchain_name
            );

            ProgressStyle::default_bar().template(&template)
        };

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
                // 2: testing
                // 3: done
                let progress = multi.add(ProgressBar::new(3));
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

        // Determine worker count based on number of intended checkouts,
        // type of action, job limit specified for the action, and
        // number of CPU cores.
        let worker_count = if checkouts.len() == 1 {
            1
        } else {
            std::cmp::min(
                checkouts.len(),
                action.job_count().unwrap_or_else(num_cpus::get),
            )
        };

        eprintln!("Using {} workers. {}.", worker_count, checkout_source);

        let result = scope(|scope| -> Result<bool, Error> {
            let (tx, rx) = bounded::<&Checkout>(checkouts.len());

            // spawn workers
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

            // do checkout and send to workers
            let checkout_success = match checkout_source {
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

    opts_to_program(opts)?.run()
}
