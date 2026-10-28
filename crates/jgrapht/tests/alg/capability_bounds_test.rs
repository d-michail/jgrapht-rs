// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
// Rust port (C) Copyright 2026, by Dimitrios Michail and jgrapht-rs Contributors.
//
// See the CONTRIBUTORS.md file distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the GNU Lesser General Public
// License v2.1 or later which is available at
// http://www.gnu.org/licenses/old-licenses/lgpl-2.1-standalone.html.
//
// SPDX-License-Identifier: EPL-2.0 OR LGPL-2.1-or-later

//! Capability bounds of the algorithms (API roadmap R1): algorithms that only follow outgoing
//! edges run on a graph without an incoming index, and algorithms that need incoming edges accept
//! the lazy and full indexes. The rejection of `NoIncoming` is covered by `compile_fail` doctests
//! on the algorithms themselves.
use jgrapht::alg::shortestpath::{
    BfsShortestPath, BidirectionalDijkstraShortestPath, Dijkstra, ShortestPathAlgorithm,
};
use jgrapht::graph::sparse::{
    FullIncoming, LazyIncoming, NoIncoming, SparseIntDirectedWeightedGraph,
};
use jgrapht::traverse::{Bfs, Dfs, DfsPostOrder, topological_sort};

const EDGES: [(usize, usize, f64); 5] = [
    (0, 1, 1.0),
    (1, 2, 2.0),
    (0, 2, 5.0),
    (2, 3, 1.0),
    (1, 3, 7.0),
];

#[test]
fn traversals_and_forward_searches_run_without_an_incoming_index() {
    let g = SparseIntDirectedWeightedGraph::<NoIncoming>::from_edges(4, &EDGES).unwrap();
    assert_eq!(Bfs::new(&g, &0).unwrap().count(), 4);
    assert_eq!(Dfs::new(&g, &0).unwrap().count(), 4);
    assert_eq!(DfsPostOrder::new(&g, &0).unwrap().count(), 4);
    assert_eq!(Dijkstra::new(&g).path_weight(&0, &3).unwrap(), Some(4.0));
    assert_eq!(
        BfsShortestPath::new(&g).path_weight(&0, &3).unwrap(),
        Some(2.0)
    );
}

#[test]
fn reverse_aware_algorithms_accept_lazy_and_full_indexes() {
    let lazy = SparseIntDirectedWeightedGraph::<LazyIncoming>::from_edges(4, &EDGES).unwrap();
    let full = SparseIntDirectedWeightedGraph::<FullIncoming>::from_edges(4, &EDGES).unwrap();
    for weight in [
        BidirectionalDijkstraShortestPath::new(&lazy).path_weight(&0, &3),
        BidirectionalDijkstraShortestPath::new(&full).path_weight(&0, &3),
    ] {
        assert_eq!(weight.unwrap(), Some(4.0));
    }
    assert_eq!(topological_sort(&lazy).unwrap().len(), 4);
    assert_eq!(topological_sort(&full).unwrap().len(), 4);
}

mod connectivity_and_partitioning {
    use jgrapht::Graph;
    use jgrapht::alg::connectivity::{
        biconnectivity, condensation, connected_components, strongly_connected_components,
    };
    use jgrapht::alg::partition::{bipartite_partition, is_bipartite};
    use jgrapht::graph::sparse::{FullIncoming, LazyIncoming, NoIncoming, SparseIntDirectedGraph};

    /// Two directed 3-cycles joined by one edge, plus an isolated vertex.
    const EDGES: [(usize, usize); 7] = [(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3), (2, 3)];

    fn sorted(mut components: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
        for c in &mut components {
            c.sort();
        }
        components.sort();
        components
    }

    #[test]
    fn component_algorithms_run_on_outgoing_only_graphs() {
        let no = SparseIntDirectedGraph::<NoIncoming>::from_edges(7, &EDGES).unwrap();
        let full = SparseIntDirectedGraph::<FullIncoming>::from_edges(7, &EDGES).unwrap();

        assert_eq!(
            sorted(strongly_connected_components(&no)),
            [vec![0, 1, 2], vec![3, 4, 5], vec![6]]
        );
        assert_eq!(
            sorted(strongly_connected_components(&no)),
            sorted(strongly_connected_components(&full))
        );
        assert_eq!(
            sorted(connected_components(&no)),
            sorted(connected_components(&full))
        );
        let (dag, members) = condensation(&no);
        assert_eq!(members.len(), 3);
        assert_eq!(dag.vertex_count(), 3);
        assert_eq!(
            biconnectivity(&no).blocks.len(),
            biconnectivity(&full).blocks.len()
        );
    }

