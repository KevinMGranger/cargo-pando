use super::Checkout;
use crossbeam::channel::unbounded;
use crossbeam::thread::Scope;
use failure::{Error, ResultExt};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, ExitStatus, Stdio};

pub fn run_cmd<'scope, 'env: 'scope>(scope: &'scope Scope<'env>, checkout: &'env Checkout) -> bool {
    match run_cmd_inner(scope, checkout) {
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
) -> Result<ExitStatus, Error> {
    let mut file = File::create(&checkout.output)
        .with_context(|_| format!("error creating output file {}", checkout.output.display()))?;

    checkout.progress.inc(1);
    checkout.progress.set_message("testing");

    // TODO: use cargo from env var
    let mut child = Command::new("cargo")
        .arg(&format!("+{}", checkout.toolchain))
        .arg("test")
        // TODO: look up correct env var for this
        //.env("RUST_TOOLCHAIN", tchain)
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
    }

    Ok(child.wait()?)
}
