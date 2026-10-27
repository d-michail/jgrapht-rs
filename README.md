# jgrapht-rs

A Rust port of the core of [JGraphT](https://jgrapht.org), the Java graph library.

It is a re-design rather than a transliteration: graphs are **traits**, vertices and edges are
**your own types** (`Clone + Eq + Hash`), iterators borrow from the graph instead of building
sets, views are plain wrapper types, and fallible operations return `Result` instead of throwing.
Heap-based algorithms (Dijkstra, A\*, Prim, Johnson, ...) use the priority queues from
[`rheaps`](https://crates.io/crates/rheaps), the Rust port of JHeaps.

> **Status:** pre-release. The API is unstable, and neither crate is published yet. The workspace
> requires Rust 1.88 or newer.
>
> The graph, traversal, generator, sparse/map backend, and graph-I/O ports are complete. Most
> algorithm packages are also complete; the remaining fidelity work is in shortest paths. See the
> [core status register](crates/jgrapht/tests/STATUS.md),
> [I/O status register](crates/jgrapht-io/tests/STATUS.md), and
> [fidelity roadmap](docs/FIDELITY_ROADMAP.md) for the exact coverage and documented deviations.

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

## Design

| JGraphT (Java)                         | jgrapht-rs                                                         |
|----------------------------------------|--------------------------------------------------------------------|
| `Graph<V,E>` interface                 | `Graph`, `DirectedGraph`, `UndirectedGraph` (read), `GraphMut` (write) |
| `SimpleGraph`, `Multigraph`, ...       | `BaseGraph` + `GraphType`: `UGraph::simple()`, `::multi()`, `::pseudo()`, `DiGraph::dag()`, `.weighted()` |
| `Graphs` utility class                 | `GraphExt`, `DirectedExt`, `GraphMutExt` extension traits           |
| `GraphTests`                           | `graph::props`                                                     |
| `AsSubgraph`, `EdgeReversedGraph`, ... | `view` wrappers: `AsSubgraph`, `MaskSubgraph`, `EdgeReversed`, `AsUndirected`, `AsWeighted`, `AsUnweighted`, `Unmodifiable`, `GraphUnion` |
| `ListenableGraph`                      | `event::Listenable` + `GraphListener`                               |
| `GraphPath` / `GraphWalk`              | `GraphWalk`                                                        |
| `GraphIterator`                        | `traverse` iterators (`Bfs`, `Dfs`, `TopologicalOrder`, ...)        |
| generators                             | `generate` (`GraphGenerator` trait)                                |
| `*Algorithm` interfaces                | small traits (e.g. `ShortestPathAlgorithm`) plus plain functions   |

Edges are values too: use the built-in `DefaultEdge`, or your own type via
`BaseGraph::with_edge_factory`. Weights live on edges as `f64` (`1.0` for unweighted graphs).

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

## Java-to-Rust package map

| JGraphT package | Rust location | Port status |
|---|---|---|
| `org.jgrapht` and `org.jgrapht.graph` | `jgrapht::{Graph, GraphMut, GraphType}`, `graph`, `view`, `event`, `walk` | done |
| `org.jgrapht.traverse` | `jgrapht::traverse` | done |
| `org.jgrapht.generate` and `generate.netgen` | `jgrapht::generate` | done |
| `org.jgrapht.alg` except `alg.shortestpath` | `jgrapht::alg` | done class by class |
| `org.jgrapht.alg.shortestpath` | `jgrapht::alg::shortestpath` | partial; see the fidelity roadmap |
| `org.jgrapht.nio` | companion crate `jgrapht-io` | done for all planned formats |
| `org.jgrapht.opt.graph.sparse` | `jgrapht::graph::sparse` | done |
| `org.jgrapht.opt.graph.fastutil` | `jgrapht::graph::map` | done with Rust map backends |

The Java integration modules (`ext`, `guava`, `unimi-dsi`, `osm`, and demos) are currently out of
scope. The status registers linked above are the source of truth for individual upstream test
classes.

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
```

The Java-port suites can also be run independently:

```sh
cargo test -p jgrapht --test alg
cargo test -p jgrapht --test graph
cargo test -p jgrapht --test generate
cargo test -p jgrapht --test traverse
cargo test -p jgrapht --test util
cargo test -p jgrapht --test delta_stepping
cargo test -p jgrapht-io
```

The Java sources used as the oracle live in a separate JGraphT checkout. Rust test modules mirror
the upstream test classes, while `crates/jgrapht/tests/STATUS.md` and
`crates/jgrapht-io/tests/STATUS.md` record ported and deliberately inapplicable cases.

## License

This project is a port of [JGraphT](https://github.com/jgrapht/jgrapht) and, like upstream, is
licensed under your choice of either the [Eclipse Public License 2.0](license-EPL.txt) or the
[GNU Lesser General Public License v2.1 or later](license-LGPL.txt)
(`SPDX-License-Identifier: EPL-2.0 OR LGPL-2.1-or-later`).

Copyright in ported material remains with the original JGraphT authors; see
[CONTRIBUTORS.md](CONTRIBUTORS.md) and [NOTICE](NOTICE).
