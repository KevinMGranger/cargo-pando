# cargo-pando &emsp; [![Latest Version]][crates.io] [![Rustc Version 1.34.2+]][rustc] [![Build Status]][travis_ci]

[Latest Version]: https://img.shields.io/crates/v/cargo-pando.svg
[crates.io]: https://crates.io/crates/cargo-pando
[Rustc Version 1.34.2+]: https://img.shields.io/badge/rustc-1.34.2+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2019/05/14/Rust-1.34.2.html
[Build Status]: https://travis-ci.com/KevinMGranger/cargo-pando.svg?branch=master
[travis_ci]: https://travis-ci.com/KevinMGranger/cargo-pando

Perform tasks concurrently over multiple copies of your repo.

- test your code against multiple rust releases in parallel with a snazzy progress bar
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
    _The configured target dir in your cargo config is respected._
3. Run `rustup run TOOLCHAIN_HERE cargo test` or some other action in each copy of the repo-- in parallel.
    For example, `cargo +1.31.0 test` in `target/pando/1.31.0/working_dir`.

Output is logged to `target/pando/TOOLCHAIN_HERE/output`, and each line is printed next to the progress bar for the checkout.

## Caveats

If your tests rely on external resources, keep in mind they won't be in the expected location.

If there are exclusive resources, you'll have to synchronize access yourself.

More parallelism doesn't always make things faster, especially since compilation
can be IO intensive as well as CPU intensive.

# Examples

See `cargo pando help` for more details.

Test the working directory against the toolchains listed in `.travis.yml`:
```bash
cargo pando test
```

Test against every installed toolchain except the default,
limiting it to 2 `cargo test`s at any given time:
```bash
cargo pando --all test -j 2 
```

Test each specified toolchain, but only doc tests:
```bash
cargo pando -t stable -t beta test -- --doc
```

If you want to run a single command across all of the checkouts at once,
use print, cut, and xargs:
```bash
cargo pando print | cut -f 2 | xargs ls
```

Run an arbitrary command against each checkout,
substituting the name of the toolchain where applicable:
```bash
cargo pando each echo the toolchain '{}' has been copied
```

If the command does not lend itself well to the single line given
by the progress bars, xargs can help again:
```bash
cargo pando print | cut -f 1 | xargs -L 1 -P 2 echo the toolchain is
```

## Git

Test the given toolchain against the _index_ (stage) of your repo.
Useful if you're incrementally adding changes to a commit and you want to check that your work in progress still works.
```bash
cargo pando --index -t stable test
```

# Handy related commands

See how much space the pando directory is taking up:

```bash
du -sh target/pando
```

Get rid of it!

```bash
rm -rf target/pando
```

# Bugs / Open Questions

- How can we know what files need to be copied over?
- How can we know when / what files to delete from checkouts?
- How can we tell the canonical name of a toolchain, to not
    duplicate between their travis representation
    and the --all representation?

# TODO

## 0.3
- [x] get target from cargo metadata instead of assuming
- [x] add support for other exec targets
  - [x] print
  - [x] cargo
  - [x] build
  - [x] cmdeach ~~/ cmdall~~ (have it print and consume it via shell / xargs!)
    - [x] document that
- [x] heck, document everything
- ~~[ ] document using cargo aliases to help with common sub-commands~~
- ~~[ ] document helpful env vars~~

(I can't remember what those last two were about. Oh well.)

## 0.4
- [ ] start writing tests
- [ ] call for (and get) feedback
- [ ] figure out what the earliest compatible rust version is
- [ ] support allowing failures from travis.yml

## 1.0
- [ ] blog post
- [ ] pointing progress bars to stdout
- [ ] answer: would one ever need more than Cargo files, test, and src? (build.rs maybe, and then more?)

## Next
- [ ] invoke subtasks with --message-format=json for better output information?
- [ ] determine number of steps for task from dependency list?
- [ ] colorize / emojify output

## Maybe?
- [ ] tmux integration (might have to refactor when output is created, etc.)
- [ ] Other toolchain selection / isolation mechanisms?
  - [ ] Docker?
  - [ ] Can we arbitrarily support this? Might not be worth it.
- [ ] vastly cleaning up actions
- [ ] make print take args to select field deliniation (whitespace, null, ASCII tabular)