    #[test]
    fn bipartite_partitioning_agrees_between_lazy_and_full_indexes() {
        let square = [(0, 1), (1, 2), (2, 3), (3, 0)];
        let triangle = [(0, 1), (1, 2), (2, 0)];
        let lazy = SparseIntDirectedGraph::<LazyIncoming>::from_edges(4, &square).unwrap();
        let full = SparseIntDirectedGraph::<FullIncoming>::from_edges(4, &square).unwrap();
        assert!(is_bipartite(&lazy) && is_bipartite(&full));
        assert_eq!(
            bipartite_partition(&lazy).map(|(a, b)| (sorted(vec![a]), sorted(vec![b]))),
            bipartite_partition(&full).map(|(a, b)| (sorted(vec![a]), sorted(vec![b])))
        );
        let odd = SparseIntDirectedGraph::<FullIncoming>::from_edges(3, &triangle).unwrap();
        assert!(!is_bipartite(&odd));
        assert!(bipartite_partition(&odd).is_none());
    }
}

mod scoring_coloring_clustering {
    use jgrapht::GraphError;
    use jgrapht::alg::clustering::{LabelPropagationClustering, NaiveGreedyModularityAlgorithm};
    use jgrapht::alg::color::{GreedyColoring, LargestDegreeFirstColoring};
    use jgrapht::alg::interfaces::{ClusteringAlgorithm, VertexColoringAlgorithm};
    use jgrapht::alg::scoring::{
        ClosenessCentrality, Coreness, HarmonicCentrality, PageRank, VertexScoringAlgorithm,
    };
    use jgrapht::graph::sparse::{
        FullIncoming, LazyIncoming, NoIncoming, SparseIntDirectedGraph, SparseIntUndirectedGraph,
    };
    use rand::{SeedableRng, rngs::StdRng};
    use std::collections::HashMap;

    const EDGES: [(usize, usize); 6] = [(0, 1), (1, 2), (2, 0), (2, 3), (3, 4), (4, 2)];

    fn sorted(scores: &HashMap<usize, f64>) -> Vec<(usize, u64)> {
        let mut v: Vec<_> = scores.iter().map(|(k, s)| (*k, s.to_bits())).collect();
        v.sort();
        v
    }

    #[test]
    fn outgoing_scoring_runs_without_an_incoming_index() {
        let no = SparseIntDirectedGraph::<NoIncoming>::from_edges(5, &EDGES).unwrap();
        let full = SparseIntDirectedGraph::<FullIncoming>::from_edges(5, &EDGES).unwrap();
        assert_eq!(
            sorted(ClosenessCentrality::new(&no).scores()),
            sorted(ClosenessCentrality::new(&full).scores())
        );
        assert_eq!(
            sorted(HarmonicCentrality::new(&no).scores()),
            sorted(HarmonicCentrality::new(&full).scores())
        );
    }

    #[test]
    fn incoming_scoring_agrees_between_lazy_and_full_indexes() {
        let lazy = SparseIntDirectedGraph::<LazyIncoming>::from_edges(5, &EDGES).unwrap();
        let full = SparseIntDirectedGraph::<FullIncoming>::from_edges(5, &EDGES).unwrap();
        assert_eq!(
            sorted(PageRank::new(&lazy).scores()),
            sorted(PageRank::new(&full).scores())
        );
    }

    #[test]
    fn undirected_only_algorithms_take_undirected_graphs() {
        let g = SparseIntUndirectedGraph::new(5, &EDGES).unwrap();
        let scores = Coreness::new(&g).unwrap();
        assert_eq!(scores.vertex_score(&0).unwrap(), 2);
        // a directed graph is a compile error (`Coreness::new` takes an `UndirectedGraph`)
    }

