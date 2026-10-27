use jgrapht::alg::shortestpath::{
    FloydWarshallShortestPaths, JohnsonShortestPaths, ShortestPathAlgorithm,
};
use jgrapht::generate::{GnpRandomGraphGenerator, GraphGenerator};
use jgrapht::{DiGraph, Graph, GraphMut, GraphMutExt};
use rand::{Rng, SeedableRng, rngs::StdRng};

#[test]
fn test_issue_408() {
    let mut graph = DiGraph::<i32>::simple();
    graph.add_vertices(0..7);
    for (source, target) in [(0, 1), (1, 2), (2, 3), (3, 0), (4, 5), (5, 6), (6, 4)] {
        graph.add_edge(&source, &target).unwrap();
    }

    let algorithm = JohnsonShortestPaths::new(&graph).unwrap();
    assert_eq!(algorithm.path_weight(&0, &2).unwrap(), 2.0);
    assert_eq!(algorithm.path_weight(&4, &5).unwrap(), 1.0);
    assert_eq!(algorithm.path_weight(&3, &4).unwrap(), f64::INFINITY);
}

#[test]
fn test_wikipedia_example() {
    let mut graph = DiGraph::<&str>::pseudo().weighted();
    graph.add_vertices(["w", "y", "x", "z"]);
    for (source, target, weight) in [
        ("w", "z", 2.0),
        ("y", "w", 4.0),
        ("x", "w", 6.0),
        ("x", "y", 3.0),
        ("z", "x", -7.0),
        ("y", "z", 5.0),
        ("z", "y", -3.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }

    let algorithm = JohnsonShortestPaths::new(&graph).unwrap();
    assert_eq!(algorithm.path_weight(&"z", &"w").unwrap(), -1.0);
    assert_eq!(algorithm.path_weight(&"z", &"y").unwrap(), -4.0);
    assert_eq!(algorithm.path_weight(&"z", &"z").unwrap(), 0.0);
    assert_eq!(algorithm.path_weight(&"z", &"x").unwrap(), -7.0);
}

#[test]
fn test_random_graphs_compare_with_floyd_warshall() {
    const N: usize = 50;
    let mut generator =
        GnpRandomGraphGenerator::new(N, 0.55, StdRng::seed_from_u64(0x408), false).unwrap();
    let mut random = StdRng::seed_from_u64(0x409);

    for _ in 0..20 {
        let mut graph = DiGraph::<usize>::pseudo()
            .weighted()
            .with_vertex_factory(|index| index as usize);
        generator.generate_default(&mut graph).unwrap();
        let edges = graph.edges().copied().collect::<Vec<_>>();
        for edge in edges {
            let source = *graph.edge_source(&edge).unwrap();
            let target = *graph.edge_target(&edge).unwrap();
            let weight = if source >= target {
                (N + 1) as f64 + 2.0 * (N + 1) as f64 * random.random::<f64>()
            } else {
                let value = random.random::<f64>();
                if random.random_bool(0.5) {
                    -value
                } else {
                    value
                }
            };
            graph.set_edge_weight(&edge, weight).unwrap();
        }

        let johnson = JohnsonShortestPaths::new(&graph).unwrap();
        let floyd_warshall = FloydWarshallShortestPaths::new(&graph).unwrap();
        for source in graph.vertices() {
            for target in graph.vertices() {
                let expected = floyd_warshall.path_weight(source, target).unwrap();
                let actual = johnson.path_weight(source, target).unwrap();
                assert!(
                    (expected - actual).abs() < 1e-9
                        || (expected.is_infinite() && actual.is_infinite()),
                    "distance {source} -> {target}: expected {expected}, got {actual}"
                );
            }
        }
    }
}
