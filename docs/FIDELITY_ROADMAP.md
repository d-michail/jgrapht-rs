# Fidelity roadmap: same techniques, algorithms and complexity as Java

Scope: only the *technique*, *algorithm* and *asymptotic cost* of each class. Coding style, naming and
API shape are out of scope. Each item is done when:

1. the Java algorithm or data structure is what the code runs (not a different algorithm with the same result);
2. the Java tests for it are ported (and pass);
3. a scaling guard test shows the expected growth (where cheap to write);
4. the `STATUS.md` row is updated, and any deliberate deviation is written down there.

Items marked **[needs 1g]** or **[needs 1i]** wait for the matching or shortest-path packages.

## A. Known algorithmic deviations in ported classes

| # | Item | Java | Rust today | Work |
|---|---|---|---|---|
| A1 (done) | `ColorRefinementAlgorithm` (`alg/color.rs`) | partition refinement, `O((n+m) log n)` | ported partition refinement, shared with the colour-refinement isomorphism inspector | Java tests were already ported; added a 10,000-vertex path scaling guard |
| A2 (done) | `TreeDynamicConnectivity` (`alg/tree_connectivity.rs`) | Euler-tour tree on `AVLTree` (worst-case `O(log n)`) | ported Euler tours over private arena-backed implicit AVL sequences and intrusive arc rings | 24 internal invariant/behavior tests include a 100,000-node AVL height guard; connectivity public behavior preserved |
| A3 (done) | `WeakChordalityInspector` buckets | `HashSet<Integer>`, expected `O(1)` operations and JDK table iteration order | private Rust bucket set reproducing JDK 21 integer hashing, resizing and iteration order | four JDK-generated golden-order unit tests; all weak-chordality certificates, including Clebsch, remain valid |
| A4 (done) | `ApBetweennessCentrality` | configurable-precision decimal Apfloat | pure-Rust radix-10 `dashu-float` with matching precision, truncation and unlimited terminating-division behavior | added Java-golden finite-precision tests and retained the upstream large overflow test |
| A5 (done) | `FarthestInsertionHeuristicTSP` | keeps its default initial sub-tour across calls | now stores the selected longest-edge sub-tour in the algorithm instance | added a two-graph reuse test distinguishing retained state from a fresh instance |

## B. Classes in Java's root `org.jgrapht.alg` with no faithful port

| # | Java class | Tests | Rust today |
|---|---|---|---|
| B1 | **done** `StoerWagnerMinimumCut` (PriorityQueue version, `O(VE log E)`) | 10 | faithful class port; the former function helper is deleted |
| B2 | **done** `TransitiveClosure` | 6 | function in `transitive.rs` |
| B3 | **done** `TransitiveReduction` (Hsu) | 8 | function in `transitive.rs` |

None of the three is in `STATUS.md` yet. Add rows, port classes and tests. The remaining `org.jgrapht.util` tests (`PrefetchIteratorTest`, `RadixSortTest`, `SupplierUtilTest`, `UnmodifiableUnionSetTest`, `VertexToIntegerMappingTest`) are listed in `STATUS.md` as ported or N/A, to be checked.

## C. Replace functional helpers by the faithful classes

The old function API runs algorithms that differ from Java. Each helper becomes a thin wrapper over the class, or is deleted when nothing uses it.

