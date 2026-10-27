# Migration notes (temporary)

Breaking changes made by the Rust API hardening roadmap (`docs/RUST_API_ROADMAP.md`). This file is
folded into `CHANGELOG.md` before the first release.

## Step 6: incidence queries moved to capability traits

`Graph` no longer has `edges_of`, `in_edges_of`, `out_edges_of`, `degree_of`, `in_degree_of` or
`out_degree_of`. They live on three traits, re-exported from the crate root and `jgrapht::graph`,
and return plain values: an absent vertex gives an empty iterator or `0`.

| Trait | Methods |
|---|---|
| `OutgoingGraph` | `out_edges_of`, `out_degree_of` |
| `IncomingGraph` | `in_edges_of`, `in_degree_of` |
| `IncidenceGraph` (implies both) | `edges_of`, `degree_of` |

```rust
// before
let n = graph.out_edges_of(&v)?.count();
fn run<G: Graph>(g: &G) -> Result<()> { for e in g.edges_of(&v)? { /* .. */ } Ok(()) }

// after
let n = graph.out_edges_of(&v).count();
fn run<G: IncidenceGraph>(g: &G) { for e in g.edges_of(&v) { /* .. */ } }
```

Algorithms and views take the narrowest bound they need; many are currently `IncidenceGraph` and
will be narrowed family by family (roadmap R4).

### Sparse directed graphs

`IncomingEdgesSupport` and `with_incoming_support` are gone; the mode is a type parameter.

```rust
// before
SparseIntDirectedGraph::with_incoming_support(n, &edges, IncomingEdgesSupport::NoIncomingEdges)?
// after
SparseIntDirectedGraph::<NoIncoming>::from_edges(n, &edges)?   // also LazyIncoming, FullIncoming
SparseIntDirectedGraph::new(n, &edges)?                        // FullIncoming, unchanged
```

`SparseIntDirectedGraph<NoIncoming>` implements `OutgoingGraph` only, so BFS, DFS and Dijkstra work
on it and bidirectional search, topological order and degree-based algorithms do not compile.

### Extension methods

`GraphExt`/`DirectedExt` lost every `try_*` variant and panic wrapper. `neighbors_of`,
`outgoing_of`, `incoming_of`, `out_neighbors_of`, `neighbor_list_of`, `neighbor_set_of`,
`successors_of`, `predecessors_of`, `successor_list_of`, `predecessor_list_of`, `ancestors_of`,
`descendants_of`, `vertex_has_successors` and `vertex_has_predecessors` are infallible and require
the capability named in their `where Self: ..` clause.

### Other

* `GraphGenerator::generate_graph` and `generate_default` require `G: GraphMut + IncidenceGraph`.
* `CentersLocationAlgorithm::centers` requires `OutgoingGraph`.
* `Listenable` forwards in/out edge queries to the wrapped graph instead of deriving them, so their
  order now matches the base graph.
* Tests that asserted `GraphError::Unsupported` for missing incoming indexes were replaced by
  `compile_fail` doctests and `tests/alg/capability_bounds_test.rs`.
