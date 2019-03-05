use ::structopt::StructOpt;

/// Perform tasks concurrently over multiple copies of your repo.
/// 
/// See the help for the various subcommands for details.
/// 
/// Note that there is no subcommand to execute _one_ command with each checkout.
/// For that, use print, cut and xargs:
/// # echoes all toolchains on one line instead of each on a separate line
/// cargo pando print | cut -f 1 | xargs echo
#[derive(StructOpt, Debug)]
#[structopt(name = "cargo pando", author = "")]
pub struct Opts {
    /// Check out the index of your repository.
    ///
    /// Mutually exclusive with --copy and --no-copy.
    #[structopt(short, long, conflicts_with = "copy", conflicts_with = "no_copy")]
    pub index: bool,

    /// Copy src and Cargo.{toml,lock} against each toolchain.
    ///
    /// Mutually exclusive with --index and --no-copy.
    #[structopt(short, long)]
    pub copy: bool,

    /// Don't copy any files, use the existing ones in target/pando.
    ///
    /// Mutually exclusive with --index and --copy.
    #[structopt(long)]
    pub no_copy: bool,

    /// Specify one or many toolchains to use. Reads from .travis.yml if unused.
    ///
    /// Mutually exclusive with --all.
    #[structopt(short, long, number_of_values = 1)]
    pub toolchain: Vec<String>,

    /// Use all installed toolchains except for the current default.
    ///
    /// Mutually exclusive with --toolchain.
    #[structopt(short, long, conflicts_with = "toolchain")]
    pub all: bool,

    #[structopt(subcommand)]
    pub action: ActionOpt,
}

#[derive(StructOpt, Debug)]
#[structopt(author = "")]
pub enum ActionOpt {
    #[structopt(name = "test", author = "")]
    /// Runs cargo test on each checkout, with the applicable toolchain.
    CargoTest {
        /// Install the proper toolchain if it's not already present.
        #[structopt(long)]
        install: bool,

        /// Max active tasks. Defaults to number of logical CPUs.
        #[structopt(short, long)]
        jobs: Option<usize>,

        /// Arguments passed along to cargo test.
        test_args: Vec<String>,
    },

    #[structopt(name = "build", author = "")]
    /// Runs cargo build on each checkout, with the applicable toolchain.
    CargoBuild {
        /// Install the proper toolchain if it's not already present.
        #[structopt(long)]
        install: bool,

        /// Max active tasks. Defaults to number of logical CPUs.
        #[structopt(short, long)]
        jobs: Option<usize>,

        /// Arguments passed along to cargo build.
        build_args: Vec<String>,
    },

    /// Any arbitrary cargo subcommand.
    #[structopt(name = "cargo", author = "")]
    CargoAny {
        /// Install the proper toolchain if it's not already present.
        #[structopt(long)]
        install: bool,

        /// Max active tasks. Defaults to number of logical CPUs.
        #[structopt(short, long)]
        jobs: Option<usize>,

        subcommand: String,
        /// Arguments passed along to cargo.
        args: Vec<String>,
    },

    /// Execute the given command once per checkout.
    ///
    /// The directory will be changed to the checkout dir.
    /// Any argument named ``{}`` will be replaced by the toolchain version.
    #[structopt(name = "each", author = "")]
    Each {
        /// Install the proper toolchain if it's not already present.
        #[structopt(long)]
        install: bool,

        /// Max active tasks. Defaults to number of logical CPUs.
        #[structopt(short, long)]
        jobs: Option<usize>,

        utility: String,
        args: Vec<String>,
    },

    /// Copy and do nothing but print the full path of each checkout, one per line.
    /// 
    /// Serves as a useful starting point to run a command across _all_ checkouts at once.
    /// 
    /// # echoes all toolchains on one line instead of each on a separate line
    /// 
    /// cargo pando print | cut -f 1 | xargs echo
    #[structopt(name = "print", author = "")]
    Print,
}

// TODO: this abstraction sucks. Convert just like checkoutsource to 
// an enum of cargo command, each, print
// might be some way around needing scope for cargo that isn't yucky
impl ActionOpt {
    pub fn job_count(&self) -> Option<usize> {
        match self {
            ActionOpt::Each { jobs, .. } => jobs.clone(),
            ActionOpt::CargoTest { jobs, .. } => jobs.clone(),
            ActionOpt::CargoBuild { jobs, .. } => jobs.clone(),
            ActionOpt::CargoAny { jobs, .. } => jobs.clone(),
            ActionOpt::Print => Some(0),
        }
    }
   
    pub fn uses_progress_bars(&self) -> bool {
        match self {
            ActionOpt::Print => false,
            _ => true,
        }
    }

    pub fn uses_workers(&self) -> bool {
        match self {
            ActionOpt::Print => false,
            _ => true,
        }
    }
}
