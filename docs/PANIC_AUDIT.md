# Panic audit (roadmap R9)

Every production `unwrap`, `expect`, `panic!` and slice or array index in the two library crates is
listed in [`panic_sites.tsv`](panic_sites.tsv): 5420 clippy hits (`unwrap_used`, `expect_used`,
`panic`, `indexing_slicing` on `--lib`), of which 751 were `unwrap`/`expect` and none is `panic!`. Since then 118 of the `unwrap`/`expect`
sites were removed (633 remain on 2026-10-28): graph-query unwraps in functions that return
`Result` or `Option` now return `EdgeNotFound` or propagate `None`, lazily computed state returns
`GraphError::Internal`, and the JSON and DOT importers return `IoError`. The rows of
`panic_sites.tsv` were not regenerated and their line numbers are out of date; regenerate it with the
command above when it is needed.
Each hit, indexing included, has its own row. Regenerate with
`cargo clippy -p <crate> --lib -- -W clippy::unwrap_used -W clippy::expect_used -W clippy::panic -W clippy::indexing_slicing`.

| Class | Rows | Meaning |
|---|---|---|
| indexing contract | 4669 | Dense array indexed by an `Indexed` position, vertex number or loop bound produced by the same routine. |
| internal invariant | 555 | The value was inserted, linked or computed earlier in the same routine, or a constructor was called with constant valid arguments. |
| third-party trait invariant | 196 | A `Graph` implementation yields edges that have vertex endpoints and a weight (documented on the trait), or a mutex was poisoned because another thread already panicked. |
| defect | 0 open | Five were found and fixed, see below. |

How the classes were assigned: the `unwrap`/`expect` sites were first sorted by rules on the
surrounding code, then the 258 that no rule covered were read one by one; the lazy-compute `Option`
fields, the list and tree node links in `tree_connectivity`, the Gusfield/Padberg-Rao tree and the
Eppstein path graph were checked against the code that fills them. Indexing sites were classified by
rule only: reading all 4669 was not done. The claim that valid input cannot reach them is tested
instead (below), and the sweep found three panics or hangs that rule-based reading had missed.

## Defects found by the audit

- `HierholzerEulerianCycle::is_eulerian` unwrapped `try_is_eulerian`, which could no longer fail.
  `is_eulerian` is now infallible and `try_is_eulerian` is gone.
- `JsonImporter`: a `\u` escape followed by a multibyte character cut the four-byte window in the
  middle of the character and `hex4` panicked (`expect("ASCII hex bytes")`). It now returns
  `IoError::Import`.

- `WeakChordalityInspector` indexed past its label buckets on graphs with parallel edges or
  self-loops (duplicate neighbours); neighbours are now listed once and without self-loops.
- `PushRelabelMFImpl` and `BoykovKolmogorovMFImpl` never returned for a negative capacity on a
  residual cycle (and BK accepted NaN). Their public constructors now reject negative and NaN
  capacities like Dinic and Edmonds-Karp; Goldberg's density network, which needs negative arcs,
  uses a `pub(crate)` constructor.
- `BellmanFord::get_paths` never returned for infinite or NaN weights; it now returns
  `InvalidWeight`.

## Second sweep

`alg_no_panic_more_tests.rs` extends the random-pseudograph sweep to matching (nine classes),
tours (ten), simple cycles and cycle bases, Eulerian and Chinese-postman cycles, planarity, VF2,
colour refinement, AHU, Stoer-Wagner, Gusfield and Padberg-Rao. It found four more defects, all fixed:

- `BoyerMyrvoldPlanarityInspector` reported a planar graph with a self-loop or parallel edges as
  non-planar and then panicked extracting the Kuratowski subdivision. Self-loops and repeated
  edges are now skipped while building the DFS structure.
- `ChinesePostman::get_cpp_solution` guarded connectivity with a `debug_assert!`; a disconnected
  graph now returns `InvalidGraph(Connected)`.
- `HopcroftKarpMaximumCardinalityBipartiteMatching` indexed out of bounds on an edge to a vertex
  outside both partitions. Edges that do not join the partitions are ignored, and overlapping
  partitions return `InvalidParameter { requirement: Disjoint }`.
- `AHU*` inspectors guarded simplicity with `debug_assert!`; they now return `InvalidGraph(Simple)`.

## Evidence for "valid public input cannot trigger an undocumented panic"

- `crates/jgrapht-io/tests/malformed_input.rs` truncates, deletes and overwrites bytes of a seed
  document for each importer (CSV three formats, DIMACS, DOT, GEXF, GML, graph6, GraphML, JSON,
  TSPLIB) and fails on any panic. It reproduces the JSON defect above when the fix is removed.
- `tests/common/fixtures.rs` and the capability tests run algorithms on graphs that break the
  `Graph` integrity contract only through the exporters (which must report an error), and on empty,
  single-vertex, self-loop, parallel-edge and disconnected graphs.
- Public functions that can reach an `unwrap`, `expect` or `assert!` carry a `# Panics` section,
  and public functions returning `Result` carry `# Errors`. Both were generated from the function
  body, so they name the panic class and the error variants the body mentions, not the exact input.
  Trait methods declared without `pub` were not covered.
- The three explicit assertions (`next_int_bound`, `Queue::new`, `Queue::enqueue`) are cheap
  precondition checks on the caller's argument and stay as `assert!` with a `# Panics` section;
  there is no expensive consistency check left to turn into `debug_assert!`.

- `crates/jgrapht/tests/alg/alg_no_panic_tests.rs` runs about 60 algorithms (cliques, colorings,
  connectivity, covers, spanning trees, scoring, graph properties, topological order, five
  shortest-path algorithms, four max-flow algorithms) on 300 random pseudographs each for several
  weight sets (including zero and negative, and NaN and infinite for the fallible entry points). It
  found the three defects above.

## Not covered

The two sweeps cover the algorithms listed, not all of them (drawing, generators, scoring variants,
decomposition, LCA, steiner, similarity and the remaining flow and cut algorithms are exercised
only by the fixture and parity tests), so the acceptance statement is tested for a large part of the surface,
not proven for all of it.