    #[test]
    fn unknown_vertices_are_reported_as_not_found() {
        let full = SparseIntDirectedGraph::<FullIncoming>::from_edges(5, &EDGES).unwrap();
        let closeness = ClosenessCentrality::new(&full);
        assert_eq!(closeness.vertex_score(&99), Err(GraphError::VertexNotFound));
    }

    #[test]
    fn coloring_agrees_between_lazy_and_full_indexes() {
        let lazy = SparseIntDirectedGraph::<LazyIncoming>::from_edges(5, &EDGES).unwrap();
        let full = SparseIntDirectedGraph::<FullIncoming>::from_edges(5, &EDGES).unwrap();
        for (a, b) in [
            (
                GreedyColoring::new(&lazy).coloring().unwrap(),
                GreedyColoring::new(&full).coloring().unwrap(),
            ),
            (
                LargestDegreeFirstColoring::new(&lazy).coloring().unwrap(),
                LargestDegreeFirstColoring::new(&full).coloring().unwrap(),
            ),
        ] {
            assert_eq!(a.count, b.count);
            assert_eq!(a.colors, b.colors);
        }
    }

    #[test]
    fn clustering_takes_undirected_graphs() {
        let g = SparseIntUndirectedGraph::new(5, &EDGES).unwrap();
        let naive = NaiveGreedyModularityAlgorithm::new(&g)
            .unwrap()
            .clustering()
            .unwrap();
        assert!(!naive.clusters().is_empty());
        let lp = LabelPropagationClustering::new(&g, 0, StdRng::seed_from_u64(7))
            .unwrap()
            .clustering()
            .unwrap();
        assert!(!lp.clusters().is_empty());
    }
}

mod cycles_isomorphism_trees {
    use jgrapht::alg::cycle::CycleDetector;
    use jgrapht::alg::interfaces::LowestCommonAncestorAlgorithm;
    use jgrapht::alg::lca::TarjanLcaFinder;
    use jgrapht::graph::sparse::{FullIncoming, NoIncoming, SparseIntDirectedGraph};

    /// `0 -> 1 -> 2 -> 0` is a cycle, `3 -> 4` hangs off vertex 2.
    const CYCLIC: [(usize, usize); 4] = [(0, 1), (1, 2), (2, 0), (2, 3)];
    /// A tree rooted at 0.
    const TREE: [(usize, usize); 4] = [(0, 1), (0, 2), (1, 3), (1, 4)];

    #[test]
    fn cycle_detection_runs_without_an_incoming_index() {
        let no = SparseIntDirectedGraph::<NoIncoming>::from_edges(4, &CYCLIC).unwrap();
        let full = SparseIntDirectedGraph::<FullIncoming>::from_edges(4, &CYCLIC).unwrap();
        assert!(CycleDetector::new(&no).detect_cycles());
        assert_eq!(
            CycleDetector::new(&no).find_cycles(),
            CycleDetector::new(&full).find_cycles()
        );
        let acyclic = SparseIntDirectedGraph::<NoIncoming>::from_edges(4, &TREE[..3]).unwrap();
        assert!(!CycleDetector::new(&acyclic).detect_cycles());
    }

    #[test]
    fn tarjan_lca_runs_without_an_incoming_index() {
        let no = SparseIntDirectedGraph::<NoIncoming>::from_edges(5, &TREE).unwrap();
        let full = SparseIntDirectedGraph::<FullIncoming>::from_edges(5, &TREE).unwrap();
        let on_no = TarjanLcaFinder::new(&no, 0usize)
            .unwrap()
            .lca(&3, &4)
            .unwrap();
        let on_full = TarjanLcaFinder::new(&full, 0usize)
            .unwrap()
            .lca(&3, &4)
            .unwrap();
        assert_eq!(on_no, Some(1));
        assert_eq!(on_no, on_full);
    }
}

mod flow_matching_tours {
    use jgrapht::alg::flow::{
        BoykovKolmogorov, Dinic, EdmondsKarp, MaximumFlowValueAlgorithm, PushRelabel,
    };
    use jgrapht::alg::matching::{GreedyMaximumCardinalityMatching, MatchingAlgorithm};
    use jgrapht::graph::sparse::{
        FullIncoming, LazyIncoming, NoIncoming, SparseIntDirectedWeightedGraph,
        SparseIntUndirectedGraph,
    };

