use super::Checkout;
use crossbeam::channel::Sender;
use failure::{Error, ResultExt};
use std::fs::{copy, create_dir, remove_dir_all, remove_file, Metadata};
use std::io;
use std::path::{Path, PathBuf};

struct DirWalker(Vec<PathBuf>);

impl DirWalker {
    fn new(path: PathBuf) -> DirWalker {
        DirWalker(vec![path])
    }
    fn inner_next(&mut self) -> io::Result<Option<(PathBuf, Metadata)>> {
        Ok(if let Some(path) = self.0.pop() {
            let meta = path.metadata()?;

            if meta.is_dir() {
                let children = path.read_dir()?;
                for child in children {
                    self.0.push(child?.path());
                }
            }

            Some((path, meta))
        } else {
            None
        })
    }
}

impl Iterator for DirWalker {
    type Item = io::Result<(PathBuf, Metadata)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner_next() {
            Ok(Some(x)) => Some(Ok(x)),
            Ok(None) => None,
            Err(x) => Some(Err(x)),
        }
    }
}

const CARGO_TOML: &'static str = "Cargo.toml";
const CARGO_LOCK: &'static str = "Cargo.lock";

fn get_all_copy_targets(wdir: &Path) -> io::Result<Vec<(PathBuf, Metadata)>> {
    let cargo_toml = {
        let path = wdir.join(CARGO_TOML);
        let meta = path.metadata()?;
        (path, meta)
    };
    let cargo_lock = {
        let path = wdir.join(CARGO_LOCK);
        let meta = path.metadata()?;
        (path, meta)
    };

    let mut src_tree =
        DirWalker::new(wdir.join("src")).collect::<io::Result<Vec<(PathBuf, Metadata)>>>()?;

    src_tree.push(cargo_toml);
    src_tree.push(cargo_lock);

    Ok(src_tree)
}

fn do_copy((src, meta): &(PathBuf, Metadata), target_dir: &Path) -> io::Result<()> {
    let target = target_dir.join(src);

    let target = &target;
    match (meta.is_dir(), target.exists()) {
        (true, false) => {
            create_dir(target)?;
        }
        (false, false) => {
            copy(src, target)?;
        }
        (true, true) => {
            if target.is_dir() {
                remove_dir_all(target)?;
            } else {
                remove_file(target)?;
            }
            create_dir(target)?;
        }
        (false, true) => {
            if target.is_dir() {
                remove_dir_all(target)?;
            } else {
                remove_file(target)?;
            }
            copy(src, target)?;
        }
    }

    Ok(())
}

pub fn copy_repo<'checkout, I>(
    checkouts: I,
    worker_queue: Sender<&'checkout Checkout>,
) -> Result<bool, Error>
where
    I: IntoIterator<Item = &'checkout Checkout>,
{
    let wdir = std::env::current_dir()?;
    let mut src = get_all_copy_targets(&wdir).context("Error reading copy sources from repo")?;

    for file in &mut src {
        file.0 = file.0.strip_prefix(&wdir)?.into();
    }

    let file_count = src.len();
    let copying_message = |index| format!("copying file {} of {}", index + 1, file_count);

    let mut all_successful = true;
    'checkouts: for checkout in checkouts {
        std::fs::create_dir_all(&checkout.working_dir)?; // TODO isolate
        for (i, file) in src.iter().enumerate() {
            checkout.progress.set_message(&copying_message(i));
            if let Err(e) = do_copy(file, &checkout.working_dir) {
                checkout
                    .progress
                    .finish_with_message(&format!("error copying {}", file.0.display()));
                all_successful = false;
                continue 'checkouts;
            }
        }
        checkout
            .progress
            .set_message("copied, waiting on available worker");
        checkout.progress.inc(1);
        worker_queue.send(checkout).unwrap();
    }

    Ok(all_successful)
}
