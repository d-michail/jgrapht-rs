# Rust API hardening roadmap

Status: proposed on 2026-10-27. This roadmap must be completed before the public API is declared
stable. It expands `ISSUES.md` I-6 and I-8 into concrete work packages.

## Progress

| Execution step | Status |
|---|---|
| 1. R0 inventory, audit script, fixtures | done (`scripts/api_audit.py`, `docs/API_INVENTORY.md`, `tests/common/fixtures.rs`) |
| 2. Capability traits | done (`graph::capability`; `OutgoingGraph` is a separate trait from `Graph`) |
| 3. `BaseGraph` and map graphs | done |
| 4. Sparse directed type-state | done (`NoIncoming`/`LazyIncoming`/`FullIncoming`) |
| 5. Sparse undirected graphs and views | done |
| 6. Final `Graph` surface | done: fallible incidence methods removed, callers moved to capability bounds (many still wider than needed, see R4); notes in `MIGRATION.md` |
| 7. Normalize `GraphExt`/`DirectedExt`, remove panic helpers | done within step 6: every `try_*` variant and panic wrapper is gone, the remaining helpers are infallible with `where Self: <capability>` clauses |
| 8. First typed `GraphError` variants | done: `InvalidParameter`, `InvalidWeight`, `InvalidGraph`, `SourceEqualsSink`, `Infeasible`, `Unbounded`; about 100 `InvalidArgument(String)` sites converted (the rest, about 150 mostly in generators, followed on 2026-10-28, so the library returns none), the rest are converted as their family is migrated (R8 final pass) |
| 9. Traversals and listener iterators | done: start vertices and radii validated at construction, iteration-time errors are `Result` items, `ClosestFirstIterator` no longer panics on a negative weight, topological cycles are reported; `Strategy`/`Advance` gained an `Output` type |
| 10. Fallible path iterators | done: Yen yields `Result` items, closest-first distance is an `Option`; Eppstein and the Dijkstra closest-first iterator already validate at construction |
| 11. `graph::props`, `alg::util`, connectivity, partitioning | done: the family was already at its narrowest bounds (most of it runs on the `Indexed` snapshot and needs only `Graph`); `is_forest`, `is_dag` and `is_k33_subdivision` no longer panic on a graph that breaks the integrity contract; `Indexed::new` documents that it relies on the contract instead of substituting weight `1.0`; tests run each bound class on outgoing-only, lazy and full sparse graphs |
| 12. Coloring, scoring, clustering, drawing | done: bounds verified minimal by trial narrowing (nothing wider than needed); undirected-only algorithms (`UndirectedModularityMeasurer`, k-spanning-tree, naive greedy modularity, label propagation, `Coreness`) take `UndirectedGraph`; score lookups report `VertexNotFound`/`EdgeNotFound`; 86 silent `edge_weight(e).unwrap_or(1.0)` sites use the documented `GraphExt::weight_of`; `incoming_lists` is infallible |
| 13. Cycles, decomposition, isomorphism, planarity, trees | done: 12 more bounds narrowed by trial (cycle detector, VF2, LCA and Eulerian impls now need only `Outgoing`/`Graph`); `AHUUnrootedTreeIsomorphismInspector`, `CliqueMinimalSeparatorDecomposition` and `PatonCycleBase` take `UndirectedGraph`, `NaiveLcaFinder` drops a check its `DirectedGraph` bound already made; missing-vertex, empty-tree/forest, empty-roots and disconnected-input errors are typed. Planarity internals and `Indexed`-based algorithms keep the uniform `IncidenceGraph` they need |
| 14. Matching, tours, spanning, transforms, flow | done: Dinic, Edmonds-Karp and Boykov-Kolmogorov need only `OutgoingGraph` (the shared `flow_algorithm_api!` macro takes the bound; push-relabel keeps `IncidenceGraph`); tour and cover algorithms drop `Incidence` where they can; the Edmonds, Hopcroft-Karp, greedy, Blossom, bipartite, Gusfield, Padberg-Rao, tour, cover, centers and Stoer-Wagner entry points take `UndirectedGraph`; parameter, weight, connectivity and infeasibility errors in centers, steiner, capacitated, spanner, tours and bipartite matching are typed. The stateful flow and centers getters stay for R5 |
| 15. Shortest paths, compat helpers | done: Yen needs only `OutgoingGraph`, the many-to-many and closest-first iterators and several helpers narrowed; the functional `yen_k_shortest_paths` and `EppsteinKShortestPath::paths` are removed and `floyd_warshall`/`johnson` are crate-private behind the `all_pairs()` accessor of their classes; landmark and transit-vertex errors are typed. The Java-name type aliases (`DijkstraShortestPath`, `BellmanFordShortestPath`, `BFSShortestPath`, `AStarShortestPath`) stay: they are thin aliases of one type with identical behavior. `path_weight`'s infinity sentinel moves with step 19 |
| 16. `Indexed` and `GraphWalk` invariants | done: `GraphWalk` fields are private (read with `vertex_list()`, `edge_list()`, `weight()`), `new` is crate-private `new_unchecked` and the safe `from_parts` returns `InvalidGraphWalk` for lists that break `vertices.len() == edges.len() + 1`; `Indexed` is crate-private with the integrity contract documented; `GraphMapping` fields are private; the weak-chordality certificate is `WeakChordalityCertificate::{Hole, Antihole}` over vertex lists because an antihole is not a walk. Other result structs (`Coloring`, `SpanningTree`, `VertexCover`, flows) keep public fields: their redundancy (`count`, `weight`) is not enforced yet and moves to the R5 result-object pass |
| 17. Flow result objects, narrower flow traits | done: `MaximumFlow<V, E>` owns the value, per-edge flow and direction, both cut partitions and the cut edges; `MinimumCut<V, E>` is the cut alone; `MaximumFlowValueAlgorithm` (value only) is split from `MaximumFlowAlgorithm` so equivalent-flow trees no longer implement methods they cannot support; `max_flow_value == -1` and the `current_source`/`last_flow_value` getters are gone; Gomory-Hu cuts and min-cost-flow duals are returned by the call that computes them (`minimum_cut`, `global_minimum_cut`, `solve`) |
| 18. Planarity, centers and the remaining state machines | done: `BoyerMyrvoldPlanarityInspector::planarity()` returns `PlanarityResult::{Planar(Embedding), NonPlanar(subdivision)}` instead of two mutually failing getters; `CentersLocationAlgorithm::centers` returns a `CentersSolution` holding centers, assignment and covering radius; `lca_set` moved out of `LowestCommonAncestorAlgorithm` into `AllLowestCommonAncestors`, implemented only by the DAG finder; Berge and weak-chordality certificates are `BergeCertificate`/`WeakChordalityCertificate` enums over vertex lists, replacing edge-less `GraphWalk`s. The two `Unsupported` left are the `AsUndirected` edge-insertion refusals and the `AsWeighted` overlay restriction |
| 19. Public sentinels | done: unreachable is `None`, never infinity, in `ShortestPathAlgorithm::path_weight` (`Result<Option<f64>>`), `SingleSourcePaths`/`ListSingleSourcePaths`/`AllPairsPaths`/`ManyToManyPaths::weight` (`Option<f64>`), `GraphMeasurer::diameter`/`radius` (`Option<f64>`, `None` when disconnected) and `MinimumCycleMeanAlgorithm::get_cycle_mean` (`Option<f64>`); the error-hiding free functions `metrics::{diameter, radius, eccentricities, center, periphery}` are removed. Infinity remains only inside internal distance arrays |
| 20. I/O propagation and shape validation | done: every exporter takes edge endpoints through one helper that returns `IoError::Export` for a missing endpoint, an endpoint that is not a vertex, or a missing weight (the Matrix/JSON/CSV unwraps are gone, and a foreign endpoint no longer gets a fresh dangling id); TSPLIB `compute_2d_geographical_distance` returns `Result<i32>`; each importer's vertex factory has a fallible `try_set_vertex_factory` whose error surfaces as `IoError::Callback` with the original error as its source; `GraphImporter` documents that an error may leave earlier elements in the graph. Tests run all ten exporters on graphs that break the integrity contract |
| 21. Typed errors and Rust-only docs | done: `NegativeCycleDetected`/`BellmanFordError` derive `thiserror` (the former keeps its cycle behind an accessor); `GraphError::NegativeCycle` stays payload-free in the shared `ShortestPathAlgorithm` interface and says so; Java exception, `null` and `Double#` wording is gone from the Rust docs; `InvalidArgument(String)` is documented as the fallback for preconditions with no matchable category (about 95 sites left, none of them matched on by text) |
| 22. Final audit | done: `cargo fmt`, Clippy `-D warnings`, rustdoc `-D warnings`, packaging and tests pass, and CI checks fmt and Clippy; `missing_docs` is denied in both crates; `# Errors`/`# Panics` sections are on the public API; lib.rs doctests show `Option`, `Result<Option<_>>`, fallible iterators and capability bounds; every `unwrap`/`expect`/index site has its own row in `panic_sites.tsv` (unwrap/expect read individually, indexing classified by rule); five panics or hangs found and fixed (`is_eulerian`, JSON unicode escape, weak chordality on multigraphs, push-relabel/BK negative capacity, Bellman-Ford non-finite weights); importer mutation test and an algorithm sweep added; `# Errors` on every public trait method |

