mod action;
mod cargo;
mod cli;
mod copy;
mod git;
mod toolchains;

use action::run_cmd;
use cargo::CargoMetadata;
use cli::{ActionOpt, Opts};
use crossbeam::channel::bounded;
use crossbeam::scope;
use crossbeam::thread::ScopedJoinHandle;
use failure::{bail, format_err, Error, ResultExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::mem::drop;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

// the parsed-and-proper program obtained from the structopt Opts.
struct Program {
    toolchains: Vec<String>,
    checkout_source: CheckoutSource,
    action: ActionOpt,
    cargo_metadata: CargoMetadata,
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
        cargo_metadata: cargo::get_cargo_metadata(),
    })
}

enum CheckoutSource {
    Copy,
    Index,
    None,
}

impl CheckoutSource {
    fn do_checkout<'checkout>(
        &self,
        checkouts: impl IntoIterator<Item = &'checkout Checkout>,
        mut finished_callback: impl FnMut(&'checkout Checkout),
    ) -> Result<bool, Error> {
        match self {
            CheckoutSource::Index => git::checkout_index(checkouts, finished_callback),
            CheckoutSource::Copy => copy::copy_repo(checkouts, finished_callback),
            CheckoutSource::None => {
                for checkout in checkouts {
                    finished_callback(checkout);
                }
                drop(finished_callback);
                Ok(true)
            }
        }
    }
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
        if self.toolchains.is_empty() {
            bail!("no toolchains found");
        }

        let (checkouts, multi_handle) = {
            let style = {
                let longest_tchain_name = self.toolchains.iter().map(String::len).max().unwrap();

                let template = format!(
                    "{{prefix:<{}}} {{pos}}/{{len}} {{bar}} {{elapsed_precise}} {{msg}} ",
                    longest_tchain_name
                );

                ProgressStyle::default_bar().template(&template)
            };

            let multi = MultiProgress::new();

            if !self.action.uses_progress_bars() {
                multi.set_draw_target(indicatif::ProgressDrawTarget::hidden());
            }

            let all_checkouts = Path::new(&self.cargo_metadata.target_directory).join("pando");

            let checkouts = self
                .toolchains
                .iter()
                .cloned()
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

            (checkouts, multi_handle)
        };

        let success = if !self.action.uses_workers() {
            let print_checkout_name = |checkout: &Checkout| {
                println!("{}", checkout.toolchain);
                checkout.progress.finish();
            };
            self.checkout_source
                .do_checkout(&checkouts, print_checkout_name)?
        } else {
            // Determine worker count based on number of intended checkouts,
            // type of action, job limit specified for the action, and
            // number of CPU cores.
            let worker_count = std::cmp::min(
                checkouts.len(),
                self.action.job_count().unwrap_or_else(num_cpus::get),
            );

            eprintln!("Using {} workers. {}.", worker_count, self.checkout_source);

            scope(|scope| -> Result<bool, Error> {
                let (tx, rx) = bounded::<&Checkout>(checkouts.len());

                let actn = &self.action;
                // spawn workers
                let worker_handles = (0..worker_count)
                    .map(|i| {
                        let rx = rx.clone();
                        scope
                            .builder()
                            .name(format!("worker {}", i))
                            .spawn(move |scope| -> bool {
                                rx.iter()
                                    .map(|checkout| run_cmd(scope, &checkout, actn))
                                    .fold(true, |x, y| x && y)
                            })
                            .with_context(|_| format!("failed to spawn worker {}", i))
                    })
                    .collect::<Result<Vec<ScopedJoinHandle<'_, bool>>, _>>()?;

                // do checkout and send to workers
                let checkout_success = self
                    .checkout_source
                    .do_checkout(&checkouts, move |checkout| tx.send(checkout).unwrap())?;

                Ok(checkout_success
                    && worker_handles
                        .into_iter()
                        .map(|x| x.join().unwrap())
                        .fold(true, |x, y| x && y))
            })
            .map_err(|_| format_err!("panicked"))??
        };

        multi_handle.join().unwrap();

        if !success {
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
