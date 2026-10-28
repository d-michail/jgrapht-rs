# Known issues

Review of 2026-10-27 on `master` (e8d4f07). Build and `cargo test --workspace` are green (2337 passed, 0 failed, 4 ignored). Items are ordered by how much they block a release. Algorithmic deviations from Java live in `FIDELITY_ROADMAP.md`; this file covers everything else.

## Blocking CI

**I-1. (resolved)** Fixed: the counting `PartialEq` carries a documented `allow`; CI runs Clippy with `-D warnings`.

**I-2. (resolved)** Fixed: all warnings are gone and `cargo fmt --check` plus `clippy -D warnings` run in CI.

## Remaining fidelity gaps

**I-3. (resolved)** Shortest-path package (1i) is complete: all Java test classes are ported and contraction hierarchies, CH many-to-many and transit node routing follow the Java techniques (see `FIDELITY_ROADMAP.md` section G). What remains is `HashSet` iteration order in tie situations, documented there.

**I-4. (accepted)** Exact Java reproducibility for seeded `BarabasiAlbert`, `DirectedScaleFree`, `RandomRegular` and `PathGrowingWeightedMatching` is not a goal; results are valid but may differ from Java for the same seed. Checked against the Java source: `BarabasiAlbert` already matches for default integer vertices.

**I-5. Ignored tests (4).** Two Brown backtracking colouring cases (`insertions2_3Test`, `jeanTest`, very slow upstream too), the `ApBetweenness` overflow test (about a minute) and the `MaximumWeightBipartiteMatching` n = 1000 case. A weekly scheduled CI job runs them (`cargo test --release -- --ignored`).

## API and robustness

**I-6. (resolved, with limits)** Caller-reachable panics found by the audit are fixed (five defects, see [`PANIC_AUDIT.md`](PANIC_AUDIT.md)); incidence is a compile-time capability, fallible iterators yield `Result`, and importers are mutation-tested and about 60 algorithms are swept on random pseudographs. 751 `unwrap`/`expect` sites remain, each recorded in [`panic_sites.tsv`](panic_sites.tsv) with a class; indexing sites are classified by rule, not individually read.

**I-7. (resolved)** Fixed: both crates have `#![deny(missing_docs)]`, public fallible functions have `# Errors` sections, and `lib.rs` has doctests for `Option`, `Result<Option<_>>`, fallible iterators and capability bounds. Many of the 686 added doc lines are short name-derived summaries; improve them as the APIs are touched.

**I-8. (resolved)** Done per [`RUST_API_ROADMAP.md`](RUST_API_ROADMAP.md): capability traits, typed errors, result objects, `Option` for unreachable. About 95 `InvalidArgument(String)` sites remain as the documented fallback.

**I-9. Sparse graphs borrow ids from an identity array** of `max(n, m)` elements because `Graph` returns references (documented in `STATUS.md`). Costs memory proportional to the larger of `n` and `m`; revisit if the `Graph` trait ever returns owned ids.

**I-10. `NeighborCache` returns snapshots, not live views**, and `AsSubgraph`/other views borrow their base, so Java's listener-based base-change tracking is absent by design. Document in the user guide to avoid surprises when porting Java code.

## Process

**I-11. (partly resolved)** `cargo fmt --check` is in CI; `benches/` have no regression gate (the Java numbers comparison is manual). The 4 Java `jgrapht-ext`/guava/unimi/osm bridges remain out of scope.

**I-12. (resolved)** The hand-off text in `FIDELITY_ROADMAP.md` sections F/G was trimmed.
