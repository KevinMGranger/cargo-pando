# cargo-checkout &emsp; [![Latest Version]][crates.io] [![Rustc Version 1.31+]][rustc]

[Latest Version]: https://img.shields.io/crates/v/cargo-checkout.svg
[crates.io]: https://crates.io/crates/cargo-checkout
[Rustc Version 1.31+]: https://img.shields.io/badge/rustc-1.31+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2018/12/06/Rust-1.31-and-rust-2018.html

Check out copies of your repository at any point in one or many alternate directories.

This makes it easy to:

- Test your index while you interactively stage changes, to make sure each commit passes.
- Create one directory per toolchain version to test all of them at once (in parallel, even (TODO))

# Stability

HERE BE DRAGONS. This extension is in the early stages of development and may
cause data loss or worse. Only use if you're very comfortable with git and have backups.

There may also be backwards incompatible changes for each version.

# Open Questions

Is `checkout` the best name? Other ideas: `treedo`, `partest` (parallel test), `multido`

How does one effectively limit which toolchains to check against? (Current ideas: version range flags, host flags, combinations thereof)

# TODO

- [ ] verbose output
- [ ] consider if toolchains should be the default and single checkout should be the exception
- [ ] passing args to cargo test
- [ ] general per-dir execution (borrow methodology from `find` `-exec` and `-execdir`)
  - [ ] building a list with -exec will require restructuring how checkouts feed into runcmd
- [ ] concurrent execution / logging per dir
- [ ] support checking out any tree (including working dir)