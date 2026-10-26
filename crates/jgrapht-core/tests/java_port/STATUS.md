# Port of the JGraphT test suite

Each Java test class under `jgrapht-core/src/test/java/org/jgrapht` maps to a Rust module in this
test binary (`cargo test --test java_port`). Test method names are snake_cased Java names.

Legend: **done** = every `@Test` ported; **partial** = some skipped (see notes); **n/a** = tests a Java
mechanism that has no Rust counterpart (serialization, JVM collections, thread-safety wrappers...).

| Java test | Rust module | State |
|---|---|---|
| `GraphsTest` | `graphs_test` | done |
| `GraphTestsTest` | `graph_tests_test` | done |
| `GraphMetricsTest` | `graph_metrics_test` | done |
| `FastTestSuite`, `IntegrationTestSuite`, `PerformanceTestSuite` | - | n/a (JUnit suite aggregators) |
| `TestUtil`, `GraphTestsUtils` | `common` | helpers |
| `graph/DirectedGraphTest` | `graph_basic::directed_graph_test` | done |
| `graph/WeightedGraphTest` | `graph_basic::weighted_graph_test` | done (null/IllegalArgument checks mapped to `None`/`Err`) |
| `graph/IncomingOutgoingEdgesTest` | `graph_basic::incoming_outgoing_edges_test` | done |
| `graph/GenericGraphsTest` | `graph_basic::generic_graphs_test` | partial: Foo/Bar vertex variance and intrusive-edge `toString` are Java-only |
| `graph/EqualsAndHashCodeTest` | `graph_basic::equals_and_hash_code_test` | done |
| `graph/CloneTest` | `graph_basic::clone_and_unmodifiable_test` | partial: `ParanoidGraph` n/a |
| `graph/AsUnmodifiableGraphTest` | `graph_basic::clone_and_unmodifiable_test` | partial: mutation attempts are compile-time errors |
| `graph/SimpleDirectedGraphTest` | `simple_directed_graph_test` | done (cross-graph `IntrusiveEdgeException` n/a) |
| `graph/SimpleIdentityDirectedGraphTest` | - | n/a: identity-hash vertex maps; same API surface as `SimpleDirectedGraphTest` |
| `graph/DirectedAcyclicGraphTest` | `directed_acyclic_graph_test` | partial: fail-fast iterator (`testIterationBehaviors`) n/a |
| `graph/GraphWalkTest` | `graph_walk_test` | done |
| `graph/ListenableGraphTest` | `listenable_graph_test` | done |
| `graph/AsUndirectedGraphTest` | `graph_views::as_undirected_graph_test` | done |
| `graph/AsUnweightedGraphTest` | `graph_views::as_unweighted_graph_test` | done |
| `graph/UnweightedGraphAsWeightedGraphTest`, `graph/WeightedGraphAsWeightedGraphTest` | `graph_views::as_weighted_graph_tests` | done |
| `graph/AsSubgraphTest` | `graph_views::as_subgraph_test` | partial: base-change tracking via listeners n/a (views borrow their base) |
| `graph/MaskSubgraphTest` | `graph_views::mask_subgraph_test` | done |
| `graph/AsGraphUnionTest` | `graph_views::as_graph_union_test` | done (shared edge weights are per graph, see note in file) |
| `graph/MaskVertexSetTest`, `graph/MaskEdgeSetTest` | - | n/a: Java `Set` classes |
| `graph/SerializationTest`, `graph/concurrent/AsSynchronizedGraphTest` | - | n/a |
