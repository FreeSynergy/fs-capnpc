# capnp-compiler

A Cap'n Proto schema compiler written in pure Rust. It reads `.capnp` schema files and produces the
`CodeGeneratorRequest` that `capnpc::codegen` (capnproto-rust) turns into Rust code - **without the
C++ `capnp` tool and without any C/C++ toolchain**.

The output is verified against the official compiler (capnp 1.4.0, run through `capnpc-embedded`
in the tests): every node, every source-info record and the generated Rust text are compared for the
official `test.capnp` family, the FreeSynergy schemas and a set of hand written feature schemas.

Part of the [FreeSynergy](https://freesynergy.github.io/fs-documentation/) project.

## Command line

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

## Library

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

## Known difference

The serialized request is content-identical to the official one, but for large schemas the *segment
layout* of the message differs (the C++ builder only fills its most recent segment). Readers such as
`capnpc` do not care.

## Licence

MIT OR Apache-2.0. See [NOTICE](NOTICE) for credits.