## Goal

Keep JGraphT's algorithms and observable algorithmic behavior while exposing a Rust-native API.
Java remains the fidelity oracle for results, ordering and complexity; it is not the API-design
oracle. In particular, Java exceptions, nullable results, unsupported-operation exceptions,
stateful getters and numeric sentinels must not be copied when Rust can represent the same contract
with traits, enums, `Option` or `Result`.

This is a breaking-change roadmap. The crates are unpublished and unstable, so prefer a coherent
API over compatibility aliases that preserve accidental Java shapes.

## API policy

Use the following rule at every public boundary.

| Situation | Rust representation |
|---|---|
| A value is normally absent | `Option<T>` |
| An operation can fail for caller-controlled data or external state | `Result<T, E>` |
| A computation can fail and its value can normally be absent | `Result<Option<T>, E>` |
| An operation is unsupported by a graph representation | A capability trait or type-state marker; use `Result` only when the capability is genuinely dynamic |
| A method is valid only after another method ran | Return a result object from the computation, or use `Option`; do not use sentinel fields |
| The caller violates an explicitly documented indexing contract | A panic may be acceptable, matching slice indexing; also offer `get` when useful |
| An internal invariant established by the implementation is broken | `debug_assert!`, `expect` or an internal panic is acceptable; it must not be reachable from valid public input |