| # | Helper | Replace by | Callers to update |
|---|---|---|---|
| C1 | **done** `isomorphism/legacy.rs` (backtracking, deleted along with the non-induced variants; the older tests and `generate_tests` use `VF2GraphIsomorphismInspector` and `VF2SubgraphIsomorphismInspector`) | VF2 / AHU / colour-refinement inspectors | |
| C2 | **done** `flow/legacy.rs`: `max_flow`, `edmonds_karp`, `dinic`, `push_relabel`, `minimum_st_cut`, `gomory_hu_tree` (deleted; Goldberg and k-connectivity now use `PushRelabelMFImpl` and `EdmondsKarpMFImpl`, the old `goldberg_densest_subgraph` is gone) | the class implementations in `flow/` | |
| C3 | **done** `flow/legacy.rs` (deleted): `min_cost_flow` (SPFA successive shortest paths) and its owned `MinimumCostFlowProblem` struct; NETGEN now returns `MinimumCostFlowProblemImpl`, and the spanning-flow test uses `CapacityScalingMinimumCostFlow` | `CapacityScalingMinimumCostFlow` | |
| C4 | **done** `flow/legacy.rs`: `stoer_wagner` (deleted, callers use B1) | B1 | |
| C5 | **done** `tour/legacy.rs` (nearest neighbour, insertion, double tree, 2-opt, Held–Karp removed; callers use the tour classes, and the non-Java cheapest-insertion variant was deleted) | the tour classes | |
| C6 | `tour/legacy.rs`: `christofides_tour` **[needs 1g]** | `ChristofidesThreeHalvesApproxMetricTSP` on the Java matching | |
| C7 | **done** `cycle/legacy.rs` (`has_cycle`, `simple_cycles`, `cycle_basis`, `cycle_to_walk`; deleted, callers use `CycleDetector`, `JohnsonSimpleCycles` and the cycle-basis classes) | `CycleDetector`, the simple-cycle classes, cycle-basis classes | |
| C8 | **done** `hamiltonian.rs` (deleted; its one test now uses `BacktrackingHamiltonianPath`, Java has no general exact Hamiltonian cycle search) | `tour` Hamiltonian-path/cycle classes | |
| C9 | **done** `eulerian.rs` (deleted; `props::is_eulerian` and Christofides use `HierholzerEulerianCycle`; the Eulerian-trail helper had no Java counterpart) | `HierholzerEulerianCycle` | |
| C10 | **done** `chordal.rs` (deleted; `ChordalGraphColoring` and `ChordalGraphMaxCliqueFinder` now follow Java and use `ChordalityInspector`, new `ChordalGraphIndependentSetFinder`, `props::is_chordal` delegates to the inspector; the chromatic-number and maximal-cliques helpers had no Java counterpart) | `ChordalityInspector` | |
| C11 | **done** `planarity.rs` (left-right, deleted; `props::is_planar` delegates to `BoyerMyrvoldPlanarityInspector`, the old LR tests now run on the inspector) | `BoyerMyrvoldPlanarityInspector`; `props::is_planar` delegates | |
| C12 | **done** `matching/legacy.rs` (deleted; callers use the matching classes) | Dense/Sparse Edmonds, Kuhn–Munkres, greedy, path-growing; Blossom V remains tracked by F3 | |
| C13 (done) | `paths.rs` (deleted; callers/tests use `AllDirectedPaths`, `BellmanFordShortestPath`, `SuurballeKDisjointShortestPaths` and `BhandariKDisjointShortestPaths`; the non-Java vertex-disjoint helper was removed) | Java classes, with typed Bellman–Ford negative-cycle errors carrying the detected `GraphWalk` | |
| C14 | `shortestpath/*` functions **[needs 1i]** | Java algorithm classes | |
| C15 | `ChinesePostman` **[needs 1g]** | faithful class | |

## D. Audit of earlier modules

Modules written before the class-by-class method were never checked for algorithm and complexity fidelity: clique, cover, lca, clustering, decomposition, scoring, connectivity, color (besides A1), spanning, partition, matching-adjacent helpers. For each: compare with the Java source, record any deviation as a new row above, fix it when the tests are ported (the package ports in the main roadmap already do this).

## E. Guardrails

- A grep-based register of remaining `legacy.rs` helpers and their callers, kept in `STATUS.md` until it is empty.
- Where Java is known to be superlinear (list scans such as `edges_between`, `add_edge` on a star), keep it superlinear. Do not tighten it.

## Order

1. A1, A2, A3, B1–B3 (self-contained, no dependency).
2. C1–C5, C7–C10 (wrappers, deletions, caller updates), then C11.
3. Continue the main roadmap with 1g (then C6, C12, C15) and 1i (then C13, C14).
4. D as each remaining package is ported. A4 and A5 whenever convenient.

## F. Matching package (1g): state and hand-off

Rules that apply to everything below: same techniques and complexity as Java, no attribution lines in commits, one cargo command at a time, stage only your own files (`git add <paths>`), push to `origin master`, never force-push, update `crates/jgrapht/tests/STATUS.md` in the same commit.

