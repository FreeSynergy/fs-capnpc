# PLAN - capnp-compiler (pure Rust Cap'n Proto schema compiler)

Goal: parse `.capnp` and emit a `CodeGeneratorRequest` identical to the official C++ compiler (capnp 1.4.0),
so `capnpc::codegen::CodeGenerationCommand` produces the same Rust code. Oracle: `capnpc-embedded` (dev-dep only).
Reference sources (MIT, read for understanding; own Rust implementation): `/srv/FreeSynergy/.research/capnproto/c++/src/capnp/compiler/`.
Versions: `capnp` 0.27.3, `capnpc` 0.27.0 (newest on crates.io, 2026-11); `capnpc-embedded` 0.2.0+1.4.0 does not link `capnp`, it returns bytes only.

## Components and C++ counterparts

| Rust module | C++ counterpart | Responsibility |
|---|---|---|
| `error` | error-reporter.c++ | `Diagnostic`, error list with byte spans, line/col rendering |
| `lexer` | lexer.c++, lexer.capnp | UTF-8 text -> token tree (idents, literals, operators, `()` / `[]` lists) and statement tree (`;` / `{}` blocks), doc comments |
| `ast` | grammar.capnp | `Expression`, `Declaration` (all kinds), params, annotations |
| `parser` | parser.c++ | statement/token tree -> AST per declaration grammar |
| `typeid` (+`md5`) | type-id.c++ | child / group / method-param ids (MD5 based) |
| `loader` | module-loader.c++ | file lookup (src prefix, import paths, relative/absolute imports, embedded `/capnp/*.capnp` standard files), module cache |
| `compiler` | compiler.c++ | node tree (stub/expanded/bootstrap/final states), name resolution, aliases (`using`), traversal order, request assembly |
| `generics` | generics.c++ | `BrandScope`, `BrandedDecl`, brand computation/inheritance |
| `translator` | node-translator.c++ | per-declaration schema node translation: struct layout (data/pointer sections, holes, unions, groups), enums, interfaces (param/result structs), consts, annotations, default value encoding |
| `layout` | node-translator.c++ (StructLayout) | pure layout allocator, unit-testable |
| `value` | node-translator.c++ (ValueTranslator) | constant/default expression evaluation to `schema::Value` |
| `request` | capnp.c++ | `CodeGeneratorRequest` serialisation with `capnp::schema_capnp` builders |
| `command` | capnpc `CompilerCommand` | builder API (`file`, `src_prefix`, `import_path`, `output_path`, `run`) + in-process codegen |
| `main` | capnp.c++ CLI | `capnp-compiler compile -o <dir> files...` |

## Data structures
- Lexer output: `Token { kind, start, end }` with `Paren(Vec<Vec<Token>>)`/`Bracket`; `Statement { tokens, body: Line | Block(Vec<Statement>), doc, start, end }`.
- AST mirrors grammar.capnp (owned Rust enums); every node carries `Span`.
- Compiler arena: `Vec<Node>` indexed by `NodeIdx`, `HashMap<u64, NodeIdx>` by id; builtin nodes for scalar types / List / AnyPointer ...
- Schema output: own plain Rust structs (`schema::Node` ...) built by the translator, serialised at the end via `capnp::schema_capnp` builders. This keeps the compiler free of capnp message juggling and makes comparison with the oracle simple (compare `Debug` output of readers).

## Test strategy
1. Unit tests per module (lexer tokens, parser shapes, md5/type ids against known values from type-id-test.c++, layout allocator).
2. Oracle tests (`tests/oracle_*.rs`): compile the same file with `capnpc-embedded` and with the Rust compiler, decode both `CodeGeneratorRequest`s, compare node by node (sorted by id and, separately, in output order), requested files, imports, sourceInfo (doc comments); finally generated Rust text from `capnpc::codegen`.
3. Corpus: tests/schemas/*.capnp (hand written), official `test*.capnp`, fs-documentation contracts, fs-hub schema.

## Milestones
- M1 lexer; M2 parser/AST; M3 ids + resolution + imports; M4 translation incl. struct layout; M5 constants/defaults/annotations/generics; M6 request emission + CLI + full oracle; M7 docs / metadata.

## State
- (initial) Cargo skeleton and oracle dump experiment only. Nothing implemented yet.
