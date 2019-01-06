# cargo-checkout &emsp; [![Latest Version]][crates.io] [![Rustc Version 1.31+]][rustc]

[Latest Version]: https://img.shields.io/crates/v/cargo-checkout.svg
[crates.io]: https://crates.io/crates/cargo-checkout
[Rustc Version 1.31+]: https://img.shields.io/badge/rustc-1.31+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2018/12/06/Rust-1.31-and-rust-2018.html

Check out copies of your repository at any point in one or many alternate directories.

This makes it easy to:

- Test your index while you interactively stage changes, to make sure each commit passes.
- Create one directory per toolchain version to test all of them at once (in parallel)

# Stability

HERE BE DRAGONS. This extension is in the early stages of development and may
cause data loss or worse. Only use if you're very comfortable with git and have backups.

There may also be backwards incompatible changes for each version.

# Installation

Will be easily installable from crates.io once it's more mature.

```bash
git clone (repo url here)
cd cargo-checkout
cargo install --path .
```

Upgrading
```bash
git pull origin master
cargo install --path . --force
```

# Examples

Test the current index against every toolchain you have installed, aside from the default:
```bash
cargo checkout test
```

Do the above, limiting it to 2 tests at any given time:
```bash
cargo checkout test -j 2
```

Test the current contents of the git index with the default toolchain
```bash
git add foo.rs
# hmm, is it okay if I just commit that file and leave these other changes here?
cargo checkout --single test
```

# Open Questions

Is `checkout` the best name? Other ideas: `treedo`, `partest` (parallel test), `multido`

How does one effectively limit which toolchains to check against? (Current ideas: version range flags, host flags, combinations thereof)

# TODO

- [ ] consider switching to crossbeam-channel so the git checkout can be concurrent with the rest
- [ ] passing args to cargo test
- [ ] general per-dir execution (borrow methodology from `find` `-exec` and `-execdir`)
  - [ ] building a list with -exec will require restructuring how checkouts feed into runcmd
- [ ] support checking out any tree (including working dir)