**Done (uncommitted work may exist, check `git status`)** — new module `crates/jgrapht/src/alg/matching/`: `base.rs` (`Matching<V,E>`, `MatchingAlgorithm`, `DEFAULT_EPSILON`), `greedy.rs` (`GreedyMaximumCardinalityMatching`, `GreedyWeightedMatching`), `path_growing.rs` (`PathGrowingWeightedMatching` + `DynamicProgrammingPathSolver`), `hopcroft_karp.rs`, `max_weight_bipartite.rs` (exact arithmetic with `dashu_float::DBig` at unlimited precision instead of `BigDecimal`; Fibonacci heap from `rheaps`), `kuhn_munkres.rs`, `dense_edmonds.rs`, `sparse_edmonds.rs`. The old function-style matching module has been deleted and its callers use these classes. `DulmageMendelsohnDecomposition` and bipartite vertex cover use `HopcroftKarpMaximumCardinalityBipartiteMatching`.
Ported tests so far (`tests/alg/alg_matching_tests.rs`, 117 pass, 1 ignored): greedy cardinality (2), `ApproximateWeightedMatchingTest` for greedy-weighted, path-growing and no-heuristics path-growing plus the base path-growing tests, Hopcroft–Karp bipartite (8), `MaximumWeightBipartiteMatchingTest` (7, one `#[ignore]` for n = 1000), Kuhn–Munkres (11), Dense Edmonds (23), and Sparse Edmonds (24).
Known deviation: `PathGrowingWeightedMatching` picks the start vertex as the first of an insertion-ordered set (Java: `HashSet.stream().findAny()`, i.e. hash order). Tests pass; revisit with `JavaIntHashSet` only if a test depends on it.

**F1 (done). Non-Blossom tests** (`tests/alg/alg_matching_tests.rs`, one module per Java test class):
- `KuhnMunkresMinimalWeightBipartitePerfectMatchingTest` (11 tests; vertices of the two partitions are distinct, use e.g. `(u8, u8)` tuples; build the graph with `SimpleWeightedBipartiteGraphMatrixGenerator`; the 8x8, 20x20 and 21x21 cost matrices are in the Java file).
- `DenseEdmondsMaximumCardinalityMatchingTest` (23) and `SparseEdmondsMaximumCardinalityMatchingTest` (24). The ten large edge lists are already generated in `tests/alg/matching_data.rs` (`GRAPH1`..`GRAPH10`, identical for both classes); expected cardinalities are in the Java files (58, 100, 100, 99, 98, 96, 91, 86, 75, 66; graph 2 is also perfect). `isMaximumMatching` / `isOptimalMatching` + odd set cover are the certificates. Random tests use `GnmRandomGraphGenerator`, `BarabasiAlbertGraphGenerator`.
- All 58 tests pass without requiring algorithm changes; the large shared edge lists live in `tests/alg/matching_data.rs`.
- The `STATUS.md` rows are complete.

**F2 (done). Remove the legacy matching helpers.** `alg/cover.rs`, the decomposition and netgen tests, and the older clique/cover, path and matching/color integration tests now use `HopcroftKarpMaximumCardinalityBipartiteMatching`, `DenseEdmondsMaximumCardinalityMatching`, `GreedyWeightedMatching`, `PathGrowingWeightedMatching`, and `MaximumWeightBipartiteMatching` through `MatchingAlgorithm`. The small exact weighted oracle moved into its test. `matching/legacy.rs` and all of its function re-exports were deleted. The current legacy Christofides helper has its own temporary subset-DP implementation, so it did not require retaining matching legacy code. C12 is done.

**F3. Kolmogorov Blossom V** (`org/jgrapht/alg/matching/blossom/v5`, 5917 lines): `BlossomVOptions`, `BlossomVState`, `BlossomVNode`, `BlossomVEdge`, `BlossomVTree`, `BlossomVTreeEdge`, `BlossomVInitializer`, `BlossomVPrimalUpdater`, `BlossomVDualUpdater`, `KolmogorovWeightedPerfectMatching`, `KolmogorovWeightedMatching`, `ObjectiveSense`. Port into `src/alg/matching/blossom/` with arena/index based nodes, edges and trees (no pointers), same data structures and update rules as Java (priority queues of edges per tree, slack/dual bookkeeping, `ObjectiveSense`, options such as `InitializationType`, `UpdateDualsAfter`, `DualUpdateStrategy`). Port the Java tests in this order, running them after each step: `BlossomVStateTest`, `BlossomVTreeEdgeTest`, `BlossomVTreeTest`, `BlossomVNodeTest`, `BlossomVInitializerTest`, `BlossomVDualUpdaterTest`, `BlossomVPrimalUpdaterTest`, `KolmogorovWeightedMatchingTest` (19), `KolmogorovWeightedPerfectMatchingTest` (52); `BlossomVDebugger` is a test helper. Commit per step. Java's tests are sensitive to tie-breaking and iteration order: keep insertion order everywhere Java iterates a list.

