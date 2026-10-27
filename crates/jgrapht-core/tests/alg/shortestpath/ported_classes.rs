use super::*;
use dashu_int::UBig;
use std::collections::HashMap;

#[test]
fn all_directed_paths_matches_java_edge_cases() {
    let mut graph = DiGraph::<u32>::pseudo().weighted();
    graph.add_vertex(0);
    graph.add_vertex(1);
    let loop_zero = graph.add_edge(&0, &0).unwrap().unwrap();
    let to_one = graph.add_edge(&0, &1).unwrap().unwrap();
    let loop_one = graph.add_edge(&1, &1).unwrap().unwrap();
    graph.set_edge_weight(&loop_zero, 1.0).unwrap();
    graph.set_edge_weight(&to_one, 2.0).unwrap();
    graph.set_edge_weight(&loop_one, 3.0).unwrap();

    let algorithm = AllDirectedPaths::new(&graph);
    let walks = algorithm.get_all_paths(&0, &1, false, Some(3)).unwrap();
    assert_eq!(walks.len(), 6);
    assert!(walks.iter().all(|path| path.length() <= 3));
    let zero = algorithm
        .get_all_paths_between([&0, &1], [&0, &1], false, Some(0))
        .unwrap();
    assert_eq!(zero.len(), 2);
    assert!(zero.iter().all(|path| path.length() == 0));
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