Additional rules:

- Do not add `Result` mechanically. Make invalid operations unrepresentable when practical.
- Do not use `f64::INFINITY`, `-1`, empty collections or error strings to encode a second return
  channel in a public API.
- A public method returning `Result` must not call another caller-reachable `Result` with `unwrap`.
- `Iterator::next` must not silently discard an error. Either make iteration infallible by
  construction or yield `Result<T, E>`.
- `try_` names are reserved for a fallible alternative to an otherwise genuinely infallible
  operation. A panic wrapper around the real fallible operation must not receive the shorter name.
- Errors that downstream users may reasonably branch on need typed variants, not message matching.
- Panics that assert a third-party `Graph` implementation's invariants are allowed only when the
  invariant is part of the documented trait contract. Prefer returning an error at I/O boundaries.

## Target graph capability model

The current `Graph` trait makes complete incidence access runtime-fallible because one sparse graph
configuration omits incoming indexes. That forces unrelated algorithms either to return
`GraphError::Unsupported` or to panic. Replace this with capability-oriented traits.

The exact names may change during the design spike, but the intended separation is:

```rust
trait Graph {
    type Vertex;
    type Edge;

    fn vertices(&self) -> impl Iterator<Item = &Self::Vertex>;
    fn edges(&self) -> impl Iterator<Item = &Self::Edge>;
    fn edge_source(&self, edge: &Self::Edge) -> Option<&Self::Vertex>;
    fn edge_target(&self, edge: &Self::Edge) -> Option<&Self::Vertex>;
    fn edge_weight(&self, edge: &Self::Edge) -> Option<f64>;
    fn edges_between(&self, u: &Self::Vertex, v: &Self::Vertex)
        -> impl Iterator<Item = &Self::Edge>;
}

trait OutgoingGraph: Graph {
    fn out_edges_of(&self, vertex: &Self::Vertex)
        -> impl Iterator<Item = &Self::Edge>;
}

trait IncomingGraph: Graph {
    fn in_edges_of(&self, vertex: &Self::Vertex)
        -> impl Iterator<Item = &Self::Edge>;
}

trait IncidenceGraph: OutgoingGraph + IncomingGraph {
    fn edges_of(&self, vertex: &Self::Vertex)
        -> impl Iterator<Item = &Self::Edge>;
}
```

Degree methods become infallible defaults on the capability that supplies the required incidence.
`DirectedGraph`, `UndirectedGraph` and `GraphMut` remain orthogonal capabilities. An algorithm must
request the narrowest bounds it actually uses.

The sketch is deliberately not final. Work package R1 must decide whether `OutgoingGraph` belongs
in the base `Graph` contract, since every current backend supports outgoing access, and whether
mixed graphs require a small additional capability. Avoid duplicate methods with subtly different
semantics.

### Sparse graph type-state

Replace the runtime `IncomingEdgesSupport` choice with a type parameter or separate concrete types:

```rust
SparseIntDirectedGraph<NoIncoming>
SparseIntDirectedGraph<LazyIncoming>
SparseIntDirectedGraph<FullIncoming>
```

Use `FullIncoming` as the default type argument if that keeps the common constructor concise. The
`NoIncoming` form implements `Graph`/`OutgoingGraph` but not `IncomingGraph` or `IncidenceGraph`.
The lazy and full forms implement all applicable capabilities. Weighted variants follow the same
pattern.

This makes unsupported incidence a compile-time error for concrete sparse types. It also lets
generic algorithms remain infallible with respect to representation capabilities.

If a user truly needs to select the storage mode at runtime, provide an enum wrapper with explicitly
fallible dynamic methods. Do not weaken the primary static API for that use case.

### Missing-element semantics

Decide and document these semantics once in R1; every backend and view must agree:

- `edge_source`, `edge_target` and `edge_weight` return `None` for an edge not in the graph.
- `edges_between` returns an empty iterator if either endpoint is absent.
- Collection-like incidence queries return an empty iterator for an absent vertex.
- Algorithms requiring a particular source, sink, root or terminal return
  `GraphError::VertexNotFound` when it is absent.
- Mutations continue to distinguish failure from "no change" with `Result<bool>` or
  `Result<Option<T>>`.
- Every edge yielded by `edges()` or an incidence iterator must have endpoints in `vertices()` and
  a weight. State this as a `Graph` safety/integrity contract even though the trait is not unsafe.

