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

## Step 17: flow results

The flow algorithms keep no state to query afterwards; every run returns what it produced.

| Before | After |
|---|---|
| `MaximumFlow<E> { value, flow_map }` (public fields) | `MaximumFlow<V, E>` with `value()`, `flow(e)`, `flow_map()`, `flow_direction(e) -> Option<&V>`, `cut_capacity()`, `source_partition()`, `sink_partition()`, `cut_edges()` |
| `alg.flow_map()`, `alg.flow_direction(e)` after a run | read them from the `MaximumFlow` that `maximum_flow(s, t)` returned |
| `alg.calculate_min_cut(s, t)?` then `alg.cut_capacity()`, `alg.source_partition()`, `alg.sink_partition()`, `alg.cut_edges()` | `let cut = alg.minimum_cut(s, t)?;` then `cut.capacity()`, `cut.source_partition()`, `cut.sink_partition()`, `cut.cut_edges()` (`MinimumCut<V, E>`) |
| `alg.last_flow_value()`, `current_source()`, `current_sink()` (`-1` before the first run) | removed; use the result of the run |
| `GusfieldEquivalentFlowTree`/`GusfieldGomoryHuCutTree` implemented `MaximumFlowAlgorithm` and returned `Unsupported` for the flow | they implement `MaximumFlowValueAlgorithm` (value only); the Gomory-Hu tree also implements `MinimumSTCutAlgorithm` |
| `gomory.calculate_global_min_cut()?` then `gomory.source_partition()` | `gomory.global_minimum_cut()?` returns the `MinimumCut` |
| `mcf.get_minimum_cost_flow(p)?` then `mcf.dual_solution()` and `mcf.test_optimality(eps)?` | `let s = mcf.solve(p)?;` then `s.flow()`, `s.dual_solution()`, `s.is_optimal(eps)` (`MinimumCostSolution`) |
| `MaximumFlowAlgorithm::maximum_flow_value` | `MaximumFlowValueAlgorithm::maximum_flow_value` (supertrait of `MaximumFlowAlgorithm`; import it to call the method) |

`PadbergRaoOddMinimumCutset::calculate_min_cut(odd, compression)` returning `f64` (`f64::MAX` when
there was no odd cut) plus `source_partition()`, `sink_partition()`, `cut_edges()` is now
`minimum_odd_cut(odd, compression) -> Result<Option<MinimumCut<V, E>>>`; `None` means no odd cut-set
exists. An odd-sized `odd_vertices` is `InvalidParameter { name: "odd_vertices", requirement: Even }`.

Errors are typed: an unknown source or sink is `VertexNotFound`, identical terminals are
`SourceEqualsSink`, and a negative capacity is `InvalidWeight(Negative)`.

## Step 16: walks and mappings

* `GraphWalk` fields are private. Replace `walk.vertices` with `walk.vertex_list()` (a slice;
  `.to_vec()` for an owned list), `walk.edges` with `walk.edge_list()` and `walk.weight` with
  `walk.weight()`.
* `GraphWalk::new(vertices, edges, weight)` is gone. Use `GraphWalk::from_parts(..)?`, which checks
  `vertices.len() == edges.len() + 1` (or both empty), or `from_vertex_list` / `from_edge_list` /
  `try_new` to verify the walk against a graph.
* `GraphMapping` fields are private; use `vertex`, `vertex_inverse`, `edge`, `edge_inverse`,
  `map_vertex` and `map_edge`.
* `WeakChordalityInspector::get_certificate` returns
  `Option<&WeakChordalityCertificate<V>>`, an enum `Hole(Vec<V>) | Antihole(Vec<V>)` of closed vertex
  lists with `vertex_list()` and `is_hole()`, instead of a `GraphWalk` that held no edges for an
  antihole.
* `alg::util::Indexed` is no longer public.

## Step 15: shortest paths

* Removed: `yen_k_shortest_paths(g, s, t, k)` (use `YenKShortestPath::new(g).get_paths(s, t, k)`),
  `EppsteinKShortestPath::paths` (use `get_paths`), and the public `floyd_warshall(g)` and
  `johnson(g)` functions (use `FloydWarshallShortestPaths::new(g)?.all_pairs()` and
  `JohnsonShortestPaths::new(g)?.all_pairs()`, which return the same `AllPairsPaths`).
* `YenKShortestPath`, `YenShortestPathIterator` and `BoundedPrunedYenKShortestPath` require only
  `OutgoingGraph`; `DijkstraClosestFirstIterator` and the many-to-many helpers have narrower bounds.
