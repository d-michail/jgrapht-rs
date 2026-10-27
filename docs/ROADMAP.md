# jgrapht-rs roadmap

Status on 2026-10-27. Source of truth for what is ported: `crates/jgrapht/tests/STATUS.md`.

## Where we are

| Area | Java | Rust |
|---|---|---|
| core `graph/`, views, events, `traverse/`, `util/`, `generate/` (incl. netgen) | ported, tests ported (about 400 tests passing, `tests/{alg,graph,generate,traverse,util}`) | done |
| core `alg/` | 312 files, 230 test files, **1509 tests** | class-by-class ports for 1a to 1f (isomorphism, flow, tour, cycle done; 1458 ported tests pass in `tests/{alg,graph,generate,traverse,util}`); 1g (matching) and 1i (shortest paths) still functional. See `FIDELITY_ROADMAP.md` for deviations from Java |
| `jgrapht-io` (`nio`) | 62 files (about 14k loc), 28 test files, about 283 tests | all planned format modules started; 25 Rust integration tests pass, with full Java test-parity expansion still in progress |
| `jgrapht::graph::{sparse,map}` | 22 files (about 2.5k loc), 4 test files | specialized graph backends not started; they belong in the main crate (Phase 3b) |
| other modules (ext, guava, unimi-dsi, osm, demo) | see "Out of scope" | no |

The existing `alg/` is a *functional* port: one function per family (`maximal_cliques`, `simple_cycles`, `nearest_neighbor_tour`, ...). The Java test suite exercises the individual *classes* (every Bron–Kerbosch variant, every simple-cycle algorithm, every TSP heuristic, ...) and often depends on their exact tie-breaking. Porting the tests therefore means growing the algorithm layer class by class, and checking the results against Java.

## Principles (carried over)

1. Tests drive the work: a Java test file is ported together with whatever main code it needs.
2. Fidelity where the tests need it (iteration order, tie-breaking, seeded `Random`). Idiomatic Rust everywhere else (traits, `Result`, borrowed iterators).
3. Java is the oracle. JDK 21 and Maven are installed, so for anything order- or seed-dependent we generate golden output from the real implementation (done for netgen: `tests/generate/fixtures/netgen_seed7.txt`, and the JHeaps jar for heaps).
4. One commit per package or per format, pushed to `origin master`, no `Co-Authored-By`. `STATUS.md` updated in the same commit.
5. N/A is allowed only for Java-only constructs (serialization, reflection, synchronized wrappers, Java collections classes) and is recorded in `STATUS.md`.

## Phase 0: infrastructure for the algorithm tests (about 1 session)

- **Interface traits.** Mirror `alg/interfaces/` (39 files) as traits where more than one implementation exists: `MatchingAlgorithm`, `SpanningTreeAlgorithm`, `VertexColoringAlgorithm`, `CliqueAlgorithm`, `MaximumFlowAlgorithm`, `MinimumCostFlowAlgorithm`, `SimpleCycleAlgorithm`, `HamiltonianCycleAlgorithm`, `TSPAlgorithm`, `VertexScoringAlgorithm`, `ClusteringAlgorithm`, `LCAAlgorithm`, `VertexCoverAlgorithm`, `IsomorphismInspector`, ... Existing functions become thin wrappers, so nothing already written breaks.
- **`alg/util`** (6 test files, 20 tests): `NeighborCache`, `UnorderedPair`, `FixedSizeIntegerQueue`, `VertexDegreeComparator`, `Pair`/`Triple`, `Extension*` (probably N/A).
- **Java iteration order.** Introduce `util::JavaHashMap`/`JavaHashSet` only if a test cannot be satisfied with insertion order; decide per package, using the Java oracle. (BarabasiAlbert, DirectedScaleFree and RandomRegular already use approximations that are unverified against Java.)
- **Test layout.** `tests/alg/<package>.rs`, one module per Java test file, plus a script that lists Java test files not yet in `STATUS.md`.
- **Golden-file helper.** A small `scripts/java-oracle/` (javac + the 1.5.3 jar plus the netgen-style trick of compiling checkout sources against it) to produce expected outputs on demand.