The empty-query policy is intentional: these methods describe graph relations. Membership-sensitive
operations remain fallible instead of silently treating a missing required vertex as isolated.

## Work packages

### R0. Freeze the behavior matrix and install guardrails

Purpose: distinguish caller-reachable panics from valid internal invariant checks before changing
signatures.

Tasks:

- Inventory every public function and trait method in `jgrapht` and `jgrapht-io` as one of:
  infallible, `Option`, `Result`, iterator, state query or unchecked constructor.
- Record every public numeric sentinel (`INFINITY`, `-1`, `usize::MAX`) and every public method whose
  validity depends on a previous call.
- Classify every `unwrap`, `expect`, indexing operation and explicit panic reachable from a public
  entry point. Do not require removing invariant-only checks inside Blossom V and other arena-heavy
  implementations.
- Add regression graph implementations used only by tests:
  - outgoing-only;
  - full incidence;
  - deliberately inconsistent endpoints;
  - edge-weight failure cases such as NaN and negative values.
- Add `catch_unwind` tests only as temporary audit tools. Final tests must assert typed outcomes or
  demonstrate that the invalid call does not compile.
- Add a CI audit script that reports new `unwrap`/`expect` calls applied directly to fallible graph
  queries. Start as reporting-only; make it a gate after R4.

Deliverable: an API inventory checked into `docs/` or generated by a script, plus regression
fixtures shared by the graph, algorithm and I/O tests.

Acceptance:

- Every known public sentinel and call-order dependency has an owner in R1-R8.
- New direct `edges_of(...).unwrap()`, `degree_of(...).unwrap()` and related patterns are visible in
  CI.

### R1. Introduce graph capability traits and sparse type-state

Purpose: remove `Unsupported` from operations whose support can be expressed statically.

Tasks:

- Finalize and document the `Graph`/outgoing/incoming/incidence trait split.
- Change `BaseGraph`, sparse graphs, map graphs and every view to implement the narrowest accurate
  capability set.
- Make capability methods infallible; retain `Result` only on a dynamic adapter, if one is kept.
- Convert sparse incoming-index configuration to marker types or distinct concrete types.
- Ensure view capabilities follow their base graph. `EdgeReversed` swaps incoming and outgoing;
  `AsUndirected`, subgraphs, masks, unions and weighted views must preserve only capabilities they
  can honor.
- Review `GraphMut`: read-only views should not implement it instead of returning
  `GraphError::Unmodifiable` wherever the type system can express immutability. Keep fallible
  mutation for graph rules such as loops, parallel edges, acyclicity and missing factories.
- Update trait documentation with endpoint, weight, membership and iterator-lifetime invariants.

Migration:

- Remove `IncomingEdgesSupport` from the primary constructors.
- Add temporary type aliases only where they map one-to-one to the new type-state API.
- Do not retain a panic-based compatibility trait solely to keep old generic bounds compiling.

Acceptance:

- A no-incoming sparse graph cannot be passed to an algorithm requiring incoming incidence.
- BFS and Dijkstra compile for an outgoing-only directed graph.
- Bidirectional search, SCC algorithms requiring reverse traversal, and full-degree scoring do not
  compile without the incoming capability.
- Standard graphs and full/lazy sparse graphs continue to pass the existing parity suites.
- Capability selection adds no per-query runtime branch.

### R2. Normalize graph queries and extension methods

Purpose: make the shortest method name the safe, idiomatic operation.

Tasks:

- Remove the current `try_neighbors_of`/`neighbors_of().unwrap()` pairing after R1 makes capability
  support static.
- Give infallible capability-bound methods the direct names: `neighbors_of`, `outgoing_of`,
  `incoming_of`, `successors_of` and `predecessors_of`.
- Remove or explicitly rename any remaining panic convenience method to `_unchecked`; keep one only
  when it has a demonstrated ergonomic use.
- Make owned collection helpers simple adapters over borrowed iterators. Avoid duplicate `try_`
  families.
- Review `Graph::get_edge`, endpoint lookup, degree and neighbor collection for the missing-element
  policy above.
- Rename Java-shaped helpers during this breaking pass where a conventional Rust name is clearer.
- Document which operations preserve parallel-edge repetitions and insertion order.

Acceptance:

- No public extension method hides `Result` with `unwrap`.
- There is one canonical method for each graph query, with owned-list helpers clearly secondary.
- Absent-element behavior is identical across base, sparse, map and view graphs.

### R3. Make traversal and path iteration error-safe

Purpose: guarantee that successful construction is not followed by a hidden capability panic or a
silently discarded algorithm error.

Tasks:

- Add capability bounds to `Bfs`, `Dfs`, post-order DFS, topological order, closest-first and random
  walk iterators.
- Validate start vertices, radii, edge weights and weighted-random-walk distributions in
  constructors when validation is finite and does not change the documented complexity.
- Where an error can genuinely arise only during iteration, use `Iterator<Item = Result<T>>` or a
  fallible streaming method. Do not silently terminate.
