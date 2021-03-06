// TODO: redo this whole module basically
use super::cli::ActionOpt;
use super::Checkout;
use crossbeam::channel::unbounded;
use crossbeam::thread::Scope;
use failure::{Error, ResultExt};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, ExitStatus, Stdio};

fn make_an_each_command(
    install: bool,
    toolchain: &str,
    cargo: bool,
    replacements: bool,
    utility: &str,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> Command {
    let mut cmd = Command::new("rustup");
    cmd.arg("run");
    if install {
        cmd.arg("--install");
    }
    cmd.arg(toolchain);

    if cargo {
        cmd.arg("cargo");
    }

    cmd.arg(utility);
    if replacements {
        for arg in args {
            if arg.as_ref() == "{}" {
                cmd.arg(toolchain);
            } else {
                cmd.arg(arg);
            }
        }
    } else {
        cmd.args(args);
    }
    cmd
}

fn command_from_action(toolchain: &str, action: &ActionOpt) -> Option<Command> {
    let (install, cargo, replacements, util, args) = match action {
        ActionOpt::CargoTest {
            install,
            test_args: args,
            ..
        } => (install, true, false, "test", args),
        ActionOpt::CargoBuild {
            install,
            build_args: args,
            ..
        } => (install, true, false, "build", args),
        ActionOpt::CargoAny {
            install,
            subcommand: util,
            args,
            ..
        } => (install, true, false, util.as_str(), args),
        ActionOpt::Each {
            install,
            utility: util,
            args,
            ..
        } => (install, false, true, util.as_str(), args),
        _ => unimplemented!()
    };

    Some(make_an_each_command(
        *install,
        toolchain,
        cargo,
        replacements,
        util,
        args,
    ))
}

pub fn run_cmd<'scope, 'env: 'scope>(
    scope: &'scope Scope<'env>,
    checkout: &'env Checkout,
    action: &'env ActionOpt,
) -> bool {
    match run_cmd_inner(scope, checkout, action) {
        Err(e) => {
            checkout
                .progress
                .finish_with_message(&format!("failure: {}", e));
            false
        }
        Ok(status) => match (status.success(), status.code()) {
            (true, _) => {
                checkout.progress.finish_with_message("success");
                true
            }
            (false, Some(code)) => {
                checkout.progress.finish_with_message(&format!(
                    "failure: status {}. Check output in {}",
                    code,
                    checkout.output.display()
                ));
                false
            }
            (false, None) => {
                checkout.progress.finish_with_message(&format!(
                    "failure: status unknown. Check output in {}",
                    checkout.output.display()
                ));
                false
            }
        },
    }
}

fn run_cmd_inner<'scope, 'env: 'scope>(
    scope: &'scope Scope<'env>,
    checkout: &'env Checkout,
    action: &'env ActionOpt,
) -> Result<ExitStatus, Error> {
    let mut file = File::create(&checkout.output)
        .with_context(|_| format!("error creating output file {}", checkout.output.display()))?;

    checkout.progress.inc(1);
    checkout.progress.set_message("testing");

    let mut cmd = command_from_action(&checkout.toolchain, action).unwrap();

    let mut child = cmd
        .current_dir(&checkout.working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|_| format!("error spawning cargo +{} test", checkout.toolchain))?;

    checkout.progress.enable_steady_tick(500); // ms

    let (lines_tx, lines_rx) = unbounded::<String>();

    let stdout = child.stdout.take().unwrap();
    let stdout_tx = lines_tx.clone();
    scope.spawn(move |_| {
        for line in BufReader::new(stdout).lines() {
            stdout_tx.send(line.unwrap()).unwrap();
        }
    });

    let stderr = child.stderr.take().unwrap();
    let stderr_tx = lines_tx;
    scope.spawn(move |_| {
        for line in BufReader::new(stderr).lines() {
            let line = line.unwrap();
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                checkout
                    .progress
                    .set_message(&format!("testing: {}", trimmed));
            }
            stderr_tx.send(line).unwrap();
        }
    });

    for line in lines_rx {
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
    }

    Ok(child.wait()?)
}
