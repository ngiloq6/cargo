Canonical definitions of `home_dir`, `cargo_home`, and `rustup_home`.

This provides the definition of `home_dir` used by Cargo and rustup,
as well functions to find the correct value of `CARGO_HOME` and
`RUSTUP_HOME`.

The definition of `home_dir` provided by the standard library is
incorrect because it considers the `HOME` environment variable on
Windows. This causes surprising situations where a Rust program will
behave differently depending on whether it is run under a Unix
emulation environment like Cygwin or MinGW. Neither Cargo nor rustup
use the standard libraries definition - they use the definition here.

This crate further provides two functions, `cargo_home` and
`rustup_home`, which are the canonical way to determine the location
that Cargo and rustup store their data.

## License

MIT/Apache-2.0