- Change `YenShortestPathIterator`, which currently stores an error and returns `None`, to expose
  the error. Prefer `Iterator<Item = Result<GraphWalk<...>>>`; retain `try_next` only if it adds
  value.
- Audit Eppstein and all traversal listeners/callbacks for the same rule.
- Replace `shortest_path_length() -> f64` returning infinity for an unseen vertex with
  `Option<f64>`; add an explicitly named infinity adapter only if an algorithm needs it.
- Decide whether whole-graph and from-start traversal should be separate constructors or types so
  invalid cross-component/radius combinations are not runtime states.

Acceptance:

- No `Iterator::next` implementation swallows a `GraphError` or unwraps a caller-reachable graph
  query.
- All start-vertex errors are reported at construction.
- Iterator consumers can use normal `collect::<Result<Vec<_>>>()` when iteration is fallible.
- Existing traversal order and listener-event parity tests remain unchanged.

### R4. Propagate the capability model through algorithms and predicates

Purpose: eliminate caller-reachable incidence panics and unnecessary `Result`s throughout the
algorithm layer.

Process each family in this order so foundational predicates settle first:

1. `graph::props`, `alg::util`, connectivity and partitioning;
2. traversal-dependent coloring, scoring, clustering and drawing;
3. cycles, decomposition, isomorphism, planarity and tree algorithms;
4. matching, flow, tours, spanning and transforms;
5. shortest paths and remaining compatibility functions.

For each public function or algorithm class:

- Add the narrowest capability trait bound.
- Propagate genuine runtime errors with `?`.
- Remove `Result` when its only possible error was a now-static unsupported capability.
- Keep `Result` for invalid roots/terminals, invalid graph properties that cannot be encoded in the
  type, negative or NaN weights, arithmetic overflow and algorithm-specific failure.
- Use `Option` for legitimate absence such as no path, no clique or no LCA.
- Compare function-style helpers with class-style implementations. Keep one canonical Rust API;
  aliases must be thin and must not have weaker error behavior.
- Replace runtime direction checks with `DirectedGraph` or `UndirectedGraph` bounds where mixed
  graphs are not supported.
- Add an outgoing-only and full-incidence test for every family whose bounds differ.

Special cases:

- `graph::props` predicates must not panic. A predicate that requires full incidence gets the
  corresponding bound rather than `Result<bool>` solely for capability failure.
- `Indexed::new` must either rely on a documented `Graph` integrity invariant or become
  `try_new`. Do not silently substitute weight `1.0` for an enumerated edge whose weight lookup
  fails unless that is explicitly the unweighted-graph contract.
- `GraphWalk::new` should become `new_unchecked` or private if it does not enforce its documented
  cardinality invariant. Safe constructors return `Result` for malformed walks.
- Public result structs should keep their invariants behind private fields where practical.

Acceptance:

- No direct unwrap remains on fallible graph capability methods in production algorithm code.
- Each algorithm either rejects an unsupported graph at compile time or returns a documented typed
  runtime error.
- All Java parity tests still pass; only Rust signatures and error representation may change.

### R5. Replace stateful Java getters with result objects

Purpose: remove APIs whose validity depends on call order.

#### Flow

Introduce a computation result containing everything produced by one run:

```rust
struct MaximumFlowResult<V, E> {
    value: f64,
    flow: HashMap<E, f64>,
    source_partition: IndexSet<V>,
    sink_partition: IndexSet<V>,
    cut_edges: IndexSet<E>,
}
```

Tasks:

- Remove `max_flow_value == -1.0` as an uncomputed sentinel.
- Make `maximum_flow` return the result object, or cache and expose `Option<&...>` without any
  possibility of indexing uninitialized state.
- Make `last_flow_value`, `flow_map`, `cut_capacity`, partitions and cut edges return `Option` or
  references into a completed result.
- Ensure equivalent-flow-tree implementations that cannot provide an edge flow use a distinct
  result type or trait instead of returning `Unsupported` from methods they can never implement.

#### Planarity

Return a single certificate enum:

```rust
enum PlanarityResult<V, E> {
    Planar(Embedding<V, E>),
    NonPlanar(KuratowskiSubdivision<V, E>),
}
```

Avoid `embedding()` returning `InvalidArgument` after a non-planar result and the inverse failure
for `kuratowski_subdivision()`.

#### Centers, scoring and other lazy inspectors

- Return a solution containing centers, assignment and covering radius.
- Replace "call compute first" `Unsupported` errors with `Option` only when caching remains useful.
- Audit every `scores()`, certificate, matching, decomposition and lazy-inspector getter for the
  same state-machine smell.

Acceptance:

- No public numeric or empty-collection sentinel means "not computed."
- No valid public call order can panic.
- Result values can be retained independently of the mutable algorithm object when cloning graph
  values is already part of the API.

### R6. Normalize absence, sentinels and compound outcomes

Purpose: represent normal algorithm outcomes explicitly.

Tasks:

- Change shortest-path weight queries to `Result<Option<f64>>`; reserve infinity for internal
  distance arrays.
