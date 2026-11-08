# FreeSynergy Capnpc — build the contracts with Rust alone

**This is a building block. It is why anyone with a Rust toolchain can build FreeSynergy, with no C++ compiler.**

Capnpc is a Cap'n Proto schema compiler written in pure Rust. FreeSynergy describes its interfaces in Cap'n Proto. The official compiler is written in C++, which means every contributor would need a C++ toolchain. This one reads the same schema files and produces the same output, so the normal Rust code generator can take over from there.

What is different here: we do not ask you to trust it. Its output is compared with the official compiler (capnp 1.4.0): every node, every source-info record and the generated Rust text, for the official test schemas, the FreeSynergy schemas and a set of hand-written feature schemas.

## What it can do today

Latest tag: `v0.1.1`.

- A command line, `capnp-compiler compile`, with output folder, import paths and source prefix.
- A library with a builder, so schemas can come from memory and errors can be collected elsewhere.
- Output identical in content to the official compiler for the schemas above.
- Used by `fs-interfaces` to regenerate its contracts.

## What is not there yet

- For large schemas the layout of the message in memory differs from the official one. The content is identical and readers do not care.
- The release is a draft on GitHub, not yet public (according to the release repo's status note).
- Only compilation to the request for the Rust generator, not other languages.

## Part of FreeSynergy

Capnpc keeps the contract repo buildable on any machine with Rust. FreeSynergy is a family of programs that are free for everyone, forever, and that keep your data yours. The documentation, the manifesto and the overall plan live in [fs-documentation](https://github.com/FreeSynergy/fs-documentation); the reading version is at <https://freesynergy.github.io/fs-documentation/>. Start with the [manifesto](https://freesynergy.github.io/fs-documentation/manifesto/) if you want to know why we do this.


## For developers

- One crate, `capnp-compiler`. The pipeline is a chain of traits (`SourceLoader`, `Frontend`, `ErrorReporter`, `Emitter`); `CompileCommand` composes them.
- Library use: `CompileCommand::new().file("schema/hello.capnp").run_bytes()?` gives the request for `capnpc::codegen`.
- Build and test: `cargo build` and `cargo test`. The tests use the official compiler (as WASM, `capnpc-embedded`) as an oracle.
- Design and the pattern map: [DESIGN.md](DESIGN.md). The status of each oracle schema is in `PLAN.md`. Credits in `NOTICE`.
- Licence: MIT OR Apache-2.0.

## Reference

A Cap'n Proto schema compiler written in pure Rust. It reads `.capnp` schema files and produces the
`CodeGeneratorRequest` that `capnpc::codegen` (capnproto-rust) turns into Rust code - **without the
C++ `capnp` tool and without any C/C++ toolchain**.

The output is verified against the official compiler (capnp 1.4.0, run through `capnpc-embedded`
in the tests): every node, every source-info record and the generated Rust text are compared for the
official `test.capnp` family, the FreeSynergy schemas and a set of hand written feature schemas.

Part of the [FreeSynergy](https://freesynergy.github.io/fs-documentation/) project.

### Command line

```text
capnp-compiler compile -o <dir> [-I <import-dir>]... [--src-prefix <dir>]... files...
```

| Option | Meaning |
|---|---|
| `-o, --output <dir>` | write `<name>_capnp.rs` for every requested file into `<dir>` |
| `-I, --import-path <dir>` | search `<dir>` for absolute imports (`import "/foo.capnp"`) |
| `--src-prefix <dir>` | strip `<dir>` from file names (as `capnp compile --src-prefix`) |
| `--no-standard-import` | do not fall back to the embedded `/capnp/*.capnp` files |
| `--request <file>` | additionally write the raw serialized `CodeGeneratorRequest` |

### Library

```rust
use capnp_compiler::CompileCommand;

let mut cmd = CompileCommand::new();
cmd.src_prefix("schema").file("schema/hello.capnp");
let request: Vec<u8> = cmd.run_bytes()?;           // CodeGeneratorRequest, ready for capnpc
capnpc::codegen::CodeGenerationCommand::new()
    .output_directory("src/generated")
    .run(&request[..])?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Every stage is a trait (`SourceLoader`, `Frontend`, `ErrorReporter`, `Emitter`), so sources can come
from memory (`MemoryLoader`) and diagnostics can be collected elsewhere. See `DESIGN.md` for the
architecture and the pattern map, `PLAN.md` for the status of each oracle schema, and `examples/`.

### Known difference

The serialized request is content-identical to the official one, but for large schemas the *segment
layout* of the message differs (the C++ builder only fills its most recent segment). Readers such as
`capnpc` do not care.
