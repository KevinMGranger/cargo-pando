use ::structopt::StructOpt;

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
    #[structopt(name = "test")]
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

    #[structopt(name = "build")]
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
    #[structopt(name = "cargo")]
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
    #[structopt(name = "each")]
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
    #[structopt(name = "print")]
    Print,
}

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