- Change closest-first distance queries for unseen vertices to `Option<f64>`.
- Audit every public `INFINITY`, `-1`, `usize::MAX`, NaN and empty-result sentinel.
- Keep `Result<Option<T>>` where the operation can fail independently of a normal absent result.
- Introduce result enums where more than absence must be represented. The existing Hamiltonian path
  `Found`/`ProvenAbsent`/`Aborted` model is the pattern to follow.
- Review `bool` mutation results: retain them for "changed/already present"; use typed errors for
  prohibited loops, cycles, intrusive edges and unavailable factories.

Acceptance:

- Public docs contain no numeric sentinel contract except where the numeric value is genuinely part
  of the mathematical API.
- Unreachable, uncomputed, unsupported and invalid are distinct outcomes.

### R7. Close I/O error leaks

Purpose: ensure importers and exporters honor their existing `Result` contracts.

Tasks:

- Replace the CSV adjacency export `out_edges_of(...).unwrap()` with propagation or a capability
  bound, according to R1.
- Replace JSON and Matrix endpoint unwraps with `IoError::Export` unless the final `Graph` contract
  makes enumerated-edge endpoints an explicit invariant. Even then, prefer a diagnostic error at
  the external I/O boundary.
- Audit every exporter for missing vertex IDs, invalid weights, writer errors and provider failures.
- Audit every importer for partial mutation. Document whether an error may leave vertices or edges
  already inserted; provide a transactional helper only if users need all-or-nothing behavior.
- Make TSPLIB geographical distance accept a validated two-dimensional coordinate type or return
  `Result<i32>` for nodes with too few coordinates.
- Audit public provider callbacks. Fallible user callbacks must return `Result` and propagate their
  original error context.
- Add malformed custom `Graph` implementations to exporter tests, alongside failing readers and
  writers already covered.

Acceptance:

- No production `jgrapht-io` code unwraps a graph capability result or caller-controlled shape.
- Every read, write, callback and representation failure reaches the caller as `IoError`.
- Existing byte-for-byte Java format parity remains unchanged on valid inputs.

### R8. Make errors useful to Rust callers

Purpose: let downstream code handle important failures without parsing messages.

Design the initial variant set immediately after R2, before traversal and algorithm signatures are
migrated. Add variants as each family is converted, then perform the cross-crate consistency pass
described here after R7. This avoids first converting APIs to `InvalidArgument(String)` and then
changing them a second time.

Tasks:

- Keep `GraphError` `#[non_exhaustive]`.
- Replace high-value uses of `InvalidArgument(String)` with typed variants, initially:
  - invalid or missing source/sink/root/terminal;
  - graph direction/type mismatch;
  - negative or NaN edge weight;
  - non-positive tolerance/radius/count;
  - graph not simple/complete/Eulerian/planar as required;
  - computation not performed, if R5 leaves any such state;
  - no feasible solution where absence is not a normal `Option` outcome.
- Remove `Unsupported(String)` where R1 or a narrower trait makes it impossible. Retain a typed
  dynamic-capability error only for erased/runtime graph adapters.
- Use algorithm-specific errors for payloads such as Bellman-Ford's negative cycle. Implement them
  with `thiserror`, preserve sources, and remove Java exception terminology (`null`, `cause`,
  `get_*`) from Rust documentation.
- Review whether `GraphError::NegativeCycle` should remain payload-free in the common shortest-path
  trait or whether the trait needs an associated error. Prefer a simple common error unless callers
  lose actionable information.
- Keep `IoError` as the boundary error for `jgrapht-io`, transparently wrapping graph and standard
  I/O errors.
- Write tests that pattern-match variants, never messages, except where external format compatibility
  explicitly promises text.

Acceptance:

- Downstream callers can distinguish all documented recovery actions by matching variants.
- Java exception messages are not part of the Rust API contract.
- Error sources are preserved across I/O and algorithm boundaries.

### R9. Finish the panic and documentation audit

Purpose: verify the completed surface instead of treating raw `unwrap` counts as the goal.

Tasks:

- Re-run Clippy with `unwrap_used`, `expect_used`, `panic` and `indexing_slicing` as audit warnings.
- For every remaining production occurrence, record one of:
  internal invariant, documented indexing contract, third-party trait invariant, or defect.
- Prefer `debug_assert!` for expensive internal consistency checks that need not run in release
  builds; keep `expect` messages precise when corruption would indicate a library bug.
- Add `# Errors` and `# Panics` sections to every public fallible or intentionally panicking API.
- Enable `#![warn(missing_docs)]` for `jgrapht`, then raise it to `deny` once the backlog is empty.
- Add doctests showing `Option`, `Result<Option<_>>`, fallible iterator collection and capability
  bounds.
- Add `cargo fmt --all -- --check` to CI and run the complete workspace Clippy target.

Acceptance:

- Valid public input cannot trigger an undocumented panic.
- Every intentional public panic has a `# Panics` section.
- Both crates deny missing public documentation.
- `cargo fmt`, Clippy, docs, packaging and the full test suite are green.

## Current-to-target API register

