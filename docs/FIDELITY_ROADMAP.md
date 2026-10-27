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
| A2 (done) | `TreeDynamicConnectivity` (`alg/tree_connectivity.rs`) | Euler-tour tree on `AVLTree` (worst-case `O(log n)`) | ported Euler tours over arena-index `AvlTree` and `DoublyLinkedList` | utility tests ported with a 100,000-node AVL height guard; connectivity public behaviour preserved |
| A3 | `WeakChordalityInspector` buckets | `HashSet<Integer>`, `O(1)` ops, ascending order for small ints | `BTreeSet`, extra `log` factor | add a `JavaIntHashSet` that reproduces Java's iteration order with `O(1)` operations |
| A4 | `ApBetweennessCentrality` | Apfloat arbitrary precision | exact `BigRational` | decide: keep as documented deviation (exact is a superset of the Apfloat behaviour but costs differ) or emulate Apfloat precision |
| A5 | `FarthestInsertionHeuristicTSP` | keeps its default initial sub-tour across calls | recomputed | persist state as Java does |

## B. Classes in Java's root `org.jgrapht.alg` with no faithful port

| # | Java class | Tests | Rust today |
|---|---|---|---|
| B1 | **done** `StoerWagnerMinimumCut` (PriorityQueue version, `O(VE log E)`) | 10 | function `stoer_wagner` in `flow/legacy.rs` |
| B2 | **done** `TransitiveClosure` | 6 | function in `transitive.rs` |
| B3 | **done** `TransitiveReduction` (Hsu) | 8 | function in `transitive.rs` |

None of the three is in `STATUS.md` yet. Add rows, port classes and tests. The remaining `org.jgrapht.util` tests (`PrefetchIteratorTest`, `RadixSortTest`, `SupplierUtilTest`, `UnmodifiableUnionSetTest`, `VertexToIntegerMappingTest`) are listed in `STATUS.md` as ported or N/A, to be checked.

## C. Replace functional helpers by the faithful classes

The old function API runs algorithms that differ from Java. Each helper becomes a thin wrapper over the class, or is deleted when nothing uses it.

| # | Helper | Replace by | Callers to update |
|---|---|---|---|
| C1 | `isomorphism/legacy.rs` (backtracking) | VF2 / AHU / colour-refinement inspectors; delete the non-induced variants (no Java counterpart) | `generate_tests`, anything using `count_isomorphisms` |
| C2 | **done** `flow/legacy.rs`: `max_flow`, `edmonds_karp`, `dinic`, `push_relabel`, `minimum_st_cut`, `gomory_hu_tree` (deleted; Goldberg and k-connectivity now use `PushRelabelMFImpl` and `EdmondsKarpMFImpl`, the old `goldberg_densest_subgraph` is gone) | the class implementations in `flow/` | |
| C3 | `flow/legacy.rs`: `min_cost_flow` (SPFA successive shortest paths) and its `MinimumCostFlowProblem` struct | `CapacityScalingMinimumCostFlow`. Remaining callers: `paths.rs` (goes with C13/1i, Suurballe) and netgen, which must return `MinimumCostFlowProblemImpl` | `paths.rs`, `generate/netgen` |
| C4 | **done** `flow/legacy.rs`: `stoer_wagner` (deleted, callers use B1) | B1 | |
| C5 | `tour/legacy.rs` (nearest neighbour, insertion, double tree, 2-opt, Held–Karp) | the tour classes | |
| C6 | `tour/legacy.rs`: `christofides_tour` **[needs 1g]** | `ChristofidesThreeHalvesApproxMetricTSP` on the Java matching | |
| C7 | `cycle/legacy.rs` (`has_cycle`, `simple_cycles`, `cycle_basis`, `cycle_to_walk`) | `CycleDetector`, the simple-cycle classes, cycle-basis classes | |
| C8 | `hamiltonian.rs` (backtracking) | `tour` Hamiltonian-path/cycle classes | |
| C9 | `eulerian.rs` | `HierholzerEulerianCycle` | |
| C10 | `chordal.rs` | `ChordalityInspector` | |
| C11 | `planarity.rs` (left-right) | `BoyerMyrvoldPlanarityInspector`; `props::is_planar` delegates | many callers |
| C12 | `matching.rs` **[needs 1g]** | Dense/Sparse Edmonds, Kuhn–Munkres, greedy, path-growing, Blossom V | |
| C13 | `paths.rs` (`all_simple_paths`, `find_negative_cycle`, edge/vertex-disjoint shortest paths) | Java classes (`AllDirectedPaths`, `BellmanFord`-based, `SuurballeKDisjointShortestPaths`, ...) | |
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
