# jgrapht-rs

[![CI](https://github.com/d-michail/jgrapht-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/d-michail/jgrapht-rs/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-EPL--2.0%20OR%20LGPL--2.1--or--later-blue)](#license)
![MSRV](https://img.shields.io/badge/rustc-1.88%2B-orange)

A graph library for Rust: graph data structures, a large collection of graph algorithms, graph
generators and import/export for common file formats. It is a port of
[JGraphT](https://jgrapht.org), redesigned for Rust.

- **Graphs are traits.** Algorithms ask for the narrowest capability they need (`Graph`,
  `OutgoingGraph`, `IncomingGraph`, `IncidenceGraph`), so a graph that cannot answer a query does
  not pretend to.
- **Your own vertex and edge types** (`Clone + Eq + Hash`); the default edge is `DefaultEdge`.
- **No hidden state.** Iterators borrow from the graph, views are plain wrapper types, and results
  are plain structs instead of getters that only work after a `compute()` call.
- **Errors are values.** Fallible operations return `Result<_, GraphError>` with typed variants;
  absent values (an unreachable vertex, no cycle) are `Option`.
- **Compact graphs.** Sparse CSR graphs keep only the edge arrays; the default graph is an
  insertion-ordered adjacency map.
- **Optional `serde` support** for graphs and result types, in any serde format.
- Heap-based algorithms (Dijkstra, A\*, Prim, Johnson, ...) use the priority queues from
  [`rheaps`](https://crates.io/crates/rheaps).

## Installation

Add the crates as dependencies:

```toml
[dependencies]
jgrapht = "1.0.0"
jgrapht-io = "1.0.0"   # importers and exporters
```

The optional `serde` feature of `jgrapht` serializes graphs and results. The documentation starts
with the [user guide](crates/jgrapht/src/guide.rs) (`cargo doc --open`).

## Crates

- `jgrapht` contains graph traits, mutable and immutable graph implementations, algorithms,
  traversal and generators. The CSR sparse graphs (`graph::sparse`) and map-backed graphs
  (`graph::map`) also live here because they are graph storage choices rather than optional
  integrations.
- `jgrapht-io` is the companion import/export crate. All supported formats live in this one crate
  as modules, keeping parser dependencies out of `jgrapht` without creating a crate per format.

## Quick start

```rust
use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
use jgrapht::alg::spanning::kruskal;
use jgrapht::*;

fn main() -> Result<()> {
    let mut g = UGraph::<&str>::simple().weighted();
    g.add_vertices(["a", "b", "c", "d"]);
    g.add_weighted_edge(&"a", &"b", 1.0)?;
    g.add_weighted_edge(&"b", &"c", 2.0)?;
    g.add_weighted_edge(&"a", &"c", 5.0)?;
    g.add_weighted_edge(&"c", &"d", 1.0)?;

    let path = Dijkstra::new(&g).path(&"a", &"d")?.expect("d is reachable");
    println!("a -> d: {:?} (weight {})", path.vertices, path.weight);
    println!("MST weight: {}", kruskal(&g).weight);
    Ok(())
}
```

From the repository root, run it with:

```sh
cargo run --example quickstart
```

## Using the library

- Build graphs with `UGraph` (undirected) and `DiGraph` (directed): `UGraph::simple()`, `::multi()`,
  `::pseudo()`, `DiGraph::dag()`, then `.weighted()`. `GraphType` describes what a graph allows.
- `GraphExt`, `DirectedExt` and `GraphMutExt` add convenience methods to every graph;
  `graph::props` tests structural properties (bipartite, planar, Eulerian, ...).
- Views (`AsSubgraph`, `MaskSubgraph`, `EdgeReversed`, `AsUndirected`, `AsWeighted`, `AsUnweighted`,
  `Unmodifiable`, `GraphUnion`) borrow their base graph; `event::Listenable` reports changes.
- Algorithms are small traits (for example `ShortestPathAlgorithm`) plus plain functions; paths are
  `GraphWalk`s.
- Weights live on edges as `f64` (`1.0` for unweighted graphs). Use the built-in `DefaultEdge`, or
  your own edge type with `BaseGraph::with_edge_factory` or `add_edge_with`.
- Enable the `serde` feature to serialize graphs and result objects.

The rustdoc of the crate root (`cargo doc --open`) explains the design decisions that are not
obvious from the signatures. `AGENTS.md` describes the conventions the code follows: absent values
and errors, capability bounds, vertex references, views and snapshots, randomness and serialization.
## Algorithms

- **Shortest paths:** Dijkstra (any `rheaps` decrease-key heap), bidirectional Dijkstra, A\*,
  BFS, Bellman–Ford, delta-stepping, Floyd–Warshall, Johnson, Yen's k shortest paths
- **Traversal:** BFS, DFS (pre/post-order), topological order, closest-first, random walk,
  lexicographic BFS, maximum cardinality search, degeneracy ordering
- **Connectivity & structure:** connected and strongly connected components (Tarjan,
  Kosaraju), condensation, articulation points/bridges/blocks and the block–cut tree, Eulerian
  circuits, simple cycles, cycle bases, chordal graphs, transitive closure/reduction,
  LCA, Boyer–Myrvold planarity testing and embedding, (sub)graph isomorphism,
  Hamiltonian cycles/paths, negative cycle detection, simple paths and edge-disjoint shortest paths
- **Spanning trees & flows:** Kruskal, Prim, Borůvka, minimum/maximum arborescences
  (Chu–Liu/Edmonds), max flow (Edmonds–Karp, Dinic, push–relabel), minimum s–t cuts,
  Gomory–Hu trees, Stoer–Wagner, minimum cost flow
- **Matching & colouring:** Hopcroft–Karp, Kuhn–Munkres, dense/sparse Edmonds, greedy and
  path-growing matching, and Kolmogorov Blossom V weighted matching;
  greedy / largest-degree-first / smallest-degree-last / DSATUR / chordal colouring, colour refinement
- **Combinatorial:** maximal cliques (Bron–Kerbosch), independent sets, vertex covers
  (greedy, 2-approximations, König), bipartite partitioning
- **Analysis:** betweenness, closeness, harmonic, degree, PageRank, eigenvector, Katz,
  coreness, clustering coefficients, diameter/radius/center/periphery/girth,
  link prediction and similarity measures, label propagation, k-spanning-tree clustering
- **Tours & transforms:** nearest-neighbour, insertion, double-tree, Christofides, 2-opt,
  Held–Karp; complement, line graph, edge contraction
- **Other:** Steiner trees (KMB, Dreyfus–Wagner), greedy spanners, tree decompositions
  (min-degree, min-fill), densest subgraph (greedy, Goldberg), k-center, circular / random /
  Fruchterman–Reingold layouts
- **Generators:** complete, bipartite, ring, path, star, wheel, grid, hypercube, G(n,p), G(n,m),
  Barabási–Albert, Watts–Strogatz, Prüfer trees

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
```

The test suites can also be run independently:

```sh
cargo test -p jgrapht --test alg
cargo test -p jgrapht --test graph
cargo test -p jgrapht --test generate
cargo test -p jgrapht --test traverse
cargo test -p jgrapht --test util
cargo test -p jgrapht --test delta_stepping
cargo test -p jgrapht-io
```

## Benchmarks

Benchmarks never run automatically. Start them on demand:

```sh
cargo bench-graph                        # one criterion run of one file; also bench-shortestpath, -algorithms, -structures, -pathsearch, bench-all
scripts/bench.sh graph_perf              # 10 independent runs, mean and standard deviation per benchmark
scripts/bench.sh -b main graph_perf    # also run main, interleaved; Welch t-test, fails on a significant >30% slowdown
```

`scripts/bench.sh` (and the manual **Benchmarks** workflow on GitHub: Actions tab, Run workflow) run
at least 10 independent `cargo bench` processes, because one criterion run is a single observation
and says nothing about run-to-run variance. Each run's mean is the observation; the table shows
mean, standard deviation and coefficient of variation over the runs. With `-b <ref>` the two
revisions alternate (base, head, base, ...) so machine drift affects both, and a slowdown only fails
when it exceeds the threshold and is statistically significant (Welch t-test, p < 0.05). All runs execute strictly one after the other on a single machine (the workflow is one job on one
runner, no matrix). Use an
idle machine: shared runners are noisy, and numbers are only comparable on the same machine.
Criterion's HTML reports are in `target/criterion/report/index.html`.

The benchmark files are in `crates/jgrapht/benches/` (`graph_perf`, `shortestpath_perf`,
`algorithms_perf`, `structures_perf`, `pathsearch_perf`).

## Cite

If you use this library please cite the following paper:

* D. Michail, J. Kinable, B. Naveh, J. V. Sichi. **JGraphT—A Java Library for Graph Data Structures and Algorithms.** ACM Journal of Experimental Algorithmics, 25, 2020. [web](https://doi.org/10.1145/3381449)

```
@article{michail2020jgrapht,
      title={JGraphT---A Java Library for Graph Data Structures and Algorithms},
      author={Michail, Dimitrios and Kinable, Joris and Naveh, Barak and Sichi, John V.},
      journal={ACM Journal of Experimental Algorithmics},
      volume={25},
      year={2020},
      publisher={ACM},
      doi={10.1145/3381449},
      url={https://doi.org/10.1145/3381449},
}
```

Citing software is just as important as citing any other important sources in your research. If you're not sure whether or not to cite something, [Shouldacite](https://mr-c.github.io/shouldacite/index.html) can help you decide if you should.

## License

This project is a port of [JGraphT](https://github.com/jgrapht/jgrapht) and, like upstream, is
licensed under your choice of either the [Eclipse Public License 2.0](license-EPL.txt) or the
[GNU Lesser General Public License v2.1 or later](license-LGPL.txt)
(`SPDX-License-Identifier: EPL-2.0 OR LGPL-2.1-or-later`).

Copyright in ported material remains with the original JGraphT authors; see
[CONTRIBUTORS.md](CONTRIBUTORS.md) and [NOTICE](NOTICE).
