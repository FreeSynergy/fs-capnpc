# capnp-compiler

A Cap'n Proto schema compiler written in pure Rust. It reads `.capnp` schema files and produces the
`CodeGeneratorRequest` that `capnpc::codegen` (capnproto-rust) turns into Rust code — **without the
C++ `capnp` tool and without any C/C++ toolchain**.

Status: in development. Verified byte-for-byte against the official compiler (via `capnpc-embedded`).

Part of the [FreeSynergy](https://freesynergy.github.io/fs-documentation/) project.

## Licence

MIT OR Apache-2.0. See [NOTICE](NOTICE) for credits.
