# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

- Benchmarks no longer run on pull requests. `scripts/bench.sh` (local) and the manual `Benchmarks`
  workflow run them on demand, optionally against a base ref.

- `GraphError::InvalidArgument(String)` is no longer returned by the library. Its remaining uses
  (about 150 call sites, most of them in the generators) are typed: `InvalidParameter` with new
  `ParameterRequirement`s (`InRange`, `AbsAtMost`, `AtMostVertexCount`, `Consistent`, `FitsArea`,
  `ValidPartition`, `ValidSolution`), `InvalidGraph` with new `GraphProperty`s (`OreProperty`,
  `OneRootPerTree`, `SameDirection`), and new variants `MissingParameter`, `MissingVertexData`,
  `TooManyVertices`, `InvalidState`, `DivisionByZero`, `NonTerminatingExpansion` and `Internal`.
- Doc comments that only restated the item name now say what the item does.
- `serde`: `AsSubgraph` is serializable and validated against its base graph on input.

- `jgrapht`: optional `serde` feature, the counterpart of JGraphT's `java.io.Serializable`. It covers
  `BaseGraph` (rebuilt through `add_edge_with`, so loops, parallel edges and acyclicity are
  checked on input), the four sparse graphs, `GraphType`, `DefaultEdge`, `GraphWalk` (cardinality
  checked on input), the plain views (`Unmodifiable`, `EdgeReversed`, `AsUndirected`,
  `AsUnweighted`, `GraphUnion`) and the result types (`Coloring`, `SpanningTree`, `Matching`,
  `MaximumFlow`, `MinimumCut`, `CycleBasis`, `SingleSourcePaths`, ...). Maps keyed by vertices are
  written as sequences of pairs, so every serde format works. Factories are function pointers and
  are not serialized: use `with_default_edge_factory` or `add_edge_with` after reading.

### 0.1.0 (first release, pending tag)

- `jgrapht`: Rust port of the JGraphT core library: graph traits with capability bounds
  (`OutgoingGraph`, `IncomingGraph`, `IncidenceGraph`), `BaseGraph`, sparse and map graphs,
  views, generators, traversals, and class-by-class ports of the `alg` packages (matching incl.
  Blossom V, planarity, flow, tours, cycles, isomorphism, shortest paths incl. contraction
  hierarchies and transit node routing).
- `jgrapht-io`: importers and exporters for DIMACS, CSV, graph6/sparse6/digraph6, JSON, GML, DOT,
  GraphML, GEXF, Lemon, Matrix and TSPLIB.
- Rust-native API: `Option` for absent values (unreachable is `None`), typed `GraphError`,
  result objects instead of stateful getters, no panics on valid public input.
- See `MIGRATION.md` for differences from the Java API and `docs/FIDELITY_ROADMAP.md` for
  deviations from Java behavior.
