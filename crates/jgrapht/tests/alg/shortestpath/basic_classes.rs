use jgrapht::alg::shortestpath::*;
use jgrapht::*;

fn weighted_directed(edges: &[(u32, u32, f64)]) -> DiGraph<u32> {
    let mut graph = DiGraph::<u32>::simple().weighted();
    for &(source, target, weight) in edges {
        graph.add_vertex(source);
        graph.add_vertex(target);
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    graph
}

#[test]
fn dijkstra_closest_first_order_and_radius() {
    let graph = weighted_directed(&[
        (3, 1, 3.0),
        (3, 2, 5.0),
        (1, 2, 3.0),
        (1, 4, 10.0),
        (2, 4, 5.0),
        (4, 5, 3.0),
    ]);
    let mut iterator = DijkstraClosestFirstIterator::new(&graph, &3).unwrap();
    assert_eq!(iterator.by_ref().collect::<Vec<_>>(), [3, 1, 2, 4, 5]);
    assert_eq!(iterator.paths().weight(&4), 10.0);

    let bounded = DijkstraClosestFirstIterator::with_radius(&graph, &3, 10.0)
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(bounded, [3, 1, 2, 4]);
}

#[test]
fn java_class_names_share_the_shortest_path_contract() {
    let graph = weighted_directed(&[(1, 2, 2.0), (1, 3, 1.0), (3, 2, 1.0), (2, 4, 1.0)]);
    assert_eq!(
        DijkstraShortestPath::new(&graph)
            .path_weight(&1, &4)
            .unwrap(),
        3.0
    );
    assert_eq!(
        BellmanFordShortestPath::new(&graph)
            .path_weight(&1, &4)
            .unwrap(),
        3.0
    );
}

#[test]
fn floyd_warshall_hops_and_path_count() {
    let graph = weighted_directed(&[(1, 2, -5.0), (2, 3, -5.0), (3, 4, -100.0)]);
    let paths = FloydWarshallShortestPaths::new(&graph).unwrap();
    assert_eq!(paths.shortest_paths_count(), 6);
    assert_eq!(paths.first_hop(&1, &4), Some(2));
    assert_eq!(paths.last_hop(&1, &4), Some(3));
    assert_eq!(paths.path_weight(&1, &4).unwrap(), -110.0);
    assert_eq!(paths.first_hop(&1, &1), None);
}

#[test]
fn johnson_class_handles_negative_edges() {
    let graph = weighted_directed(&[(1, 2, 4.0), (1, 3, 5.0), (3, 2, -3.0)]);
    let paths = JohnsonShortestPaths::new(&graph).unwrap();
    assert_eq!(paths.path_weight(&1, &2).unwrap(), 2.0);
    assert_eq!(paths.path(&1, &2).unwrap().unwrap().vertices, [1, 3, 2]);
}

#[test]
fn alt_is_admissible_on_directed_graph() {
    let graph = weighted_directed(&[(1, 2, 2.0), (2, 3, 2.0), (1, 3, 7.0), (3, 4, 1.0)]);
    let heuristic = ALTAdmissibleHeuristic::new(&graph, [1, 4]).unwrap();
    let dijkstra = Dijkstra::new(&graph);
    for source in graph.vertices() {
        for target in graph.vertices() {
            let estimate = heuristic.cost_estimate(source, target);
            let distance = dijkstra.path_weight(source, target).unwrap();
            assert!(estimate >= 0.0);
            assert!(estimate <= distance || distance.is_infinite());
        }
    }
    assert!(heuristic.is_consistent());

    let bidirectional = BidirectionalAStarShortestPath::new(&graph, |source, target| {
        heuristic.cost_estimate(source, target)
    });
    for source in graph.vertices() {
        for target in graph.vertices() {
            assert_eq!(
                bidirectional.path_weight(source, target).unwrap(),
                dijkstra.path_weight(source, target).unwrap()
            );
        }
    }
}

#[test]
fn tree_center_has_one_or_two_vertices() {
    let mut tree = UGraph::<u32>::simple();
    for vertex in 1..=6 {
        tree.add_vertex(vertex);
    }
    for (source, target) in [(1, 2), (2, 3), (3, 4), (4, 5), (3, 6)] {
        tree.add_edge(&source, &target).unwrap();
    }
    assert_eq!(TreeMeasurer::new(&tree).graph_center().unwrap(), [3]);

    tree.add_vertex(7);
    tree.add_edge(&5, &7).unwrap();
    assert_eq!(TreeMeasurer::new(&tree).graph_center().unwrap(), [4, 3]);
}

#[test]
fn bidirectional_class_radius_and_astar() {
    let graph = weighted_directed(&[(1, 2, 2.0), (2, 3, 3.0), (1, 3, 9.0)]);
    assert!(
        BidirectionalDijkstraShortestPath::with_radius(&graph, 4.0)
            .unwrap()
            .path(&1, &3)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        BidirectionalAStarShortestPath::new(&graph, |_: &u32, _: &u32| 0.0)
            .path_weight(&1, &3)
            .unwrap(),
        5.0
    );
}
