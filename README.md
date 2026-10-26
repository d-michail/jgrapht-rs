# jgrapht-rs

A Rust port of the core of [JGraphT](https://jgrapht.org), the Java graph library.

It is a re-design rather than a transliteration: graphs are **traits**, vertices and edges are
**your own types** (`Clone + Eq + Hash`), iterators borrow from the graph instead of building
sets, views are plain wrapper types, and fallible operations return `Result` instead of throwing.
Heap-based algorithms (Dijkstra, A\*, Prim, Johnson, ...) use the priority queues from
[`rheaps`](https://crates.io/crates/rheaps), the Rust port of JHeaps.

> Status: early. The API is not stable and the crate is not yet published.

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

Run it with `cargo run --example quickstart`.

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
| `GraphIterator`s                       | `traverse` iterators (`Bfs`, `Dfs`, `TopologicalOrder`, ...)        |
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
  Kosaraju), condensation, articulation points/bridges/blocks, Eulerian circuits, simple
  cycles (Johnson), cycle bases, chordal graphs, transitive closure/reduction, LCA
- **Spanning trees & flows:** Kruskal, Prim, Borůvka, minimum/maximum arborescences
  (Chu–Liu/Edmonds), max flow (Edmonds–Karp, Dinic, push–relabel), minimum s–t cuts,
  Gomory–Hu trees, Stoer–Wagner, minimum cost flow
- **Matching & colouring:** Hopcroft–Karp, Hungarian, Edmonds' blossom, greedy matching;
  greedy / largest-degree-first / smallest-degree-last / DSATUR / chordal colouring, colour refinement
- **Combinatorial:** maximal cliques (Bron–Kerbosch), independent sets, vertex covers
  (greedy, 2-approximations, König), bipartite partitioning
- **Analysis:** betweenness, closeness, harmonic, degree, PageRank, eigenvector, Katz,
  coreness, clustering coefficients, diameter/radius/center/periphery/girth,
  link prediction and similarity measures, label propagation, k-spanning-tree clustering
- **Tours & transforms:** nearest-neighbour, insertion, double-tree, 2-opt, Held–Karp;
  complement, line graph, edge contraction
- **Generators:** complete, bipartite, ring, path, star, wheel, grid, hypercube, G(n,p), G(n,m),
  Barabási–Albert, Watts–Strogatz, Prüfer trees

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
```

The Java sources used as the reference live in a separate checkout of JGraphT.

## License

JGraphT is dual-licensed under EPL 2.0 and LGPL 2.1+; this port follows the same terms.
