# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### 0.1.0 (first release, pending tag)

- `jgrapht`: Rust port of the JGraphT core library: graph traits with capability bounds
  (`OutgoingGraph`, `IncomingGraph`, `IncidenceGraph`), `BaseGraph`, sparse and map graphs,
  views, generators, traversals, and class-by-class ports of the `alg` packages (matching incl.
  Blossom V, planarity, flow, tours, cycles, isomorphism, shortest paths incl. contraction
  hierarchies and transit node routing).
- `jgrapht-io`: importers and exporters for DIMACS, CSV, graph6/sparse6/digraph6, JSON, GML, DOT,
  GraphML, GEXF, Lemon, Matrix and TSPLIB.
- Rust-native API: `Option` for absent values (unreachable is `None`), typed `GraphError`,
  result objects instead of stateful getters, no panics on valid public input
  (see `docs/PANIC_AUDIT.md`).
- See `MIGRATION.md` for differences from the Java API and `docs/FIDELITY_ROADMAP.md` for
  deviations from Java behavior.
