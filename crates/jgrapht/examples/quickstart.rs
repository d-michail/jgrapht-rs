use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
use jgrapht::alg::spanning::kruskal;
use jgrapht::*;

fn main() -> Result<()> {
    let mut graph = UGraph::<&str>::simple().weighted();
    graph.add_vertices(["a", "b", "c", "d"]);
    graph.add_weighted_edge(&"a", &"b", 1.0)?;
    graph.add_weighted_edge(&"b", &"c", 2.0)?;
    graph.add_weighted_edge(&"a", &"c", 5.0)?;
    graph.add_weighted_edge(&"c", &"d", 1.0)?;

    let path = Dijkstra::new(&graph)
        .path(&"a", &"d")?
        .expect("d is reachable");
    println!("a -> d: {:?} (weight {})", path.vertices, path.weight);
    println!("MST weight: {}", kruskal(&graph).weight);
    Ok(())
}
