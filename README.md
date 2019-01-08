# cargo-pando &emsp; [![Latest Version]][crates.io] [![Rustc Version 1.31+]][rustc]

[Latest Version]: https://img.shields.io/crates/v/cargo-pando.svg
[crates.io]: https://crates.io/crates/cargo-pando
[Rustc Version 1.31+]: https://img.shields.io/badge/rustc-1.31+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2018/12/06/Rust-1.31-and-rust-2018.html

Perform tasks concurrently over multiple copies of your repo.

Example use cases:

- test your code against multiple rust relases in parallel
- test all commits in a given range in parallel to bisect a bug (TODO)
- do both of the above at the same time (TODO)
- run some other custom command across any of the above checkouts (TODO)

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

# Examples

Test the working directory against every toolchain you have installed, aside from the default:
```bash
cargo pando test
```

Do the above, limiting it to 2 tests at any given time:
```bash
cargo pando -j 2 test
```

Test each toolchain, but only doc tests:
```bash
cargo pando test -- --doc
```

# TODO

- [ ] add support for indicatif
  - [ ] figure out why indicatif is re-printing lines (try doing join immediately after creating all memebrs of the multi)
- [ ] clean up the whole shebang
- [ ] document design philosophy
- [ ] set toolchain with env var before task execution
  - [ ] just refactor task execution in general
- [ ] support working directory copy
- [ ] make checkout happen in target instead of .git
  - [ ] figure out how to get target dir correctly from cargo config
    - [ ] if there's no cargo api for extensions, then is there a subcommand for reading config?
- [ ] refactor checkout source to hide git impls (keep index for now)
- [ ] have it read from travis CI config to determine toolchains to run against
- [ ] toolchain selection flags (see what version parsing cargo uses)
- [ ] invoke subtasks with --message-format=json for better output information
- [ ] determine number of steps for task from dependency list? (means actions will have to run before fully setting up the bars?)
- [ ] tree selection
- [ ] improve toolchain list parsing (maybe should be higher on the list)
- [ ] add support for other exec targets
  - [ ] print
  - [ ] cargo
  - [ ] build
  - [ ] cmdeach / cmdall
  - [ ] shelleach / shellall
- [ ] decide if common toolchain selection stuff should be part of the subcommand or the larger command
- [ ] document using cargo aliases to help with common sub-commands
- [ ] document helpful env vars
- [ ] consider `do` subsubcommand to make multiple actions easier (use square brackets for separation)
- [ ] blog post