This register names the known changes from the audit. R0 may add rows, but no row should disappear
without either an implementation or a written decision that the current contract is intentional.

| Current API or behavior | Target | Owner |
|---|---|---|
| `Graph::{edges_of,in_edges_of,out_edges_of,degree_of,...} -> Result<_>` for a storage capability | Split into infallible capability traits | R1 |
| Runtime `IncomingEdgesSupport` on one sparse type | `NoIncoming`/`LazyIncoming`/`FullIncoming` type-state | R1 |
| Read-only views implement mutation and return `Unmodifiable` | Do not implement `GraphMut` when immutability is static | R1 |
| `try_neighbors_of() -> Result<_>` plus `neighbors_of().unwrap()` | One capability-bound, infallible `neighbors_of` | R2 |
| Equivalent `try_`/panic pairs for incoming, outgoing, predecessors, successors and owned lists | Remove panic pairs; keep one canonical method | R2 |
| `Bfs`, `Dfs` and cross-component traversal use `Graph` then unwrap adjacency | Require outgoing/incidence capability as appropriate | R3 |
| Topological traversal discovers missing incoming support through an error or panic | Require incoming capability; retain `Option` only for cyclicity where appropriate | R3 |
| Weighted/random traversal can discover bad input inside `next()` | Validate at construction or yield `Result` items | R3 |
| `YenShortestPathIterator::next` stores an error and returns `None` | Yield `Result<GraphWalk>`; never confuse failure with exhaustion | R3 |
| `ClosestFirstIterator::shortest_path_length` returns infinity for unseen vertices | `Option<f64>` | R3/R6 |
| `graph::props` and functional connectivity helpers unwrap incidence | Capability bounds and infallible predicates | R4 |
| Degree/scoring/coloring/clustering helpers unwrap incidence | Capability bounds; `Result` only for data-dependent failures | R4 |
| `vertex_degree_comparator` can panic while sorting | Require degree capability or construct a validated comparator | R4 |
| `Indexed::new` expects endpoints and substitutes `1.0` for missing weights | Document/enforce graph integrity; use `try_new` if failure remains possible | R4 |
| `GraphWalk::new` accepts inconsistent public fields without validation | Private fields plus safe constructors; `new_unchecked` only if needed internally | R4 |
| Public result structs expose fields that can violate invariants | Prefer private fields and accessors during each family migration | R4 |
| Flow uses `max_flow_value == -1.0` before calculation | `Option` or an owned completed-flow result | R5/R6 |
| `flow_map()` returns `Result` but can panic before calculation | Return completed result data or `Option`/typed error without panic | R5 |
| Cut capacity/partition/edge getters depend on a previous calculation | Fields/accessors on `MaximumFlowResult` | R5 |
| Equivalent-flow trees implement methods they can never support | Separate narrower result/algorithm traits | R5 |
| Planarity has mutually failing `embedding` and `kuratowski_subdivision` getters | `PlanarityResult::{Planar, NonPlanar}` | R5 |
| Center assignment/radius return `Unsupported` before `centers()` | Return a `CentersSolution` from computation | R5 |
| Other lazy score/certificate getters depend on call order | Return result objects or explicit `Option` caches | R5 |
| `ShortestPathAlgorithm::path_weight -> Result<f64>` uses infinity for no path | `Result<Option<f64>>` | R6 |
| Other public infinity, `-1`, max-value or empty-collection sentinels | `Option`, `Result<Option<_>>` or a result enum | R6 |
| CSV adjacency exporter unwraps outgoing incidence | Capability bound or propagated graph error | R7 |
| JSON and Matrix exporters unwrap enumerated-edge endpoints | `IoError::Export` diagnostics at the boundary | R7 |
| TSPLIB geographical distance indexes arbitrary coordinate vectors | Validated 2D type or `Result<i32>` | R7 |
| Importer partial-mutation behavior is implicit | Document it; add opt-in transaction only if justified | R7 |
| `GraphError::{InvalidArgument,Unsupported}(String)` covers matchable cases | Typed non-exhaustive variants | R8 |
| Bellman-Ford documentation and entry points preserve Java exception terminology | Rust error terminology and typed payload/source | R8 |
| Common shortest-path trait erases a negative-cycle payload | Make and document an explicit common-vs-rich-error decision | R8 |
| Public panic behavior is undocumented | `# Panics`, `# Errors`, tests and lint audit | R9 |

## Execution sequence

Use this as the implementation order. Each numbered item should normally be one commit; split a
large algorithm-family item by module, but do not combine unrelated stages.

1. Add the R0 API inventory generator/document and shared adversarial graph fixtures without
   changing public signatures.
2. Add the capability traits and their documentation alongside the current methods.
3. Implement capabilities for `BaseGraph` and map-backed graphs; migrate their core tests.
4. Convert directed sparse graphs to incoming-index type-state and add compile-time capability
   tests.
5. Implement capability propagation for sparse undirected graphs and every view/union wrapper.
6. Switch the `Graph` trait to the final surface and remove the old fallible capability methods.
   This is the main breaking commit and must leave core graph tests green.
