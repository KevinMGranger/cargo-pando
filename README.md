# cargo-pando &emsp; [![Latest Version]][crates.io] [![Rustc Version 1.32+]][rustc] [![Build Status]][travis_ci]

[Latest Version]: https://img.shields.io/crates/v/cargo-pando.svg
[crates.io]: https://crates.io/crates/cargo-pando
[Rustc Version 1.32+]: https://img.shields.io/badge/rustc-1.32+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2019/01/17/Rust-1.32.0.html
[Build Status]: https://travis-ci.com/KevinMGranger/cargo-pando.svg?branch=master
[travis_ci]: https://travis-ci.com/KevinMGranger/cargo-pando

Perform tasks concurrently over multiple copies of your repo.

Example use cases:

- test your code against multiple rust releases in parallel
- test the index / stage of your repo to validate incremental changes
- do both of the above at the same time
- run some other custom command across any of the above checkouts

The name pando comes from the [clonal colony of "multiple" trees that are actually one single organism](https://en.wikipedia.org/wiki/Pando_(tree)). It is latin for "I spread out".

# Stability

HERE BE DRAGONS. This extension is in the early stages of development and may
cause data loss or worse. Only use if you're very comfortable with git and have backups.

There may also be backwards incompatible changes for each version.

# Installation

Will be easily installable from crates.io once it's more mature.

```bash
git clone (repo url here)
cd cargo-pando
cargo install --path .
```

Upgrading
```bash
git pull origin master
cargo install --path . --force
```

# How it Works

1. Figure out what toolchains to run against, either from the CLI, `.travis.yml`, or just using all the installed ones.
2. Create a copy of the repo's code in `target/pando` _per toolchain_, e.g. `target/pando/1.31.0`. __Note that this is destructive.__
3. Run `cargo +TOOLCHAIN_HERE test` or some other action in each copy of the repo.
   For example, `cargo +1.31.0 test` in `target/pando/1.31.0/working_dir`.

Output is logged to `target/pando/TOOLCHAIN_HERE/output`.

# Examples

See `cargo pando help` for more details.

Test the working directory against the toolchains listed in `.travis.yml`:
```bash
cargo pando test
```

Test against every installed toolchain except the default, limiting it to 2 `cargo test`s at any given time:
```bash
cargo pando --all test -j 2 
```

Test each specified toolchain, but only doc tests:
```bash
cargo pando -t stable -t beta test -- --doc
```

## Git

Test the given toolchain against the _index_ (stage) of your repo.
Useful if you're incrementally adding changes to a commit and you want to check that your work in progress still works.
```bash
cargo pando --index -t stable test
```

# TODO

## 0.3
- [x] get target from cargo metadata instead of assuming
- [x] add support for other exec targets
  - [x] print
  - [x] cargo
  - [x] build
  - [x] cmdeach ~~/ cmdall~~ (have it print and consume it via shell / xargs!)
    - [ ] document that
- [ ] heck, document everything
- [ ] document using cargo aliases to help with common sub-commands
- [ ] document helpful env vars

## 0.4
- [ ] start writing tests
- [ ] call for (and get) feedback
- [ ] figure out what the earliest compatible rust version is
- [ ] support allowing failures from travis.yml

## 1.0
- [ ] blog post
- [ ] pointing progress bars to stdout

## Next
- [ ] invoke subtasks with --message-format=json for better output information?
- [ ] determine number of steps for task from dependency list?
- [ ] colorize / emojify output
- [ ] print0?

## Maybe?
- [ ] tmux integration (might have to refactor when output is created, etc.)
- [ ] Other toolchain selection / isolation mechanisms?
  - [ ] Docker?
  - [ ] Can we arbitrarily support this? Might not be worth it.
- [ ] vastly cleaning up actions