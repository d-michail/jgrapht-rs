# Plan: reference-or-value items in `Graph::vertices()` and `Graph::edges()`

Status: phases 1 to 5 implemented (see `MIGRATION.md`). The sparse graphs yield `usize` by value and
keep no identity array (`tests/sparse_memory.rs` measures it). Implicit graphs (grids, hypercubes)
are possible but not yet shipped; incidence queries on them would still need `&Edge`.

## Problem

`Graph::vertices()` and `Graph::edges()` return `impl Iterator<Item = &Vertex>`. A graph whose
vertices are the integers `0..n` has no stored values to borrow, so the sparse graphs keep an
identity array `[0, 1, ..., max(n, m) - 1]` just to hand out `&usize`.

* `SparseIntDirectedGraph<NoIncoming>`: `source` + `target` + outgoing CSR = 24 bytes per edge, the
  identity array adds 8 (25% of the graph).
* `FullIncoming`: 32 bytes per edge, plus 8 for the array (20%).
* Everything else the trait hands out already borrows from stored data: `edge_source`,
  `edge_target` and the incidence iterators point into `source`, `target` and the CSR rows.
  Only `vertices()` and `edges()` need the array.

A second consequence: no *implicit* graph can implement `Graph` (a grid, a hypercube, a complete
graph, a de Bruijn graph, a game-state graph) because it has nothing to borrow. With by-value items
those graphs need O(1) memory.

## Design

Give `vertices()` and `edges()` an item type the graph chooses, constrained to borrow as the
element type:

```rust
pub trait Graph {
    type Vertex: Clone + Eq + Hash;
    type Edge: Clone + Eq + Hash;
    /// What `vertices()` yields: `&'a Vertex` for graphs that store vertices, `Vertex` itself
    /// for graphs that compute them.
    type VertexRef<'a>: Borrow<Self::Vertex> where Self: 'a;
    type EdgeRef<'a>: Borrow<Self::Edge> where Self: 'a;

    fn vertices(&self) -> impl Iterator<Item = Self::VertexRef<'_>>;
    fn edges(&self) -> impl Iterator<Item = Self::EdgeRef<'_>>;
    // everything else unchanged
}
```

`std` implements `Borrow<T>` for both `T` and `&T`, so `BaseGraph` sets `VertexRef<'a> = &'a V`
(no change in cost) and the sparse graphs set `VertexRef<'a> = usize`. Generic algorithms call
`v.borrow()` to get a `&Vertex` either way.

What does **not** change: `edge_source`, `edge_target`, `edge_weight`, `edges_between`, the
incidence iterators (`out_edges_of`, `in_edges_of`, `edges_of`), `contains_*`, `GraphMut`. They keep
returning references because a graph that can answer them has the data to borrow from. (An implicit
graph that cannot would still need a different incidence design; out of scope here.)

### Why not the alternatives

* **A separate `IntGraph` trait with by-value iterators.** Algorithms are written against `Graph`.
  Either every algorithm is duplicated, or `IntGraph: Graph`, which brings back the reference
  methods and the identity array.
* **A process-wide shared identity table.** Removes the per-graph array but leaks memory by design.
* **Iterate `edges()` in CSR order, borrowing from the matrix.** Changes the iteration order the
  tests rely on, and does nothing for `vertices()`.
* **`Cow<'a, Vertex>` as the item.** Works, but clones strings eagerly in `into_owned()` call
  sites and makes `usize` carry a discriminant. `Borrow` is zero-cost for both cases.

## Ergonomics (so call sites do not turn into `.borrow()` noise)

Add to `GraphExt`, with `where Self: Sized`-free default bodies:

```rust
fn vertices_cloned(&self) -> impl Iterator<Item = Self::Vertex>;   // .map(|v| v.borrow().clone())
fn edges_cloned(&self)    -> impl Iterator<Item = Self::Edge>;
fn vertex_list(&self)     -> Vec<Self::Vertex>;
fn edge_list(&self)       -> Vec<Self::Edge>;
```