    /// 0 -> 1 -> 3, 0 -> 2 -> 3 and a cross edge 1 -> 2; the maximum flow is 5.
    const NETWORK: [(usize, usize, f64); 5] = [
        (0, 1, 3.0),
        (0, 2, 2.0),
        (1, 3, 2.0),
        (2, 3, 3.0),
        (1, 2, 1.0),
    ];

    #[test]
    fn augmenting_path_flows_run_without_an_incoming_index() {
        let no = SparseIntDirectedWeightedGraph::<NoIncoming>::from_edges(4, &NETWORK).unwrap();
        let full = SparseIntDirectedWeightedGraph::<FullIncoming>::from_edges(4, &NETWORK).unwrap();
        let on_no = [
            Dinic::new(&no).unwrap().maximum_flow_value(&0, &3).unwrap(),
            EdmondsKarp::new(&no)
                .unwrap()
                .maximum_flow_value(&0, &3)
                .unwrap(),
            BoykovKolmogorov::new(&no)
                .unwrap()
                .maximum_flow_value(&0, &3)
                .unwrap(),
        ];
        let on_full = [
            Dinic::new(&full)
                .unwrap()
                .maximum_flow_value(&0, &3)
                .unwrap(),
            EdmondsKarp::new(&full)
                .unwrap()
                .maximum_flow_value(&0, &3)
                .unwrap(),
            BoykovKolmogorov::new(&full)
                .unwrap()
                .maximum_flow_value(&0, &3)
                .unwrap(),
        ];
        assert_eq!(on_no, [5.0; 3]);
        assert_eq!(on_no, on_full);
    }

    #[test]
    fn push_relabel_needs_an_incoming_index() {
        let lazy = SparseIntDirectedWeightedGraph::<LazyIncoming>::from_edges(4, &NETWORK).unwrap();
        let full = SparseIntDirectedWeightedGraph::<FullIncoming>::from_edges(4, &NETWORK).unwrap();
        for value in [
            PushRelabel::new(&lazy)
                .unwrap()
                .maximum_flow_value(&0, &3)
                .unwrap(),
            PushRelabel::new(&full)
                .unwrap()
                .maximum_flow_value(&0, &3)
                .unwrap(),
        ] {
            assert_eq!(value, 5.0);
        }
    }

    #[test]
    fn undirected_only_matching_takes_sparse_undirected_graphs() {
        let g = SparseIntUndirectedGraph::new(4, &[(0, 1), (1, 2), (2, 3)]).unwrap();
        let matching = GreedyMaximumCardinalityMatching::new(&g, false)
            .unwrap()
            .matching()
            .unwrap();
        assert_eq!(matching.len(), 2);
        // a directed graph is a compile error: the matchers take an `UndirectedGraph`
    }
}

mod path_enumeration {
    use jgrapht::alg::shortestpath::YenKShortestPath;
    use jgrapht::graph::sparse::{FullIncoming, NoIncoming, SparseIntDirectedWeightedGraph};

    /// Three routes from 0 to 3 with lengths 3, 4 and 6.
    const EDGES: [(usize, usize, f64); 5] = [
        (0, 1, 1.0),
        (1, 3, 2.0),
        (0, 2, 2.0),
        (2, 3, 2.0),
        (0, 3, 6.0),
    ];

    fn lengths<V, E>(paths: &[jgrapht::GraphWalk<V, E>]) -> Vec<f64> {
        paths.iter().map(|p| p.weight()).collect()
    }

    #[test]
    fn yen_runs_without_an_incoming_index() {
        let no = SparseIntDirectedWeightedGraph::<NoIncoming>::from_edges(4, &EDGES).unwrap();
        let full = SparseIntDirectedWeightedGraph::<FullIncoming>::from_edges(4, &EDGES).unwrap();
        let on_no = YenKShortestPath::new(&no).paths(&0, &3, 5).unwrap();
        let on_full = YenKShortestPath::new(&full).paths(&0, &3, 5).unwrap();
        assert_eq!(lengths(&on_no), [3.0, 4.0, 6.0]);
        assert_eq!(lengths(&on_no), lengths(&on_full));
    }
}
