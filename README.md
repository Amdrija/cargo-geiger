cargo-geiger ☢
===============

A program that list statistics related to usage of unsafe Rust code in a Rust
crate and all its dependencies.

This project is in its current state a quick-n-dirty, glued together, remix of
two other cargo plugin projects:
<https://github.com/icefoxen/cargo-osha> and
<https://github.com/sfackler/cargo-tree>.


Usage
-----

1. `cargo install cargo-geiger`
2. Navigate to the same directory as the Cargo.toml you want to analyze.
3. `cargo geiger`


Output examples
---------------

### Default output format:
![Example output](https://user-images.githubusercontent.com/3704611/42893683-54f16930-8ab5-11e8-87a5-785fe4a1d5d9.png)


Why even care about unsafe Rust usage?
--------------------------------------

When and why to use unsafe Rust is out of scope for this project, it is simply
a tool that provides information to aid auditing and hopefully to guide
dependency selection. It is however the opinion of the author of this project
that __libraries choosing to abstain from unsafe Rust usage when possible should
be promoted__.

This project is an attempt to create pressure against __unnecessary__ usage of
unsafe Rust in public Rust libraries.


Why the name?
-------------

<https://en.wikipedia.org/wiki/Geiger_counter>

Unsafe Rust and ionizing radiation have something in common, they are both
inevitable in some situations and both should preferably be safely contained!


Known issues
------------

- Unsafe code inside macros are not detected. Needs macro expansion(?).
- Unsafe code generated by `build.rs` are probably not detected.
- Error handling is missing for the most part and there are plenty of panics lurking.
- Proper logging should be sorted out.
- Command line flags needs some more review and refactoring for this project.
- Will continue on syn parse errors. Needs a new command line flag and should
  default to exit on errors(?).
- Could probably benefit from parallelization. One `.rs` file per core should
  be parsed at all times.
- More on the github issue tracker.


Roadmap
-------

- There should be no false negatives. All unsafe code should be identified.
- Refactoring and general cleanup.
- Proper error handling using Result.
- An optional whitelist file at the root crate level to specify crates that are
  trusted to use unsafe (should only have an effect if placed in the root
  project).
- Additional output formats


Changelog
---------

### 0.4.0
 - Filters out tests by default. Tests can be included by using
   `--include-tests`. The test code is filted out by looking for the attribute
   `#[test]` on functions and `#[cfg(test)]` on modules.

### 0.3.1
 - Some bugfixes related to cargo workspace path handling.
 - Slightly better error messages in some cases.

### 0.3.0
 - Intercepts `rustc` calls and reads the `.d` files generated by `rustc` to
   identify which `.rs` files are used by the build. This allows a crate that
   contains `.rs` files with unsafe code usage to pass as "green" if the unsafe
   code isn't used by the build.
 - Each metric is now printed as `x/y`, where `x` is the unsafe code used by the
   build and `y` is the total unsafe usage found in the crate.
 - Removed the `--compact` output format to avoid some code complexity. A new
   and better compact mode can be added later if requested.

### 0.2.0
 - (alexmaco) Table based default output format. Old format still available by
   `--compact`.

### 0.1.x
 - Initial experimental versions.
 - Mostly README.md updates.


