# Guide for coding agents and contributors

jgrapht-rs is a Rust port of [JGraphT](https://jgrapht.org). This is a *port*, not a release
project: the goal is a faithful, idiomatic Rust library. Do not publish crates or create release
tags unless asked. See `README.md` for the overview and `docs/ROADMAP.md`, `docs/ISSUES.md` and
`docs/FIDELITY_ROADMAP.md` for the plans and open items.

## Layout

| Path | Contents |
|---|---|
| `crates/jgrapht/src/graph` | `Graph` traits, capability traits, `BaseGraph`, sparse (`graph::sparse`) and map graphs, builders |
| `crates/jgrapht/src/view` | view wrappers (`AsSubgraph`, `MaskSubgraph`, `GraphUnion`, ...) |
| `crates/jgrapht/src/alg` | algorithms, one module per JGraphT package; results are plain structs |
| `crates/jgrapht/src/{traverse,generate,util}` | traversals, generators, Java-compatible helpers (heaps, priority queue) |
| `crates/jgrapht-io` | importers and exporters, one module per format |
| `crates/*/tests` | ports of the Java tests; `tests/STATUS.md` in each crate maps Java test classes to Rust |
| `crates/jgrapht/benches` | criterion benchmarks (on demand only, see below) |
| `docs/` | plans, issue register, user guide, panic audit |
| `MIGRATION.md`, `CHANGELOG.md` | differences from the Java API; user-visible changes |

## Commands

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
cargo test -p jgrapht --test graph                   # one suite; also alg, generate, traverse, util, delta_stepping
cargo test -p jgrapht --features serde --test serde_roundtrip
cargo test -p jgrapht-io
cargo test --workspace                               # everything, several minutes; the alg suite alone takes about 7
scripts/bench.sh [-b <base-ref>] [bench...]          # benchmarks, only when asked for
```

The workspace requires Rust 1.88 (edition 2024). Prefer the targeted suites while working and run the
whole workspace once before a pull request. CI also runs clippy with `-D warnings`, rustdoc with
`-D warnings` and `cargo fmt --check`.

## Conventions

- **Errors, not panics.** Public functions return `Result<_, GraphError>` for bad arguments or
  graphs and `Option` for values that can be absent (unreachable is `None`, never infinity). Use
  the typed variants (`InvalidParameter`, `InvalidGraph`, `InvalidWeight`, `MissingParameter`,
  `MissingVertexData`, ...); never `InvalidArgument(String)`, and never match on message text.
  Panics are for broken internal invariants only (`docs/PANIC_AUDIT.md`, `docs/panic_sites.tsv`).
- **Capability bounds.** Ask for the narrowest trait you use: `Graph`, `OutgoingGraph`,
  `IncomingGraph`, `IncidenceGraph`. Do not add run-time "unsupported" errors for what a trait
  bound can express.
- **Vertices and edges in generic code.** `Graph::vertices()`/`edges()` yield `VertexRef`/`EdgeRef`,
  which implement `Borrow<Vertex>`; call `.borrow()` or use `vertices_cloned()`/`edges_cloned()`.
  Do not apply mass mechanical `.borrow()` edits to code on concrete graph types; the type
  inference breaks (`Borrow` against `RefCell::borrow`).
- **Randomness.** Randomised algorithms take any `RngCore`. Results need not equal Java's for the
  same seed; tests check properties and algorithm correctness, not Java's random stream.
- **Documentation.** Both crates deny `missing_docs`. Write doc comments that say what the item
  does and its conditions; never restate the name ("Returns the len."). Fallible functions have an
  `# Errors` section.
- **Serde.** The optional `serde` feature covers graphs, sparse graphs, views, walks and result
  types. Input is validated through the normal mutation rules. Function pointers (edge and vertex
  factories) are not serialized.
- **File headers.** New Rust files carry the dual license header used by the existing files.
- **Ported tests.** Keep Rust test modules mirroring the upstream test classes and record the
  mapping in the crate's `tests/STATUS.md`. `N/A` is only for Java-only constructs.

## Benchmarks

Benchmarks never run automatically (not on push, pull request or schedule). Run them locally with
`scripts/bench.sh`, or through the manual **Benchmarks** workflow; see `docs/BENCHMARKS.md` for the files, results and where
reports are written. They are noisy on shared
machines; compare ratios on the same machine, not absolute numbers.

## Working on the repository

- Keep changes focused; update `CHANGELOG.md` for user-visible changes, `MIGRATION.md` for API
  differences from Java, and the status registers or `docs/ISSUES.md` when an item is closed.
- Do not leave generated or scratch files in the repository.
- Commit messages describe the change and its reason. Do not force-push shared branches; merge
  the base branch instead.
- Ask before anything outward-facing or hard to reverse (publishing, tags, deleting branches).
