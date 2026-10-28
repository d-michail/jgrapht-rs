# Known issues

Review of 2026-10-28 on `master`. Targeted test runs are green (`alg` 1593, `generate` 163, `graph` 248, `serde_roundtrip` 11); the 2337-test full run dates from 2026-10-27. Items are ordered by how much they block a release. Algorithmic deviations from Java live in `FIDELITY_ROADMAP.md`; this file covers everything else.

## Blocking CI

**I-1. (resolved)** Fixed: the counting `PartialEq` carries a documented `allow`; CI runs Clippy with `-D warnings`.

**I-2. (resolved)** Fixed: all warnings are gone and `cargo fmt --check` plus `clippy -D warnings` run in CI.

## Remaining fidelity gaps

**I-3. (resolved)** Shortest-path package (1i) is complete: all Java test classes are ported and contraction hierarchies, CH many-to-many and transit node routing follow the Java techniques (see `FIDELITY_ROADMAP.md` section G).

**I-4. (resolved)** `JavaRandom` was removed; seeded results are not expected to match Java, and the affected tests check properties (see `tests/STATUS.md`).

## API and robustness

**I-6. (resolved, with limits)** Caller-reachable panics found by the audit are fixed (five defects, see [`PANIC_AUDIT.md`](PANIC_AUDIT.md)); incidence is a compile-time capability, fallible iterators yield `Result`, and importers are mutation-tested and about 60 algorithms are swept on random pseudographs. 633 `unwrap`/`expect` sites remain, each recorded in [`panic_sites.tsv`](panic_sites.tsv) with a class; indexing sites are classified by rule, not individually read.

**I-7. (resolved)** Fixed: both crates have `#![deny(missing_docs)]`, public fallible functions have `# Errors` sections, and `lib.rs` has doctests for `Option`, `Result<Option<_>>`, fallible iterators and capability bounds. The summaries that only restated the item name ("Creates a new instance.", "Returns the len.", "Sets the node num.") were rewritten from the code, about 300 of them; a heuristic scan finds no weak ones left outside a few accurate setter/getter docs in `jgrapht-io`.

**I-8. (resolved)** Done per [`RUST_API_ROADMAP.md`](RUST_API_ROADMAP.md): capability traits, typed errors, result objects, `Option` for unreachable. The library no longer returns `InvalidArgument(String)`: the last ~150 call sites (mostly generators) now use `InvalidParameter`, `InvalidGraph` and the variants `MissingParameter`, `MissingVertexData`, `TooManyVertices`, `InvalidState`, `DivisionByZero`, `NonTerminatingExpansion` and `Internal`. The variant itself stays for downstream code.

**I-9. (resolved)** `Graph::vertices()`/`edges()` yield `VertexRef`/`EdgeRef`; the sparse graphs yield `usize` by value and keep no identity array (`docs/REF_OR_VALUE_PLAN.md`, `tests/sparse_memory.rs`).

**I-10. (resolved)** Documented in `docs/USER_GUIDE.md` ("Views and snapshots"): views borrow their base, `NeighborCache` is a lazily filled cache that stays current only when registered as a listener, and result objects are values.

## Process

**I-11. (resolved)** `cargo fmt --check` is in CI, and benchmarks run only on demand: `scripts/bench.sh [-b <ref>] [bench...]` locally, or the manual "Benchmarks" workflow (`workflow_dispatch`); with a base ref both revisions are run on the same machine and compared by `scripts/bench_gate.py` (30% threshold). Nothing runs them automatically.

**I-12. (resolved)** The hand-off text in `FIDELITY_ROADMAP.md` sections F/G was trimmed.

**I-13. (resolved)** Java serialization has a Rust counterpart: the optional `serde` feature covers `BaseGraph`, the sparse graphs, `GraphType`, `DefaultEdge`, `GraphWalk`, `AsSubgraph`, the plain views and the result types, validating input through the normal mutation rules (`docs/USER_GUIDE.md`, `tests/serde_roundtrip.rs`). Function-pointer factories are not serialized; use `with_default_edge_factory()` after reading.
