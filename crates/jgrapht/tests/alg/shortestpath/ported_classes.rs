use super::*;
use dashu_int::UBig;
use std::collections::HashMap;

#[test]
fn bellman_ford_reports_the_actual_negative_cycle() {
    let mut graph = DiGraph::<&str>::pseudo().weighted();
    graph.add_vertices(["w", "y", "x", "z", "s"]);
    for (source, target, weight) in [
        ("w", "z", 2.0),
        ("y", "w", 4.0),
        ("x", "w", 6.0),
        ("x", "y", 3.0),
        ("z", "x", -7.0),
        ("y", "z", 3.0),
        ("z", "y", -3.0),
        ("s", "w", 0.0),
        ("s", "y", 0.0),
        ("s", "x", 0.0),
        ("s", "z", 0.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }

    let error = BellmanFord::new(&graph).get_paths(&"s").unwrap_err();
    assert_eq!(error.to_string(), "graph contains a negative-weight cycle");
    let cycle = error.cycle().unwrap();
    assert_eq!(cycle.start_vertex(), Some(&"x"));
    assert_eq!(cycle.end_vertex(), Some(&"x"));
    assert_eq!(cycle.weight, -1.0);
    assert_eq!(cycle.length(), 3);
    assert!(cycle.is_valid(&graph));
}

#[test]
fn bellman_ford_reports_an_undirected_negative_edge_as_a_cycle() {
    let mut graph = UGraph::<&str>::multi().weighted();
    graph.add_vertices(["w", "y", "x"]);
    graph.add_weighted_edge(&"w", &"y", 1.0).unwrap();
    graph.add_weighted_edge(&"y", &"x", 1.0).unwrap();
    graph.add_weighted_edge(&"y", &"x", -1.0).unwrap();

    let error = BellmanFord::new(&graph).get_paths(&"w").unwrap_err();
    let cycle = error.cycle().unwrap();
    assert_eq!(cycle.start_vertex(), Some(&"x"));
    assert_eq!(cycle.end_vertex(), Some(&"x"));
    assert_eq!(cycle.weight, -2.0);
    assert_eq!(cycle.length(), 2);
    assert!(cycle.is_valid(&graph));
}

#[test]
fn bellman_ford_ignores_unreachable_negative_cycles() {
    let graph = weighted_di(&[
        (1, 2, 1.0),
        (2, 3, 1.0),
        (3, 4, 1.0),
        (5, 4, 1.0),
        (5, 6, -1.0),
        (6, 7, -1.0),
        (7, 5, -1.0),
    ]);
    let paths = BellmanFord::new(&graph).get_paths(&1).unwrap();
    assert_eq!(paths.weight(&4), 3.0);
    assert_eq!(paths.weight(&5), f64::INFINITY);
}

#[test]
fn bellman_ford_max_hops_can_disable_cycle_detection() {
    let graph = weighted_di(&[(1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0), (4, 1, -5.0)]);
    let paths = BellmanFord::with_options(&graph, 1e-16, 3)
        .unwrap()
        .get_paths(&1)
        .unwrap();
    assert_eq!(paths.weight(&3), 2.0);

    let error = BellmanFord::with_options(&graph, 1e-16, 4)
        .unwrap()
        .get_paths(&1)
        .unwrap_err();
    let cycle = error.cycle().unwrap();
    assert_eq!(cycle.weight, -2.0);
    assert_eq!(cycle.length(), 4);
}

#[test]
fn suurballe_and_bhandari_find_two_disjoint_paths() {
    let graph = weighted_di(&[
        (1, 2, 1.0),
        (2, 4, 1.0),
        (1, 3, 2.0),
        (3, 4, 2.0),
        (2, 3, 10.0),
    ]);
    let suurballe = SuurballeKDisjointShortestPaths::new(&graph)
        .get_paths(&1, &4, 3)
        .unwrap();
    assert_eq!(
        suurballe.iter().map(|path| path.weight).collect::<Vec<_>>(),
        [2.0, 4.0]
    );
    let bhandari = BhandariKDisjointShortestPaths::new(&graph)
        .get_paths(&1, &4, 3)
        .unwrap();
    assert_eq!(
        bhandari.iter().map(|path| path.weight).collect::<Vec<_>>(),
        [2.0, 4.0]
    );
}

#[test]
fn yen_iterator_and_class_are_stably_sorted() {
    let graph = weighted_di(&[
        (1, 2, 1.0),
        (2, 4, 2.0),
        (1, 3, 2.0),
        (3, 4, 1.0),
        (1, 4, 10.0),
    ]);
    let paths = YenKShortestPath::new(&graph).get_paths(&1, &4, 10).unwrap();
    assert_eq!(
        paths.iter().map(|path| path.weight).collect::<Vec<_>>(),
        [3.0, 3.0, 10.0]
    );
    let iterator = YenShortestPathIterator::new(&graph, &1, &4).unwrap();
    assert_eq!(
        iterator.map(|path| path.weight).collect::<Vec<_>>(),
        [3.0, 3.0, 10.0]
    );
}

#[test]
fn dijkstra_many_to_many_handles_reverse_search() {
    let graph = weighted_di(&[(0, 2, 1.0), (1, 2, 2.0), (2, 3, 4.0)]);
    let paths = DijkstraManyToManyShortestPaths::new(&graph)
        .get_many_to_many_paths([0, 1], [3])
        .unwrap();
    assert_eq!(paths.weight(&0, &3), 5.0);
    assert_eq!(paths.weight(&1, &3), 6.0);
    assert_eq!(paths.path(&1, &3).unwrap().vertices, [1, 2, 3]);
}

#[test]
fn multi_objective_algorithms_return_the_pareto_front() {
    let mut graph = DiGraph::<u32>::multi().weighted();
    for vertex in 1..=3 {
        graph.add_vertex(vertex);
    }
    let direct = graph.add_edge(&1, &2).unwrap().unwrap();
    let first = graph.add_edge(&1, &3).unwrap().unwrap();
    let second = graph.add_edge(&3, &2).unwrap().unwrap();
    let values = HashMap::from([
        (direct, vec![5.0, 2.0]),
        (first, vec![2.0, 8.0]),
        (second, vec![1.0, 4.0]),
    ]);
    let martin = MartinShortestPath::new(&graph, |edge| values[edge].clone())
        .get_paths(&1)
        .unwrap();
    let correcting =
        LabelCorrectingMultiObjectiveShortestPath::new(&graph, |edge| values[edge].clone())
            .get_paths(&1)
            .unwrap();
    assert_eq!(martin.paths(&2).len(), 2);
    assert_eq!(correcting.paths(&2).len(), 2);
}

#[test]
fn dag_counter_uses_arbitrary_precision() {
    let mut graph = DiGraph::<u32>::simple();
    for vertex in 0..6 {
        graph.add_vertex(vertex);
    }
    for left in [1, 2] {
        graph.add_edge(&0, &left).unwrap();
        for right in [3, 4] {
            graph.add_edge(&left, &right).unwrap();
        }
    }
    graph.add_edge(&3, &5).unwrap();
    graph.add_edge(&4, &5).unwrap();
    assert_eq!(
        DagAllPathsCounter::count_all_paths(&graph, &0, &5).unwrap(),
        UBig::from(4u8)
    );
}

#[test]
fn integer_dijkstra_supports_dense_and_remapped_ids() {
    let graph = weighted_di(&[(100, 1, 2.0), (100, 2, 3.0), (1, 3, 5.0), (3, 4, 5.0)]);
    let paths = IntVertexDijkstraShortestPath::new(&graph)
        .single_source(&100)
        .unwrap();
    assert_eq!(paths.weight(&4), 12.0);
}
