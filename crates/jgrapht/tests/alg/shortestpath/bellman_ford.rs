use jgrapht::alg::shortestpath::BellmanFordShortestPath;
use jgrapht::{DefaultEdge, DiGraph, GraphMutExt, UGraph};

type Edges = [DefaultEdge; 6];

fn undirected_fixture() -> (UGraph<&'static str>, Edges) {
    let mut graph = UGraph::<&str>::simple().weighted();
    graph.add_vertices(["v1", "v2", "v3", "v4", "v5"]);
    let edges = [
        graph.add_weighted_edge(&"v1", &"v2", 2.0).unwrap().unwrap(),
        graph.add_weighted_edge(&"v1", &"v3", 3.0).unwrap().unwrap(),
        graph
            .add_weighted_edge(&"v1", &"v5", 100.0)
            .unwrap()
            .unwrap(),
        graph.add_weighted_edge(&"v2", &"v4", 5.0).unwrap().unwrap(),
        graph
            .add_weighted_edge(&"v3", &"v4", 20.0)
            .unwrap()
            .unwrap(),
        graph.add_weighted_edge(&"v4", &"v5", 5.0).unwrap().unwrap(),
    ];
    (graph, edges)
}

fn negative_fixture() -> (DiGraph<&'static str>, Edges) {
    let mut graph = DiGraph::<&str>::simple().weighted();
    graph.add_vertices(["v1", "v2", "v3", "v4", "v5"]);
    let edges = [
        graph
            .add_weighted_edge(&"v1", &"v2", -2.0)
            .unwrap()
            .unwrap(),
        graph
            .add_weighted_edge(&"v1", &"v3", -3.0)
            .unwrap()
            .unwrap(),
        graph
            .add_weighted_edge(&"v1", &"v5", -100.0)
            .unwrap()
            .unwrap(),
        graph
            .add_weighted_edge(&"v2", &"v4", -5.0)
            .unwrap()
            .unwrap(),
        graph
            .add_weighted_edge(&"v3", &"v4", -20.0)
            .unwrap()
            .unwrap(),
        graph
            .add_weighted_edge(&"v4", &"v5", -5.0)
            .unwrap()
            .unwrap(),
    ];
    (graph, edges)
}

#[test]
fn test_path_between() {
    let (graph, [e12, e13, _, e24, _, e45]) = undirected_fixture();
    let algorithm = BellmanFordShortestPath::new(&graph);

    assert_eq!(
        algorithm.get_path(&"v1", &"v2").unwrap().unwrap().edges,
        [e12]
    );
    assert_eq!(
        algorithm.get_path(&"v1", &"v4").unwrap().unwrap().edges,
        [e12, e24]
    );
    assert_eq!(
        algorithm.get_path(&"v1", &"v5").unwrap().unwrap().edges,
        [e12, e24, e45]
    );
    assert_eq!(
        algorithm.get_path(&"v3", &"v4").unwrap().unwrap().edges,
        [e13, e12, e24]
    );
}

#[test]
fn test_undirected() {
    let (graph, [e12, e13, _, e24, _, e45]) = undirected_fixture();
    let paths = BellmanFordShortestPath::new(&graph)
        .get_paths(&"v3")
        .unwrap();

    assert_eq!(paths.path(&"v5").unwrap().edges, [e13, e12, e24, e45]);
    for (vertex, weight) in [
        ("v1", 3.0),
        ("v2", 5.0),
        ("v3", 0.0),
        ("v4", 10.0),
        ("v5", 15.0),
    ] {
        assert_eq!(paths.weight(&vertex), weight);
    }
}

#[test]
fn test_with_negative_edges() {
    let (graph, [_, e13, e15, _, e34, _]) = negative_fixture();
    let algorithm = BellmanFordShortestPath::new(&graph);

    assert_eq!(
        algorithm.get_path(&"v1", &"v4").unwrap().unwrap().edges,
        [e13, e34]
    );
    assert_eq!(
        algorithm.get_path(&"v1", &"v5").unwrap().unwrap().edges,
        [e15]
    );
}

#[test]
fn test_no_path() {
    let mut graph = DiGraph::<&str>::pseudo().weighted();
    graph.add_vertices(["a", "b"]);
    let paths = BellmanFordShortestPath::new(&graph)
        .get_paths(&"a")
        .unwrap();

    assert_eq!(paths.weight(&"b"), f64::INFINITY);
    assert!(paths.path(&"b").is_none());
}

