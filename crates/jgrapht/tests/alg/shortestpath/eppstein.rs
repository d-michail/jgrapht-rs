use jgrapht::alg::shortestpath::{EppsteinKShortestPath, EppsteinShortestPathIterator};
use jgrapht::{DiGraph, GraphMut, GraphMutExt, UGraph};

fn graph(edges: &[(u32, u32, f64)]) -> DiGraph<u32> {
    let mut graph = DiGraph::<u32>::simple().weighted();
    for &(source, target, weight) in edges {
        graph.add_vertex(source);
        graph.add_vertex(target);
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    graph
}

fn weights(
    iterator: impl Iterator<Item = jgrapht::GraphWalk<u32, jgrapht::DefaultEdge>>,
    count: usize,
) -> Vec<f64> {
    iterator.take(count).map(|path| path.weight).collect()
}

#[test]
fn validates_graph_and_vertices() {
    let directed = graph(&[(1, 2, 1.0)]);
    assert!(EppsteinShortestPathIterator::new(&directed, &0, &2).is_err());
    assert!(EppsteinShortestPathIterator::new(&directed, &1, &3).is_err());

    let mut undirected = UGraph::<u32>::simple().weighted();
    undirected.add_vertex(1);
    undirected.add_vertex(2);
    undirected.add_weighted_edge(&1, &2, 1.0).unwrap();
    assert!(EppsteinShortestPathIterator::new(&undirected, &1, &2).is_err());

    let mut pseudograph = DiGraph::<u32>::pseudo().weighted();
    pseudograph.add_vertex(1);
    assert!(EppsteinShortestPathIterator::new(&pseudograph, &1, &1).is_err());
}

#[test]
fn empty_singleton_and_unique_path_cases() {
    let mut disconnected = DiGraph::<u32>::simple().weighted();
    disconnected.add_vertex(1);
    disconnected.add_vertex(2);
    assert_eq!(
        EppsteinShortestPathIterator::new(&disconnected, &1, &2)
            .unwrap()
            .next(),
        None
    );

    let singleton = EppsteinShortestPathIterator::new(&disconnected, &1, &1)
        .unwrap()
        .next()
        .unwrap();
    assert_eq!(singleton.vertices, [1]);
    assert_eq!(singleton.weight, 0.0);

    let chain = graph(&[(1, 2, 1.0), (2, 3, 1.0)]);
    let mut iterator = EppsteinShortestPathIterator::new(&chain, &1, &3).unwrap();
    let path = iterator.next().unwrap();
    assert_eq!(path.vertices, [1, 2, 3]);
    assert_eq!(path.weight, 2.0);
    assert!(path.is_valid(&chain));
    assert_eq!(iterator.next(), None);
}

#[test]
fn java_simple_graph_golden_weights() {
    let g = graph(&[
        (1, 2, 2.0),
        (2, 3, 20.0),
        (3, 4, 14.0),
        (1, 5, 13.0),
        (2, 6, 27.0),
        (3, 7, 14.0),
        (4, 8, 15.0),
        (5, 6, 9.0),
        (6, 7, 10.0),
        (7, 8, 25.0),
        (5, 9, 15.0),
        (6, 10, 20.0),
        (7, 11, 12.0),
        (8, 12, 7.0),
        (9, 10, 18.0),
        (10, 11, 8.0),
        (11, 12, 11.0),
    ]);
    let iterator = EppsteinShortestPathIterator::new(&g, &1, &12).unwrap();
    assert_eq!(
        weights(iterator, 20),
        [55.0, 58.0, 59.0, 61.0, 62.0, 64.0, 65.0, 68.0, 68.0, 71.0]
    );
}

#[test]
fn java_cycle_golden_weights_and_valid_walks() {
    let two_cycle = graph(&[(1, 2, 1.0), (2, 1, 1.0)]);
    let mut iterator = EppsteinShortestPathIterator::new(&two_cycle, &1, &2).unwrap();
    for expected in [1.0, 3.0, 5.0, 7.0, 9.0] {
        let path = iterator.next().unwrap();
        assert_eq!(path.weight, expected);
        assert!(path.is_valid(&two_cycle));
    }

    let graph = graph(&[
        (1, 2, 1.0),
        (2, 3, 1.0),
        (3, 4, 1.0),
        (4, 1, 1.0),
        (1, 5, 2.0),
        (5, 6, 2.0),
        (6, 7, 2.0),
        (7, 1, 2.0),
        (3, 6, 2.0),
        (6, 3, 2.0),
    ]);
    assert_eq!(
        weights(
            EppsteinShortestPathIterator::new(&graph, &1, &6).unwrap(),
            18
        ),
        [
            4.0, 4.0, 8.0, 8.0, 8.0, 8.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0,
            12.0, 12.0, 12.0,
        ]
    );
}

#[test]
fn java_rest_heap_and_k_shortest_golden_weights() {
    let g = graph(&[
        (1, 2, 2.0),
        (1, 3, 3.0),
        (1, 4, 4.0),
        (1, 5, 5.0),
        (1, 6, 6.0),
        (1, 7, 7.0),
        (1, 8, 8.0),
        (1, 9, 9.0),
        (2, 10, 1.0),
        (3, 10, 1.0),
        (4, 10, 1.0),
        (5, 10, 1.0),
        (6, 10, 1.0),
        (7, 10, 1.0),
        (8, 10, 1.0),
        (9, 10, 1.0),
    ]);
    assert_eq!(
        weights(EppsteinShortestPathIterator::new(&g, &1, &10).unwrap(), 20),
        [3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
    );

    let cyclic = graph(&[
        (1, 2, 1.0),
        (2, 3, 1.0),
        (3, 4, 1.0),
        (4, 3, 1.0),
        (4, 5, 1.0),
        (5, 4, 1.0),
    ]);
    let paths = EppsteinKShortestPath::new(&cyclic)
        .paths(&1, &3, 6)
        .unwrap();
    assert_eq!(
        paths.iter().map(|path| path.weight).collect::<Vec<_>>(),
        [2.0, 4.0, 6.0, 6.0, 8.0, 8.0]
    );
}
