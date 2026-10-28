# jgrapht-rs user guide

A short tour of the API for people who know JGraphT or who are new to the crate. The reference
documentation is `cargo doc`; this guide explains the design decisions that are not obvious from
signatures. Differences from Java are collected in the last section.

## Building and querying a graph

```rust
use jgrapht::*;
use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};

fn main() -> Result<()> {
    let mut g = UGraph::<&str>::simple().weighted();
    g.add_vertices(["a", "b", "c", "d"]);
    g.add_weighted_edge(&"a", &"b", 1.0)?;
    g.add_weighted_edge(&"b", &"c", 2.0)?;
    g.add_weighted_edge(&"c", &"d", 1.0)?;

    let path = Dijkstra::new(&g).path(&"a", &"d")?;   // Option<GraphWalk<..>>
    println!("{:?}", path.map(|p| p.weight()));
    Ok(())
}
```

`UGraph<V>` and `DiGraph<V>` are the default implementations. Vertices are any `Clone + Eq + Hash`
value; edges are `DefaultEdge` objects unless you choose another edge type.

## Absent values and errors

| Situation | Representation |
|---|---|
| a value that is normally absent (unreachable vertex, no cycle) | `Option` |
| a call that can fail because of the arguments or the graph | `Result<_, GraphError>` |
| both | `Result<Option<_>>` |
| an operation the representation cannot support | a capability trait that the type does not implement, so the call does not compile |

`GraphError` has typed variants (`VertexNotFound`, `InvalidWeight`, `InvalidGraph(property)`, ...).
Match on variants, never on message text. Unreachable is `None`, never `f64::INFINITY`.

## Capability bounds

Incidence queries live on three traits so that a graph that cannot answer one does not pretend to:

| Trait | Methods |
|---|---|
| `OutgoingGraph` | `out_edges_of`, `out_degree_of` |
| `IncomingGraph` | `in_edges_of`, `in_degree_of` |
| `IncidenceGraph` (both) | `edges_of`, `degree_of` |

A sparse directed graph built as `SparseIntDirectedGraph::<NoIncoming>` implements only
`OutgoingGraph`; passing it to an algorithm that needs incoming edges is a compile error. When you
write your own generic code, ask for the narrowest bound you use.

## Vertices and edges in generic code

`Graph::vertices()` and `Graph::edges()` yield `Graph::VertexRef` / `Graph::EdgeRef`. For a graph
that stores its vertices (`BaseGraph`) that is `&Vertex`; for a graph that computes them (the sparse
integer graphs) it is the vertex itself. Both implement `Borrow<Vertex>`, so generic code looks
like this:

```rust
use std::borrow::Borrow;
use jgrapht::*;

fn total_degree<G: IncidenceGraph>(g: &G) -> usize {
    g.vertices().map(|v| g.degree_of(v.borrow())).sum()
}
```

When you only need owned values, use `g.vertices_cloned()` and `g.edges_cloned()`. Code that works
with a concrete type such as `UGraph<String>` sees plain references and needs no `borrow()`.

The same rule explains why traversal iterators (`Bfs`, `Dfs`, `DepthFirstIterator`, ...) and
`topological_sort` yield owned vertices: they cannot hand out references into a graph that has none.

## Views and snapshots (differences from JGraphT's live views)

JGraphT's views and helper caches register themselves as `GraphListener`s on the base graph and
update when it changes. Rust's borrowing rules make that unnecessary in most places and impossible
in others, so the design differs on purpose:

* **Views borrow their base.** `AsSubgraph`, `MaskSubgraph`, `AsUnweighted`, `AsUndirected`,
  `EdgeReversed`, `GraphUnion` and `Unmodifiable` hold the base graph by value or by reference.
  While a view holds a shared borrow nobody can mutate the base, so a view can never go stale.
  `MaskSubgraph` (a predicate over the base) always reflects the base it borrows. `AsSubgraph`
  copies its vertex and edge *sets* when it is created, so it does not track elements added to or
  removed from the base later; build a new one. Views over `&mut G` write through where the Java
  view does (`add_vertex`, `set_edge_weight`, ...), and the unmodifiable ones simply do not
  implement `GraphMut`.
* **`NeighborCache` is a lazily filled cache, not a live view.** `neighbors_of`,
  `successors_of` and `predecessors_of` compute a vertex's neighbour set on the first call and
  return a borrowed `&Neighbors<V>` from the cache. The sets are correct as of that first call. To
  keep the cache current as the graph changes, create it with `NeighborCache::shared()` and register
  it as a listener on a `Listenable` graph (its `edge_added`, `edge_removed` and `vertex_removed`
  methods are the callbacks); otherwise treat it as a snapshot and rebuild it after mutating. The
  cache stores clones of the neighbouring vertices (with a count per parallel edge) because it holds
  no reference to the graph: that is what lets it be updated while the graph is mutated. Cloning is
  free for integer vertices; use `Rc`-wrapped vertices if cloning a vertex is expensive.
* **Cached algorithm state is not invalidated.** Result objects (`Coloring`, `SpanningTree`,
  `MaximumFlow`, ...) are values computed from the graph at the time of the call; they do not
  update when the graph changes, and they do not borrow it.

Rule of thumb: if you need to see a mutation, ask for the data again.

## Randomness

Randomised algorithms take any `rand::RngCore` (`StdRng::seed_from_u64(7)` for reproducible runs).
Results for a given seed are valid but are **not** the same as JGraphT's for the same seed: the
random number generators differ.

## Differences from JGraphT at a glance

| JGraphT | jgrapht-rs |
|---|---|
| exceptions, `null` | `Result`, `Option` |
| `Double.POSITIVE_INFINITY` for "no path" | `None` |
| `getX()` valid only after `compute()` | the computation returns the result object |
| `UnsupportedOperationException` on some graphs | the trait is not implemented |
| live views and listener-driven caches | borrowing views; explicit snapshots (see above) |
| `Random` with a fixed seed reproduces output | any `RngCore`; same seed gives a valid but different result |
| `HashSet` iteration order in ties | insertion order |