7. Normalize `GraphExt`/`DirectedExt`, remove panic compatibility helpers and update direct callers.
8. Define the first typed `GraphError` variants needed by subsequent migrations.
9. Migrate lightweight traversal (`Bfs`, `Dfs`, ordering and random walk), then faithful traversal
   and listener-based iterators.
10. Migrate fallible path iterators, starting with Yen's silent-error behavior, followed by
    Eppstein and closest-first sentinels.
11. Migrate `graph::props`, algorithm utilities, connectivity and partitioning.
12. Migrate coloring, scoring, clustering and drawing.
13. Migrate cycles, decomposition, isomorphism, planarity internals and tree algorithms.
14. Migrate matching, tours, spanning, transforms and flow incidence use.
15. Migrate shortest-path algorithms and remove obsolete functional compatibility helpers.
16. Tighten `Indexed` and `GraphWalk` invariants after all algorithm call sites are known.
17. Replace flow state with completed result objects and narrower flow traits.
18. Replace planarity, centers and remaining lazy state machines with result enums/objects.
19. Remove public numeric sentinels and complete the `Option`/`Result<Option<_>>` migration.
20. Fix I/O propagation and shape validation, format by format; document importer mutation behavior.
21. Complete the typed-error migration and remove Java exception language from Rust docs.
22. Run the final panic classification, documentation, Clippy, formatting, test, doc and package
    gates; close I-6 and I-8 only after all acceptance criteria pass.

At steps 6, 10, 17, 18 and 19, update a temporary `MIGRATION.md` or the eventual `CHANGELOG.md` with
old and new call examples. Do not postpone all migration notes until the release commit.

## Dependency order and milestones

```text
R0 inventory
    |
    v
R1 capability traits and sparse type-state
    |
    +--> R2 graph queries
    |       |
    |       +--> R3 traversal
    |       |
    |       +--> R4 algorithms
    |
    +------------------> R7 I/O

R2 --> R8 typed-error design --> R3 + R4

R3 + R4 --> R5 stateful results --> R6 sentinels/outcomes

R1 through R7 --> R8 final error pass --> R9 final audit
```

Milestones:

1. **M1 — capability-safe core:** R0-R2 complete. Graph storage and views compile against the new
   traits; sparse unsupported operations are type errors.
2. **M2 — panic-safe computation:** R3-R4 complete. Traversals, predicates and algorithms neither
   unwrap capability failures nor discard iterator errors.
3. **M3 — Rust-native outcomes:** R5-R6 complete. Stateful getters and numeric sentinels are gone.
4. **M4 — boundary-safe crates:** R7-R8 complete. I/O propagates failures and errors are matchable.
5. **M5 — release gate:** R9 complete. Documentation, lint, test and package gates pass.

Do not begin the crates.io release step until M5 is complete. The remaining shortest-path fidelity
work may proceed before R1, but avoid adding new public compatibility APIs that will immediately be
removed by this roadmap.

## Test matrix

Every affected family should be exercised against the smallest applicable matrix:

| Graph/input | Expected coverage |
|---|---|
| `BaseGraph` directed and undirected | Full behavior and mutation errors |
| Map-backed graph | Same public outcomes as `BaseGraph` |
| Sparse outgoing-only directed graph | Outgoing-only algorithms compile and run |
| Sparse lazy/full incoming graph | Full-incidence algorithms compile and run |
| Views and unions | Capability propagation and endpoint integrity |
| Missing source/sink/root | Typed membership error |
| Disconnected graph | Normal `Option` absence where applicable |
| Negative and NaN weights | Typed error for algorithms that prohibit them |
| Empty and singleton graphs | Documented mathematical result, never incidental panic |
| Failing reader/writer/callback | `IoError` with preserved source |
| Malformed custom `Graph` | Documented trait-invariant response at I/O boundaries |

Add compile-fail doctests or a small `trybuild` suite for capability rejection only if ordinary
generic compile tests cannot express the intended guarantee without a new development dependency.

## Per-work-package completion checklist

Each work package is complete only when:

- its public signatures follow the policy table;
- migration notes describe every breaking rename or return-type change;
- Rust-native regression tests and existing Java parity tests pass;
- `STATUS.md` changes only if algorithm fidelity or coverage changed;
- `ISSUES.md` is updated in the same commit;
- `cargo test --workspace` passes;
- `cargo clippy --workspace --all-targets` introduces no new warnings;
- `cargo doc --workspace --no-deps` passes with the configured rustdoc warnings;
- `cargo package -p jgrapht --locked` and the `jgrapht-io` package-list audit pass.

## Explicit non-goals

- Removing all internal `unwrap` and indexing from proven arena or dense-array algorithms.
- Preserving Java class names, getter names or exception messages when they harm Rust ergonomics.
- Changing Java-compatible algorithm results, tie-breaking or asymptotic complexity.
- Making every graph validation compile-time. Weight signs, completeness, simplicity and many
  algorithm preconditions are data-dependent and correctly remain runtime errors.
- Adding transactional graph mutation to every importer unless user demand justifies its cost.
