// (C) Copyright 2003-2026, by John V Sichi and JGraphT Contributors.
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
/// A $3/2$-approximation algorithm for the metric TSP problem.
///
/// The [travelling salesman problem](https://en.wikipedia.org/wiki/Travelling_salesman_problem) (TSP) asks the following question: "Given a list of cities and the distances between
/// each pair of cities, what is the shortest possible route that visits each city exactly once and
/// returns to the origin city?". In the metric TSP, the intercity distances satisfy the triangle
/// inequality.
///
/// This is an implementation of the [Christofides algorithm](https://en.wikipedia.org/wiki/Christofides_algorithm). The algorithms is a $3/2$-approximation assuming that the input graph
/// satisfies triangle inequality and all edge weights are nonnegative. The implementation requires
/// the input graph to be *undirected* and *complete*. The worst case running time
/// complexity is $\mathcal{O}(V^3E)$.
///
/// The algorithm performs following steps to compute the resulting tour:
///
/// - Compute a minimum spanning tree in the input graph.
/// - Find vertices with odd degree in the MST.
/// - Compute minimum weight perfect matching in the induced subgraph on odd degree vertices.
/// - Add edges from the minimum weight perfect matching to the MST (forming a pseudograph).
/// - Compute an Eulerian cycle in the obtained pseudograph and form a closed tour in this
/// cycle.
///
/// The following two observations yield the $3/2$ approximation bound:
///
/// - The cost of every minimum spanning tree is less than or equal to the cost of every
/// Hamiltonian cycle since after one edge removal every Hamiltonian cycle becomes a spanning
/// tree
/// - Twice the cost of a perfect matching in a graph is less than or equal to the cost of every
/// Hamiltonian cycle. This follows from the fact that after forming a closed tour using the edges of
/// a perfect matching the cost of the edges not from the matching is greater than or equal to the
/// cost of the matching edges.
///
/// For more details, see *Christofides, N.: Worst-case analysis of a new heuristic for the
/// travelling salesman problem. Graduate School of Industrial Administration, Carnegie Mellon
/// University (1976).*
///
/// # Rust notes
///
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