## Phase 1: port `alg/` tests, package by package

Order: cheap and self-contained first, so each step is a commit. Test counts are `@Test` annotations in the Java suite.

| Step | Packages (tests) | Main work needed |
|---|---|---|
| 1a (done) | transform (6), steiner (4), independentset (4), partition (14), similarity (7), centers (12), densesubgraph (19), decomposition (13), util (20) | `ZhangShasha` tree edit distance, `HeavyPathDecomposition`, `DulmageMendelsohn`, Goldberg variants (node weights, per-edge weight), `GonHeuristic`, `LineGraphConverter` |
| 1b (done) | lca (27), linkprediction (17), clique (16), vertexcover (29), color (34), clustering (29) | LCA finders (naive, Tarjan, binary lifting, Euler/RMQ, heavy path); the 9 link-prediction indices as classes; Bron–Kerbosch base/pivot/degeneracy + `CliqueMinimalSeparatorDecomposition` + chordal max clique; exact/greedy/approx vertex cover variants; Brown backtracking and random greedy colouring; Girvan–Newman, greedy modularity, naive greedy modularity |
| 1c (done) | connectivity (37), spanning (32) | `ConnectivityInspector`, Gabow/Kosaraju/Tarjan SCC as separate classes, `BiconnectivityInspector`, `BlockCutpointGraph`, `KConnectivityFlowAlgorithm`, `TreeDynamicConnectivity`; Kruskal/Prim/Boruvka classes, **capacitated MST** (Esau–Williams, Ahuja–Orlin–Sharma), `GreedyMultiplicativeSpanner` |
| 1d (done) | scoring (94), drawing (35), isomorphism (65) | exact betweenness variants (`ApBetweenness`, `EdgeBetweenness`), clustering coefficient, drawing models (`Box2D`, `Point2D`, `LayoutModel2D`, FR quadtree, indexed FR, two-layered bipartite layouts), VF2 (graph + subgraph state classes), AHU tree isomorphism (rooted/unrooted/forest), colour-refinement inspector |
| 1e (done except Christofides and Chinese postman, which wait for the matching of 1g) | tour (119), cycle (216) | every TSP heuristic (greedy, nearest/farthest insertion, nearest neighbour, 2-opt, random, Christofides, Held–Karp, 2-approx), Hamiltonian cycle/path algorithms (Palmer, backtracking, DAG, Held–Karp), all simple-cycle enumerators (Tiernan, Tarjan, Johnson, Szwarcfiter–Lauer, Hawick–James), cycle bases (fundamental BFS variants, Paton), `ChinesePostman`, `HowardMinimumMeanCycle`, `AhujaOrlinSharmaCyclicExchange`, chordality / weak-chordality / Berge inspectors, `HierholzerEulerianCycle` |
| 1f (done) | flow (96) | `BoykovKolmogorov`, Gusfield equivalent-flow / Gomory–Hu classes, `PadbergRaoOddMinimumCutset`, **`CapacityScalingMinimumCostFlow`** (933 loc) with the `MinimumCostFlowProblem` interface (already started in `alg/flow.rs`) |
| 1g | matching (223) | Dense/Sparse Edmonds, Kuhn–Munkres, greedy/path-growing variants, and **Kolmogorov Blossom V** (about 5.7k loc: initializer, primal and dual updaters, trees, options) |
| 1h (done) | planar (40) | **`BoyerMyrvoldPlanarityInspector`** (2.4k loc): planarity test, embedding, Kuratowski subgraph |
| 1i | shortestpath (283) | all path-iterator classes (Dijkstra closest-first, Yen, Eppstein), k-disjoint (Suurballe, Bhandari), bounded/pruned Yen, many-to-many (default, Dijkstra, CH), multi-objective (Martin, label-correcting), ALT and bidirectional A*, `DagAllPathsCounter`, `AllDirectedPaths`, **contraction hierarchies** (precomputation 1.3k loc, bidirectional Dijkstra, many-to-many) and **transit node routing** (1.3k loc), `IntVertexDijkstra`, `TreeMeasurer`, tree/list single-source path impls |

