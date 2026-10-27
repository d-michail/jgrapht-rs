# Known issues

Review of 2026-10-27 on `master` (e8d4f07). Build and `cargo test --workspace` are green (2337 passed, 0 failed, 4 ignored). Items are ordered by how much they block a release. Algorithmic deviations from Java live in `FIDELITY_ROADMAP.md`; this file covers everything else.

## Blocking CI

**I-1. Clippy error in the `graph` test target.** `crates/jgrapht/tests/graph/map_graph_test.rs:25` derives `Hash` while implementing `PartialEq` by hand (`clippy::derived_hash_with_manual_eq`, deny by default). `cargo clippy --workspace --all-targets`, which CI runs, fails. Fix: implement `Hash` manually consistent with `eq`, or derive both.

**I-2. 128 clippy warnings in the `jgrapht` lib, 40 more in tests/benches.** 116 are `doc list item without indentation` and 12 `overindented`, all from the Javadoc port (`<ul>`/numbered lists turned into Markdown). Others: 21 unnecessary `as u64` literal casts, loops indexing by counter (`needless_range_loop`, Floyd–Warshall and Johnson tests), `type_complexity`, excessive float precision. Not CI-fatal (warnings), but `-D warnings` is impossible until fixed. Mostly machine-fixable with `cargo clippy --fix` plus manual list reindentation.

## Remaining fidelity gaps

**I-3. (resolved)** Shortest-path package (1i) is complete: all Java test classes are ported and contraction hierarchies, CH many-to-many and transit node routing follow the Java techniques (see `FIDELITY_ROADMAP.md` section G). What remains is `HashSet` iteration order in tie situations, documented there.

**I-4. Unverified Java iteration order.** `BarabasiAlbert`, `DirectedScaleFree` and `RandomRegular` use approximations not checked against Java (Phase 0 note), and `PathGrowingWeightedMatching` starts from the first inserted vertex rather than Java's `HashSet` order. Tests pass; results can differ from Java for the same seed. Resolve with golden files from the Java oracle if exact reproducibility matters to users.

**I-5. Ignored tests (4).** Two Brown backtracking colouring cases (`insertions2_3Test`, `jeanTest`, very slow upstream too), the `ApBetweenness` overflow test (about a minute) and the `MaximumWeightBipartiteMatching` n = 1000 case. Run them occasionally (`cargo test -- --ignored`), ideally in a scheduled CI job.

## API and robustness

**I-6. Panics in library code.** About 950 `unwrap`/`expect`/`panic!` sites in `crates/jgrapht/src` (concentrated in `alg/matching/blossom/*`, about 270, then `clustering.rs` 33 and `connectivity.rs` 24) and 19 in `jgrapht-io`. Many are internal invariants of the Blossom V port, but public entry points still hide iterator failures or rely on stateful sentinels. Caller-reachable unwraps of incidence errors are gone (roadmap step 6: incidence is a compile-time capability). The correction plan and acceptance criteria are R0-R9 in [`RUST_API_ROADMAP.md`](RUST_API_ROADMAP.md).

**I-7. Public API is undocumented in `jgrapht`.** 729 public items lack docs (`-W missing_docs`); `jgrapht-io` already has `#![deny(missing_docs)]`. Enabling the lint crate-wide is the Phase 4 target.

**I-8. API consistency not yet reviewed.** Phase 4 items still open: capability bounds are in place but many algorithms still ask for `IncidenceGraph` where a narrower trait would do, `Result`/`Option` policy, fallible iterators, stateful result objects, numeric sentinels, `*_num` versus `*_count` naming, `#[non_exhaustive]` on public enums, and error type uniformity between `jgrapht::GraphError` and per-module errors. Typed `GraphError` variants exist for parameters, weights and graph properties (about 100 sites converted); roughly 150 `InvalidArgument(String)` sites remain. Tracked in detail by [`RUST_API_ROADMAP.md`](RUST_API_ROADMAP.md).

**I-9. Sparse graphs borrow ids from an identity array** of `max(n, m)` elements because `Graph` returns references (documented in `STATUS.md`). Costs memory proportional to the larger of `n` and `m`; revisit if the `Graph` trait ever returns owned ids.

**I-10. `NeighborCache` returns snapshots, not live views**, and `AsSubgraph`/other views borrow their base, so Java's listener-based base-change tracking is absent by design. Document in the user guide to avoid surprises when porting Java code.

## Process

**I-11. `cargo fmt --check` is not in CI**, and `benches/` have no regression gate (the Java numbers comparison is manual). The 4 Java `jgrapht-ext`/guava/unimi/osm bridges remain out of scope.

**I-12. Docs drift.** `FIDELITY_ROADMAP.md` section G still says "checkpoint commit: see the first local commit after `b887d7e`" and sections F/G mention uncommitted work and hand-off rules (no attribution lines in commits) that predate the current workflow. Trim once 1i closes.