Call-site inventory (library, tests, `jgrapht-io`; grep, 2026-10-28):

| Pattern | Sites | Migration |
|---|---|---|
| `.vertices()/.edges()` total | ~930 lib, ~530 tests, ~59 io | |
| `.cloned()` / `.copied()` directly | 181 | `vertices_cloned()` / `edges_cloned()`, mechanical |
| `.collect()` directly | 49 | `vertex_list()` / `edge_list()` where the result is owned |
| `for x in g.vertices()` | 241 | `for x in g.vertices() { let x = x.borrow(); ... }` or `vertices_cloned()` when the loop clones anyway |
| adapter chains (`map`, `filter`, `any`, ...) | 121 | `.borrow()` inside the closure |
| signatures `impl Iterator<Item = &G::Vertex>` | ~100 | take the `VertexRef` item type or return owned |

Type inference: in generic code the `where` clause makes `v.borrow()` unambiguous. In code on a
concrete `BaseGraph<String, _>` it can be ambiguous (`Borrow<String>` vs `Borrow<str>`); annotate
`let v: &String = v.borrow()` or use the `*_cloned()` helpers there.

## Migration (each phase compiles and keeps the tests green)

1. **Helpers first, trait unchanged.** Add `vertices_cloned`, `edges_cloned`, `vertex_list`,
   `edge_list`; rewrite the ~230 `.cloned()/.copied()/.collect()` sites to use them. Pure refactor.
2. **Flip the trait** (`VertexRef`, `EdgeRef`), set `&'a V` in `BaseGraph`, map graphs, views and
   test fixtures, and fix the remaining ~450 library sites guided by the compiler. Views
   (`AsSubgraph`, `AsUnion`, weights, unmodifiable, ...) forward the base graph's item type.
   Benchmarks must show no regression for `BaseGraph`.
3. **Sparse graphs:** return `usize` / by-value items, delete `identity_ids`, `ids` and the
   `max(n, m)` allocation; measure bytes per edge before and after.
4. **Tests and `jgrapht-io`:** compile-driven fixes (~590 sites). Add a test graph that computes
   its vertices (a grid) to prove implicit graphs work with the algorithms.
5. **Docs:** `MIGRATION.md`, user guide section, `ISSUES.md` I-9 closed, `STATUS.md`.

Phases 2 and 3 are the risky ones; do them on their own PR with the benchmarks attached.

## Acceptance criteria

* `cargo test --workspace`, clippy `-D warnings` and rustdoc `-D warnings` green.
* `graph_perf` and `shortestpath_perf` within noise of the baseline for `BaseGraph`.
* Sparse directed graph: 24 bytes per edge (`NoIncoming`) and 32 (`FullIncoming`), measured.
* One implicit graph (grid or hypercube) with O(1) memory passes BFS, Dijkstra and connectivity.
* No algorithm signature mentions `&G::Vertex` iterators where an owned or `VertexRef` item is
  enough.

## Risks and open questions

1. **Breaking change** to every external `Graph` implementation (two new associated types). The
   crates are unpublished, so acceptable, but `MIGRATION.md` must say it.
2. **Compile times and error messages** for GATs inside `impl Iterator` returns. Prototype on
   `graph/mod.rs`, `base.rs` and one algorithm before committing to the full flip.
3. **Should `Vertex` be required to be `Copy` for by-value graphs?** Not needed: `Borrow` plus
   `Clone` is enough, and a non-`Copy` computed vertex (a `Vec<u8>` state) just allocates per item.
4. **Edges by value too?** Needed for implicit graphs and for dropping the array when `m > n`;
   adds the same machinery for `EdgeRef`. Recommended.
5. **Implicit graphs and incidence:** `out_edges_of` still returns `&Edge`. A computed-edge graph
   would need `EdgeRef` there too. Defer until a real use case exists.
