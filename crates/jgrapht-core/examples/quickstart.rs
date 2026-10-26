use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
use jgrapht::alg::spanning::kruskal;
use jgrapht::*;

fn main() -> Result<()> {
    // A weighted, undirected simple graph over `&str` vertices.
    let mut g = UGraph::<&str>::simple().weighted();
    g.add_vertices(["a", "b", "c", "d"]);
    g.add_weighted_edge(&"a", &"b", 1.0)?;
    g.add_weighted_edge(&"b", &"c", 2.0)?;
    g.add_weighted_edge(&"a", &"c", 5.0)?;
    g.add_weighted_edge(&"c", &"d", 1.0)?;

    // Shortest path with Dijkstra (pairing heap from `rheaps` by default).
    let path = Dijkstra::new(&g).path(&"a", &"d")?.expect("d is reachable");
    println!("a -> d: {:?} (weight {})", path.vertices, path.weight);

    // Minimum spanning tree.
    let mst = kruskal(&g);
    println!("MST weight: {}", mst.weight);
    Ok(())
}