fn wikipedia_graph(y_to_z: f64) -> DiGraph<&'static str> {
    let mut graph = DiGraph::<&str>::pseudo().weighted();
    graph.add_vertices(["w", "y", "x", "z", "s"]);
    for (source, target, weight) in [
        ("w", "z", 2.0),
        ("y", "w", 4.0),
        ("x", "w", 6.0),
        ("x", "y", 3.0),
        ("z", "x", -7.0),
        ("y", "z", y_to_z),
        ("z", "y", -3.0),
        ("s", "w", 0.0),
        ("s", "y", 0.0),
        ("s", "x", 0.0),
        ("s", "z", 0.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    graph
}

#[test]
fn test_wikipedia_example_bellman_ford() {
    let graph = wikipedia_graph(5.0);
    let paths = BellmanFordShortestPath::new(&graph)
        .get_paths(&"s")
        .unwrap();

    for (vertex, weight) in [
        ("s", 0.0),
        ("w", -1.0),
        ("y", -4.0),
        ("x", -7.0),
        ("z", 0.0),
    ] {
        assert_eq!(paths.weight(&vertex), weight);
    }
}

#[test]
fn test_negative_cycle_detection() {
    let graph = wikipedia_graph(3.0);
    let error = BellmanFordShortestPath::new(&graph)
        .get_paths(&"s")
        .unwrap_err();

    assert_eq!(error.to_string(), "graph contains a negative-weight cycle");
}

#[test]
fn test_negative_cycle_detection_actual_cycle() {
    let graph = wikipedia_graph(3.0);
    let error = BellmanFordShortestPath::new(&graph)
        .get_paths(&"s")
        .unwrap_err();
    let cycle = error.cycle().unwrap();

    assert_eq!(cycle.start_vertex(), Some(&"x"));
    assert_eq!(cycle.end_vertex(), Some(&"x"));
    assert_eq!(cycle.weight, -1.0);
    assert_eq!(cycle.length(), 3);
    assert!(cycle.is_valid(&graph));
}

fn undirected_negative_graph() -> UGraph<&'static str> {
    let mut graph = UGraph::<&str>::multi().weighted();
    graph.add_vertices(["w", "y", "x"]);
    graph.add_weighted_edge(&"w", &"y", 1.0).unwrap();
    graph.add_weighted_edge(&"y", &"x", 1.0).unwrap();
    graph.add_weighted_edge(&"y", &"x", -1.0).unwrap();
    graph
}

#[test]
fn test_negative_edge_undirected_graph() {
    let graph = undirected_negative_graph();
    assert!(
        BellmanFordShortestPath::new(&graph)
            .get_paths(&"w")
            .is_err()
    );
}

#[test]
fn test_negative_edge_undirected_graph_actual_cycle() {
    let graph = undirected_negative_graph();
    let error = BellmanFordShortestPath::new(&graph)
        .get_paths(&"w")
        .unwrap_err();
    let cycle = error.cycle().unwrap();

    assert_eq!(cycle.start_vertex(), Some(&"x"));
    assert_eq!(cycle.end_vertex(), Some(&"x"));
    assert_eq!(cycle.weight, -2.0);
    assert_eq!(cycle.length(), 2);
    assert!(cycle.is_valid(&graph));
}

#[test]
fn test_do_not_detect_non_reachable_negative_cycle() {
    let mut graph = DiGraph::<i32>::pseudo().weighted();
    graph.add_vertices(1..=7);
    for (source, target, weight) in [
        (1, 2, 1.0),
        (2, 3, 1.0),
        (3, 4, 1.0),
        (5, 4, 1.0),
        (5, 6, -1.0),
        (6, 7, -1.0),
        (7, 5, -1.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }

    assert!(BellmanFordShortestPath::new(&graph).get_paths(&1).is_ok());
}

#[test]
fn test_negative_cycle() {
    let mut graph = DiGraph::<&str>::pseudo().weighted();
    graph.add_vertices(["1", "2", "3", "4", "5", "6", "7", "8", "9", "x"]);
    for (source, target, weight) in [
        ("1", "2", 1.0),
        ("2", "3", 1.0),
        ("3", "4", 1.0),
        ("4", "5", 1.0),
        ("5", "6", 1.0),
        ("6", "7", 1.0),
        ("7", "8", 1.0),
        ("8", "9", 1.0),
        ("7", "x", -3.0),
        ("x", "4", -3.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }

    let error = BellmanFordShortestPath::new(&graph)
        .get_paths(&"1")
        .unwrap_err();
    let cycle = error.cycle().unwrap();
    assert_eq!(cycle.start_vertex(), Some(&"6"));
    assert_eq!(cycle.end_vertex(), Some(&"6"));
    assert_eq!(cycle.weight, -3.0);
    assert_eq!(cycle.length(), 5);
}

#[test]
fn test_negative_cycle_with_max_hops() {
    let mut graph = DiGraph::<i32>::pseudo().weighted();
    graph.add_vertices(1..=4);
    for (source, target, weight) in [(1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0), (4, 1, -5.0)] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }

    let paths = BellmanFordShortestPath::with_options(&graph, 1e-16, 3)
        .unwrap()
        .get_paths(&1)
        .unwrap();
    assert_eq!(paths.weight(&3), 2.0);

    let error = BellmanFordShortestPath::with_options(&graph, 1e-16, 4)
        .unwrap()
        .get_paths(&1)
        .unwrap_err();
    let cycle = error.cycle().unwrap();
    assert_eq!(cycle.start_vertex(), Some(&1));
    assert_eq!(cycle.end_vertex(), Some(&1));
    assert_eq!(cycle.weight, -2.0);
    assert_eq!(cycle.length(), 4);
}
