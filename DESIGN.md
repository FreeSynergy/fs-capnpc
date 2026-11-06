# DESIGN - capnp-compiler

Pure-Rust Cap'n Proto schema compiler producing the same `CodeGeneratorRequest` as the official
C++ compiler (capnp 1.4.0), so `capnpc::codegen` yields identical Rust. Rules: ADR 0043 (traits first,
named patterns, files <= 200 lines / hard 300), ADR 0040 (why a Rust compiler).

## Pipeline (each stage behind an interface)

`SourceLoader` (bytes) -> `LexStage` -> `ParseStage` (together `Frontend`) -> resolve/translate
(`compiler`, `translator`, `layout`, `generics`, `value`) -> `Emitter` (`CodeGeneratorRequest`).
`ErrorReporter` receives diagnostics from every stage. `CompileCommand` composes everything.

## Pattern map

| Module | Pattern | Why |
|---|---|---|
| `command` | Facade + Builder | one fluent entry point hiding the pipeline; collaborators injectable |
| `source` (`SourceLoader`, `DiskLoader`, `MemoryLoader`) | Strategy | disk vs in-memory sources; tests and embedding |
| `source::ModuleLoader` | Facade | lookup rules (relative / absolute / std), caching, parsing |
| `source::std_files` | Registry | embedded `/capnp/*.capnp` table |
| `pipeline` (`LexStage`, `ParseStage`, `Frontend`) | Strategy + Facade | swap/compose front-end stages |
| `error` (`ErrorReporter`, `CollectingReporter`) | Strategy | diagnostics sink is injectable |
| `lexer` | Iterator-style scanner | token tree and statements |
| `parser` | Template method over a cursor | declaration grammar by kind |
| `ast` | Value types | grammar.capnp mirror |
| `compiler` | State (stub/expanded/bootstrap/final) + Template method (driver) | lazy node compilation like compiler.c++ |
| `generics` | Value types + Strategy-like brand evaluation | BrandScope / BrandedDecl |
| `translator` | Template method | per-kind node translation |
| `layout` | Strategy-free pure allocator | struct layout, unit-testable on its own |
| `value` | Interpreter | constants to `schema::Value` |
| `request` (`Emitter`, `RequestEmitter`) | Strategy + Adapter | serialise own IR with `capnp::schema_capnp` |
| `hashorder` | Registry-like lookup table emulation | official output order (kj hash tables) |

## Decisions
- Own IR (`schema`) instead of building capnp messages during compilation; serialised once at the end.
- `schema_capnp.rs` is vendored generated code (patched to carry source locations), exempt from size rules.
- `capnpc-embedded` is a dev-dependency only: the oracle for tests.

## State
See `PLAN.md` (State section) for the current status and per-schema oracle table.