Guidance for the large items (blossom V, Boyer–Myrvold, CH, TNR, capacity scaling, VF2): port the Java line by line into one Rust module each, keep the Java class names as type names, add the Java tests, and only then refactor. These are where the tests are most order-sensitive.

Each step ends with: tests green, `cargo clippy --all-targets` clean, STATUS.md updated, commit, push.

## Phase 2: parallel delta-stepping

After the sequential `DeltaStepping` is verified by the Java `DeltaStepping` tests (inside step 1i):
- `DeltaSteppingShortestPath` with a thread pool (`rayon` or scoped threads): parallel light/heavy relaxation per bucket, atomic distance array, same results as sequential.
- Tests: equality with Dijkstra on random graphs and on the Java test graphs for 1..N threads; a benchmark in `benches/`.

## Phase 3: `jgrapht-io` (the Java `nio` package)

Implementation status is tracked in `crates/jgrapht-io/tests/STATUS.md`. A first vertical tranche is
now present with the shared Rust API, deterministic attributes, structured errors, event-driven
parsing and baseline implementations of every planned format: DIMACS, CSV, graph6/sparse6/digraph6,
JSON, GML, DOT, GraphML, GEXF, Lemon, Matrix and TSPLIB.
Formats are not marked complete until their full upstream test files have been ported.

Workspace crate `crates/jgrapht-io` (lib name `jgrapht_io`), depending on `jgrapht`. All formats stay together as modules in this one companion crate; do not create a crate per format. The Java package has 62 files, 28 test files, about 283 tests and about 14k loc, of which about 30% are the event-driven importer variants.

**Shared layer** (`org.jgrapht.nio`, 13 files): `Attribute`/`AttributeType`/`DefaultAttribute`, `GraphImporter`, `GraphExporter`, `EventDrivenImporter` with `ImportEvent` (consumer closures instead of listeners), `BaseExporter` (vertex-id provider, per-vertex and per-edge attribute providers as closures), `BaseEventDrivenImporter`, `IntegerIdProvider`, `ImportException`/`ExportException` (as an `IoError` enum with `thiserror`, wrapping `std::io::Error`). Importers read from `impl Read`/`&str`, exporters write to `impl Write`.

**Formats** (by size and tests; in this order, value first):

| # | Format | Java tests | Notes |
|---|---|---|---|
| 1 | DIMACS (4 classes) | 21 | line-based; shortest start; exercises the shared layer |
| 2 | CSV (+Visio exporter) | 32 | hand-written replacement for the ANTLR `CSV.g4` |
| 3 | graph6 / sparse6 | 30 | bit packing; also useful for generators |
| 4 | JSON | 23 | hand-written parser replacing `Json.g4` |
| 5 | GML | 33 | hand-written parser replacing `Gml.g4` |
| 6 | DOT | 52 | hand-written parser replacing `DOT.g4` (subgraphs, ports, `strict`, HTML IDs); the biggest grammar |
| 7 | GraphML (6 classes) | 49 | `quick-xml`; typed attributes, key defaults, simple/edge-list importers |
| 8 | GEXF | 11 | `quick-xml`; attribute types |
| 9 | Lemon exporter, Matrix exporter | 5 + 3 | small |
| 10 | TSPLIB importer | 24 | depends on the TSP data in `tour` |

**Decisions to confirm before starting:**
- **Decided: hand-written parsers** for CSV, JSON, GML and DOT, replacing the Java ANTLR grammars (`CSV.g4`, `Json.g4`, `Gml.g4`, `DOT.g4`). No parser-generator dependency, no JVM in the build. `antlr4rust` and the newer `antlr-rust-runtime` were considered and rejected (young third-party runtimes, unknown license for the latter). Fidelity is kept by porting the Java listener logic (value conversion, error messages) and by driving each parser with the Java tests, plus the Java importers as oracle for odd inputs.
- Exporters must match Java output byte for byte (the tests compare strings), including attribute ordering. Use insertion-ordered maps in the shared layer.
- XML comparison in tests: the Java tests use XMLUnit; write a small canonicalising comparer in the test utils.
- Dependencies: `quick-xml`, `thiserror`; no `serde` in the core path (optional feature later).

