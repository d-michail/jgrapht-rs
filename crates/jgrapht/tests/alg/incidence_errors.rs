use jgrapht::alg::cycle::{
    CycleBasisAlgorithm, QueueBFSFundamentalCycleBasis, StackBFSFundamentalCycleBasis,
};
use jgrapht::alg::matching::{
    GreedyWeightedMatching, MatchingAlgorithm, PathGrowingWeightedMatching,
};
use jgrapht::alg::scoring::{EigenvectorCentrality, KatzCentrality, PageRank};
use jgrapht::alg::shortestpath::{AllDirectedPaths, DagAllPathsCounter};
use jgrapht::graph::sparse::{IncomingEdgesSupport, SparseIntDirectedGraph};
use jgrapht::graph::{DirectedExt, GraphExt};
use jgrapht::{GraphError, Result};

fn no_incoming_graph() -> SparseIntDirectedGraph {
    SparseIntDirectedGraph::with_incoming_support(
        4,
        &[(0, 1), (0, 2), (1, 2), (2, 3)],
        IncomingEdgesSupport::NoIncomingEdges,
    )
    .unwrap()
}

fn assert_unsupported<T>(result: Result<T>) {
    assert!(matches!(result, Err(GraphError::Unsupported(_))));
}

#[test]
fn fallible_graph_helpers_preserve_incidence_errors() {
    let graph = no_incoming_graph();

    assert_eq!(graph.try_successor_list_of(&0).unwrap(), [1, 2]);
    assert_eq!(
        graph.try_descendants_of(&0).unwrap(),
        [1, 2, 3].into_iter().collect()
    );
    assert!(graph.try_vertex_has_successors(&0).unwrap());

    assert_unsupported(graph.try_neighbor_list_of(&0));
    assert_unsupported(graph.try_neighbor_set_of(&0));
    assert_unsupported(graph.try_predecessor_list_of(&2));
    assert_unsupported(graph.try_ancestors_of(&3));
    assert_unsupported(graph.try_vertex_has_predecessors(&2));
}

#[test]
fn scoring_algorithms_return_missing_incoming_index_errors() {
    let graph = no_incoming_graph();

    assert_unsupported(EigenvectorCentrality::with_options(&graph, 100, 0.0001));
    assert_unsupported(KatzCentrality::with_damping(&graph, 0.01));
    assert_unsupported(PageRank::with_options(&graph, 0.85, 100, 0.0001));
}

#[test]
fn cycle_basis_algorithms_return_incidence_errors() {
    let graph = no_incoming_graph();

    let mut queue = QueueBFSFundamentalCycleBasis::new(&graph).unwrap();
    assert_unsupported(queue.get_cycle_basis());

    let mut stack = StackBFSFundamentalCycleBasis::new(&graph).unwrap();
    assert_unsupported(stack.get_cycle_basis());
}

#[test]
fn all_directed_paths_returns_missing_incoming_index_errors() {
    let graph = no_incoming_graph();

    assert_unsupported(AllDirectedPaths::new(&graph).get_all_paths(&0, &3, true, None));
    assert_unsupported(DagAllPathsCounter::count_all_paths(&graph, &0, &3));
}

#[test]
fn matching_algorithms_return_incidence_errors() {
    let graph = no_incoming_graph();

    let mut greedy = GreedyWeightedMatching::new(&graph, true).unwrap();
    assert_unsupported(greedy.get_matching());

    let mut path_growing = PathGrowingWeightedMatching::new(&graph).unwrap();
    assert_unsupported(path_growing.get_matching());
}
