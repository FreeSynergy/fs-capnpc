//! `capnp-compiler` command line entry point.
//!
//! Pattern: Facade — the binary only forwards to [`capnp_compiler::cli::run`].

fn main() {
    std::process::exit(capnp_compiler::cli::run(std::env::args().skip(1)));
}