- **F3a foundation complete:** `BlossomVOptions`, `ObjectiveSense`, and the arena-backed node,
  edge, tree, and tree-edge structures are implemented with stable indices, intrusive linked lists,
  and addressable pairing heaps. Six Java-derived structural tests pass.
- **F3b algorithm core complete:** the simple, greedy, and fractional initializers; primal grow,
  augment, shrink, and expand operations; both global dual-update strategies; weighted perfect and
  general matching classes; statistics; optimality/error checks; and dual solutions are ported.
  Twenty-one focused tests pass, including all nine Java fractional-initializer cases, exhaustive
  complete and sparse exact-oracle sweeps across all 24 option combinations for both objectives,
  no-solution cases, parallel edges, and self-loops. The remaining F3 work is the full Java public
  fixture corpus and any defects it exposes. Public fixture progress: 16/52 perfect-matching cases
  and 5/19 general-matching cases pass for every option/objective combination.

**F4. Dependents of Blossom V.** `ChristofidesThreeHalvesApproxMetricTSP` (uses `KolmogorovWeightedPerfectMatching` on the odd-degree vertices; port the Java class and its tests, then delete `christofides_tour` in `tour/legacy.rs`, C6) and `ChinesePostman` (class and tests, C15).

## G. Shortest-path package (1i): checkpoint and hand-off

**Checkpoint commit: see the first local commit after `b887d7e`; work is intentionally incomplete.**

Implemented in `crates/jgrapht/src/alg/shortestpath/`: Java-style BFS, Dijkstra and closest-first iterator, Bellman–Ford, A*/ALT, bidirectional Dijkstra/A*, Floyd–Warshall, Johnson, list/tree single-source paths, `TreeMeasurer`, array-backed integer Dijkstra, sequential delta stepping, `DagAllPathsCounter`, default/Dijkstra/CH many-to-many, Martin and label-correcting multi-objective paths, `AllDirectedPaths`, Suurballe/Bhandari, lazy Yen, bounded/pruned Yen, Eppstein sidetrack heaps, sequential contraction hierarchies, and transit-node routing. C13 deleted the old `alg/paths.rs` helpers and their non-Java vertex-disjoint implementation; callers now use the shortest-path classes directly.

Current validation: `cargo test -p jgrapht --test alg shortestpath:: -- --nocapture` passes **111/111** shortest-path tests. These include the complete 20-test `AllDirectedPathsTest`, all 46 inherited and class-specific Suurballe/Bhandari disjoint-path cases, complete list/tree single-source path-result contracts, all four seeded `AllPairsShortestPathsTest` cross-checks, focused Java-derived golden sequences for Yen/Eppstein, bounded-pruned equivalence tests, foundational class contracts, CH/TNR comparisons with Dijkstra, and Java-derived Bellman–Ford negative-cycle payload tests.

Strict-fidelity work still required before marking 1i or C14 done:

- Port the remaining 34 Java shortest-path test files / roughly 213 tests and record each class separately in `STATUS.md`; the checkpoint is a coherent implementation base, not a completed package port.
- Make contraction preprocessing reproduce Java's parallel independent-set/random-tie ordering and implement Java's bucket-based CH many-to-many algorithm. The checkpoint uses deterministic sequential witness searches and materialized CH point queries.
- Complete transit-node routing's Java Voronoi locality filter and exact access-node selection. The checkpoint's conservative locality filter is correct but not technique-faithful.
- Add bounded-pruned Yen's alternate A* spur engine and settled-vertex accounting (`expanded_vertices` is currently zero).
- Finish Java-exact order/tie-breaking and full test coverage for many-to-many, multi-objective, integer Dijkstra, delta stepping, and graph/tree measurers.
- C14 remains open until every retained functional entry point is verified as a thin class wrapper and the full class test suite passes.

C13 is complete: `alg/paths.rs` and its non-Java vertex-disjoint implementation are deleted, all
callers and tests use the Java classes directly, and Bellman–Ford's Java-style entry points return a
typed negative-cycle error containing the detected `GraphWalk`.

The matching section above is independent and was not edited by the shortest-path track.