## Phase 3b: sparse and map-based graph implementations in `jgrapht`

These are graph storage implementations, not a separate integration boundary. Port them into `crates/jgrapht/src/graph/sparse/` and `crates/jgrapht/src/graph/map/`; do not create a `jgrapht-opt` crate. Java: `org.jgrapht.opt.graph.sparse` (CSR-based, immutable, integer vertices and edges) and `org.jgrapht.opt.graph.fastutil` (map graphs on primitive-keyed maps); 4 test files (`SparseIntGraphTest` 814 loc, `IncomingOutgoingEdgesTest`, `FastUtilMapGraphTest`, `FastUtilMapIntVertexGraphTest`).

- **Sparse graphs** (`SparseIntDirectedGraph`, `SparseIntUndirectedGraph` and the weighted variants): `CSRBooleanMatrix`, the specifics classes (no-incoming / incoming, with and without reindexing; incidence-matrix undirected), `CompleteIntegerSet`, `IncomingEdgesSupport`. They implement the `Graph` trait only (immutable: `GraphMut` is deliberately not implemented), take an edge list in the constructor, and use `usize` vertices and edges. This is where the trait design pays off: algorithms written against `Graph` run unchanged on them.
- **Map graphs**: `FastutilMapGraph` and the integer-vertex variant. In Rust these are `BaseGraph` with a different map backend, so the work is a pluggable storage strategy in `BaseGraph` (a `Specifics` parameter like JGraphT's `GraphSpecificsStrategy`) rather than new graph types. Check first whether `hashbrown`/`FxHashMap` and dense `Vec` backends for `usize` vertices are enough; do not add a fastutil clone.
- Tests: `SparseIntGraphTest` and `IncomingOutgoingEdgesTest` (the latter also runs against core graphs), plus the two map-graph tests via the new backends. A benchmark comparing `BaseGraph` and the sparse graphs on Dijkstra and BFS (ties in with the `perf/` benchmarks of Phase 4).
- Prerequisite: Phase 1 steps that touch the `Graph` trait are done, so the trait surface is stable (do the backend work after Phase 1d at the latest).

## Phase 4: hardening and release

- README: Java-to-Rust mapping table, per-package status, how to run the Java-port suite.
- API pass: naming consistency (`*_num` vs `*_count`), error types, `#[non_exhaustive]` on public enums, docs for every public item (`#![warn(missing_docs)]`), doc examples.
- `benches/` (criterion) for the Java `perf/` tests (15), compared with the Java numbers.
- CI (fmt, clippy, tests on stable and MSRV, nightly doc build) and publishing `jgrapht`/`jgrapht-io` to crates.io once the user is ready.
- Re-run the heap differential test against each `rheaps` release.

## Out of scope for now (confirm)

| Module | Size | Plan |
|---|---|---|
| `jgrapht-ext` | 3 files | JGraphX/Swing adapters: skip. |
| `jgrapht-guava`, `jgrapht-unimi-dsi`, `jgrapht-osm`, `jgrapht-demo` | Java-library bridges, OSM import, demos | skip; Rust-native equivalents could come later. |

## Suggested order of work

1. Phase 0 (traits and helper utilities).
2. Phase 1a to 1d (the many small packages; fast progress and broad coverage).
3. Phase 3 formats 1 to 5 (DIMACS, CSV, graph6, JSON, GML) in parallel with Phase 1e/1f, since they are independent and give early user-visible value.
4. Phase 1e to 1i (the big ones: cycles/tours, flows, blossom V, planar, shortest-path speed-up techniques).
5. Phase 3 formats 6 to 10 (DOT, GraphML, GEXF, ...).
6. Phase 3b (sparse and map backends inside `jgrapht`), after the `Graph` trait surface has settled.
7. Phase 2 (parallel delta-stepping), then Phase 4.

Rough size: Phase 1 is about 1500 tests and about 60k lines of Java to port; Phase 3 about 283 tests and 14k lines; Phase 3b 4 test files and 2.5k lines.
