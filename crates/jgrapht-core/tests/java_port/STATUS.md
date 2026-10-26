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
