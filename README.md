# cargo-checkout &emsp; [![Latest Version]][crates.io] [![Rustc Version 1.31+]][rustc]

[Latest Version]: https://img.shields.io/crates/v/cargo-checkout.svg
[crates.io]: https://crates.io/crates/cargo-checkout
[Rustc Version 1.31+]: https://img.shields.io/badge/rustc-1.29+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2018/12/06/Rust-1.31-and-rust-2018.html

Check out copies of your repository at any point in one or many alternate directories.

This makes it easy to:

- Test your index while you interactively stage changes, to make sure each commit passes.
- Create one directory per toolchain version to test all of them at once (in parallel, even) (TODO)

# Stability

HERE BE DRAGONS. This extension is in the early stages of development and may
cause data loss or worse. Only use if you're very comfortable with git and have backups.

There may also be backwards incompatible changes for each version.

# TODO

- [ ] proper arg parsing and/or passing args to cargo test
- [ ] support N checkout copies and one-per-toolchain copies
- [ ] switch to a cached directory instead of a temporary one
- [ ] testing convenience flags
- [ ] investigate how `target` is invalidated for some performance wins
- [ ] let user toggle ignorance of untracked files when comparing to see if a checkout is necessary (and rethink how this works in the first place)