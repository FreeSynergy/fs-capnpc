# Changelog

## 0.1.1 — 2026-11-06

- Package metadata: `rust-version = 1.88`, `authors`, security contact; release packages via fs-release (Linux, Windows, macOS), SPDX SBOM, OpenPGP signatures.

## 0.1.0 (unreleased)

- Pure Rust lexer, parser and compiler for the full Cap'n Proto schema language: structs, unions,
  groups, enums, interfaces (generic methods, streaming), constants, annotations, generics with
  brands, aliases (`using`), imports, `embed`.
- Struct layout allocator and type-id generation identical to capnp 1.4.0.
- `CodeGeneratorRequest` output content-identical to the official compiler, including source info
  (ordering reproduces the libc++ hash table of the reference build).
- Library API: `CompileCommand` builder; traits `SourceLoader`, `Frontend`, `ErrorReporter`, `Emitter`.
- CLI: `capnp-compiler compile -o <dir> files...`.
