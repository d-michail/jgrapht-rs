use super::base::{HamiltonianCycleAlgorithm, check_graph, singleton_tour, vertex_list_to_tour};
use crate::alg::cycle::{EulerianCycleAlgorithm, HierholzerEulerianCycle};
use crate::alg::matching::blossom::KolmogorovWeightedPerfectMatching;
use crate::alg::matching::MatchingAlgorithm;
use crate::alg::spanning::{
    KruskalMinimumSpanningTree, SpanningTreeAlgorithm,
};
use crate::graph::{Graph, GraphExt, GraphMut, GraphMutExt, UGraph};
use crate::view::AsSubgraph;
use crate::walk::GraphWalk;
use crate::Result;
use std::collections::HashSet;

/// Christofides' 3/2-approximation algorithm for metric travelling-salesman instances.
///
/// The input must be a non-empty, complete, undirected graph. As in JGraphT, non-negative edge
/// weights and the triangle inequality are part of the caller's metric-instance contract.
#[derive(Default)]
pub struct ChristofidesThreeHalvesApproxMetricTSP;

impl ChristofidesThreeHalvesApproxMetricTSP {
    pub fn new() -> Self {
        Self
    }
}

impl<G: Graph> HamiltonianCycleAlgorithm<G> for ChristofidesThreeHalvesApproxMetricTSP {
    fn get_tour(&mut self, graph: &G) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        check_graph(graph)?;
        if graph.vertex_count() == 1 {
            return Ok(Some(singleton_tour(graph)));
        }

        let mut mst_and_matching = UGraph::<G::Vertex>::pseudo();
        mst_and_matching.add_vertices(graph.vertices().cloned());

        let spanning_tree = KruskalMinimumSpanningTree::new(graph).spanning_tree();
        for edge in spanning_tree.edges {
            let (source, target) = graph.edge_endpoints(&edge).expect("edge of the graph");
            mst_and_matching.add_edge(source, target)?;
        }

        let mut odd_degree_vertices = Vec::new();
        for vertex in mst_and_matching.vertices() {
            if mst_and_matching.degree_of(vertex)? % 2 == 1 {
                odd_degree_vertices.push(vertex.clone());
            }
        }
        let subgraph = AsSubgraph::induced(graph, odd_degree_vertices);
        let mut matching = KolmogorovWeightedPerfectMatching::new(&subgraph)?;
        for edge in matching.get_matching()?.edges() {
            let (source, target) = graph.edge_endpoints(edge).expect("edge of the graph");
            mst_and_matching.add_edge(source, target)?;
        }

        let eulerian_cycle =
            HierholzerEulerianCycle::new().get_eulerian_cycle(&mst_and_matching)?;
        let mut visited = HashSet::with_capacity(graph.vertex_count());
        let tour_vertices = eulerian_cycle
            .vertex_list()
            .iter()
            .filter(|vertex| visited.insert((*vertex).clone()))
            .cloned()
            .collect();
        vertex_list_to_tour(tour_vertices, graph).map(Some)
    }
}
