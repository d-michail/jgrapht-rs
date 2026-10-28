# Known issues

Review of 2026-10-27 on `master` (e8d4f07). Build and `cargo test --workspace` are green (2337 passed, 0 failed, 4 ignored). Items are ordered by how much they block a release. Algorithmic deviations from Java live in `FIDELITY_ROADMAP.md`; this file covers everything else.

## Blocking CI

**I-1. (resolved)** Fixed: the counting `PartialEq` carries a documented `allow`; CI runs Clippy with `-D warnings`.

**I-2. (resolved)** Fixed: all warnings are gone and `cargo fmt --check` plus `clippy -D warnings` run in CI.

## Remaining fidelity gaps

**I-3. (resolved)** Shortest-path package (1i) is complete: all Java test classes are ported and contraction hierarchies, CH many-to-many and transit node routing follow the Java techniques (see `FIDELITY_ROADMAP.md` section G). What remains is `HashSet` iteration order in tie situations, documented there.

**I-4. (resolved)** `JavaRandom` was removed; seeded results are not expected to match Java, and the affected tests check properties (see `tests/STATUS.md`). `PathGrowingWeightedMatching` still starts from the first inserted vertex rather than Java's `HashSet` order.

**I-5. Ignored tests (4).** Two Brown backtracking colouring cases (`insertions2_3Test`, `jeanTest`, very slow upstream too), the `ApBetweenness` overflow test (about a minute) and the `MaximumWeightBipartiteMatching` n = 1000 case. A weekly scheduled CI job runs them (`cargo test --release -- --ignored`).

## API and robustness

**I-6. (resolved, with limits)** Caller-reachable panics found by the audit are fixed (five defects, see [`PANIC_AUDIT.md`](PANIC_AUDIT.md)); incidence is a compile-time capability, fallible iterators yield `Result`, and importers are mutation-tested and about 60 algorithms are swept on random pseudographs. 751 `unwrap`/`expect` sites remain, each recorded in [`panic_sites.tsv`](panic_sites.tsv) with a class; indexing sites are classified by rule, not individually read.

**I-7. (partly resolved)** Fixed: both crates have `#![deny(missing_docs)]`, public fallible functions have `# Errors` sections, and `lib.rs` has doctests for `Option`, `Result<Option<_>>`, fallible iterators and capability bounds. About 440 short name-derived summaries remain ("Creates a new instance.", "Returns the len."); the graph traits, walks, path results, LCA finders, builders and algorithm interfaces were rewritten, the rest are improved as the APIs are touched.

**I-8. (resolved)** Done per [`RUST_API_ROADMAP.md`](RUST_API_ROADMAP.md): capability traits, typed errors, result objects, `Option` for unreachable. About 57 `InvalidArgument(String)` sites remain as the documented fallback (38 were converted to typed variants).

**I-9. (resolved)** `Graph::vertices()`/`edges()` yield `VertexRef`/`EdgeRef`; the sparse graphs yield `usize` by value and keep no identity array (`docs/REF_OR_VALUE_PLAN.md`, `tests/sparse_memory.rs`).

**I-10. (resolved)** Documented in `docs/USER_GUIDE.md` ("Views and snapshots"): views borrow their base, `NeighborCache` is a lazily filled cache that stays current only when registered as a listener, and result objects are values.

## Process

**I-11. (resolved)** `cargo fmt --check` is in CI, and a `bench-gate` job compares the benchmarks of a pull request with its base on the same runner (`scripts/bench_gate.py`, 30% threshold).

**I-12. (resolved)** The hand-off text in `FIDELITY_ROADMAP.md` sections F/G was trimmed.
