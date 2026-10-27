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

## Step 10: path iterators

* `YenShortestPathIterator` is `Iterator<Item = Result<GraphWalk<..>>>`. It used to store an error
  and return `None`, which looked like exhaustion; now the error is the next item and the iterator
  then ends. `try_next` is gone (the item is already a `Result`). Use
  `iter.collect::<Result<Vec<_>>>()` or `iter.next().transpose()?`.
* `ClosestFirstIterator::shortest_path_length` returns `Option<f64>` (`None` for a vertex the
  traversal has not reached) instead of `f64::INFINITY`.
* `EppsteinShortestPathIterator` and `DijkstraClosestFirstIterator` validate everything in their
  constructors, so their `next()` cannot fail and keeps its item type.

## Step 9: traversals report errors

* `Bfs::new`, `Dfs::new`, `DfsPostOrder::new`, `ClosestFirst::new`, `ClosestFirst::with_radius` and
  `RandomWalk::new` return `Result` and reject a start vertex that is not in the graph with
  `GraphError::VertexNotFound`. `::all` (whole graph) is unchanged. A negative or NaN radius is
  `InvalidParameter { name: "radius", .. }`. The faithful iterators report a missing start vertex
  as `VertexNotFound` instead of `InvalidArgument`.
* Errors that can only appear while iterating are items. `ClosestFirstIterator` (was an `assert!`
  panic on a negative weight), `ClosestFirst`, `RandomWalk`, `RandomWalkVertexIterator` and
  `TopologicalOrderIterator` yield `Result<..>`; after the first `Err` they end. The error takes the
  place of the vertex that could not be expanded.

  ```rust
  // before
  let order: Vec<_> = TopologicalOrderIterator::new(&g)?.collect(); // a cycle ended it silently
  assert!(!it.inner().is_cyclic());
  // after
  let order = TopologicalOrderIterator::new(&g)?.collect::<Result<Vec<_>>>()?; // Err(InvalidGraph(Acyclic))
  ```
* `BreadthFirstIterator` and `DepthFirstIterator` still yield bare vertices (they cannot fail);
  generic code can call `CrossComponentIterator::try_next`.
* The ordered iterators lost `is_cross_component_traversal` and `set_cross_component_traversal`
  (they always cover every component; the setter could only fail).
* `TransitiveClosure::close_directed_acyclic_graph` now returns `InvalidGraph(Acyclic)` for a
  cyclic graph instead of closing only the vertices ahead of the cycle.

## Step 8: typed errors

`GraphError` gained variants that replace message matching. Match the variant, never the text.

| Variant | Meaning | Replaces `InvalidArgument` for |
|---|---|---|
| `InvalidParameter { name, requirement }` | a numeric or collection parameter violates `ParameterRequirement` (`Positive`, `NonNegative`, `AtLeast(n)`, `AtMost(n)`, `NonEmpty`, `Finite`, `Probability`) | non-positive tolerance, radius, count, precision, ... |
| `InvalidWeight(WeightProblem)` | `Negative`, `NaN`, `NegativeOrNaN`, `NotFinite` | negative or NaN edge weights |
| `InvalidGraph(GraphProperty)` | the graph lacks a required property: `Directed`, `Undirected`, `NotMixed`, `Simple`, `SimpleDirected`, `NoParallelEdges`, `NoSelfLoops`, `AllowsSelfLoops`, `AllowsParallelEdges`, `NonEmpty`, `Empty`, `Complete`, `CompleteBipartite`, `Bipartite`, `Connected`, `Weighted`, `Acyclic`, `Eulerian`, `Planar`, `NonPlanar`, `Tree` | "Graph must be ..." checks, `props::require_*` |
| `SourceEqualsSink` | flow or cut with identical source and sink | |
| `Infeasible`, `Unbounded` | no feasible solution / no finite optimum where that is not a normal `Option` outcome | flow problems, perfect matching |

```rust
// before
Err(GraphError::InvalidArgument(_)) => /* negative weight? bad tolerance? not simple? */
// after
Err(GraphError::InvalidWeight(WeightProblem::NegativeOrNaN)) => ..
Err(GraphError::InvalidGraph(GraphProperty::Simple)) => ..
Err(GraphError::InvalidParameter { name: "tolerance", .. }) => ..
```

Also: `props::GRAPH_MUST_BE_*` message constants are removed; view mutators that were
`Unsupported(String)` now return `Unmodifiable` (`AsSubgraph::set_edge_weight`,
`MaskSubgraph`), `NotWeighted` (`AsUnweighted::set_edge_weight`) or `NoVertexFactory`
(`AsSubgraph::new_vertex`). The new enums are `#[non_exhaustive]`, exported from the crate root.
The remaining `InvalidArgument(String)` and `Unsupported(String)` sites are converted with the
algorithm family they belong to.