* `ALTAdmissibleHeuristic` with no landmarks is `InvalidParameter { name: "landmarks", NonEmpty }`;
  too many transit vertices is `InvalidParameter { name: "transit_vertex_count", AtMost(n) }`.

## Step 14: matching, tours, spanning, flow

* `UndirectedGraph` is required (was a run-time `InvalidGraph(Undirected)`) by: the dense and
  sparse Edmonds, Hopcroft-Karp, greedy, Kolmogorov (weighted and perfect) and maximum-weight
  bipartite matchers; the Gusfield equivalent-flow and Gomory-Hu trees; `PadbergRaoOddMinimumCutset`;
  every metric TSP heuristic (`tour::base` and its users); the vertex-cover algorithms that check
  it; the k-center algorithms (`CentersLocationAlgorithm::centers` now takes
  `UndirectedGraph + OutgoingGraph`); and `StoerWagnerMinimumCut`. Passing a directed graph no
  longer compiles. `MinimumCostFlow` still checks the direction at run time and returns
  `InvalidGraph(Directed)`.
* `DinicMFImpl`, `EdmondsKarpMFImpl` and `BoykovKolmogorovMFImpl` accept any `OutgoingGraph`.
* Typed errors: `InvalidParameter` for `k` (centers, spanners), `passes`, `max_vertices`,
  `max_states`, `terminals` and the Steiner terminal limit; `InvalidGraph(Connected)` for
  disconnected Steiner and capacitated-spanning-tree input; `Infeasible` when no capacitated
  spanning tree exists; `InvalidWeight(NotFinite | NaN)` for bipartite-matching weights;
  `InvalidGraph(AtLeastTwoVertices)` for Gusfield.

## Step 13: cycles, isomorphism, trees

* `AHUUnrootedTreeIsomorphismInspector`, `CliqueMinimalSeparatorDecomposition` and `PatonCycleBase`
  require `G: UndirectedGraph` (was a run-time `InvalidGraph(Undirected)`); `NaiveLcaFinder` no
  longer re-checks the direction its `DirectedGraph` bound guarantees.
* Typed errors replace `InvalidArgument` strings: a root or vertex that is not in the graph is
  `VertexNotFound`; an empty tree or forest is `InvalidGraph(NonEmpty)`; an empty root set is
  `InvalidParameter { name: "roots", requirement: NonEmpty }`; a Chinese-postman input that is not
  connected is `InvalidGraph(Connected)`; `StoerWagnerMinimumCut` on fewer than two vertices is
  `InvalidGraph(AtLeastTwoVertices)`; an empty bipartite layout side is
  `InvalidParameter { name: "partition", .. }`.
* Narrower bounds (callers only gain): `CycleDetector`, `TarjanLcaFinder`, VF2 and the Eulerian
  cycle accept graphs with fewer capabilities.

## Step 12: coloring, scoring, clustering, drawing

* `UndirectedModularityMeasurer::new`, `KSpanningTreeClustering::new`,
  `NaiveGreedyModularityAlgorithm::new`, `LabelPropagationClustering::new` and `Coreness::new`
  require `G: UndirectedGraph` instead of returning `InvalidGraph(Undirected)` for a directed
  graph. A directed graph no longer compiles.
* `VertexScoringAlgorithm::vertex_score` returns `VertexNotFound` and
  `EdgeBetweennessCentrality::edge_score` returns `EdgeNotFound` for an unknown element (was
  `InvalidArgument("Cannot return score of unknown ...")`).
* New `GraphExt::weight_of(e)`: the weight of an edge the graph itself yielded. It replaces
  `edge_weight(e).unwrap_or(1.0)` throughout the algorithms, so a malformed graph panics with the
  integrity-contract message instead of getting an invented weight of `1.0`.

## Step 11: predicates and utilities

No signature changed. `graph::props` predicates return `false` (or skip the offending edge) for a
graph whose edges have missing endpoints instead of panicking; `Indexed::new` and the algorithms
built on it still require the `Graph` integrity contract and say so in their panic message.

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
| `InvalidGraph(GraphProperty)` | the graph lacks a required property: `Directed`, `Undirected`, `NotMixed`, `Simple`, `SimpleDirected`, `NoParallelEdges`, `NoSelfLoops`, `AllowsSelfLoops`, `AllowsParallelEdges`, `NonEmpty`, `AtLeastTwoVertices`, `Empty`, `Complete`, `CompleteBipartite`, `Bipartite`, `Connected`, `Weighted`, `Acyclic`, `Eulerian`, `Planar`, `NonPlanar`, `Tree` | "Graph must be ..." checks, `props::require_*` |
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
