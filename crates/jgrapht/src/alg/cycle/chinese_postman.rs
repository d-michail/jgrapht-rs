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

use super::{EulerianCycleAlgorithm, HierholzerEulerianCycle};
use crate::alg::matching::blossom::KolmogorovWeightedPerfectMatching;
use crate::alg::matching::{KuhnMunkresMinimalWeightBipartitePerfectMatching, MatchingAlgorithm};
use crate::alg::shortestpath::{DijkstraShortestPath, ShortestPathAlgorithm};
use crate::graph::IncidenceGraph;
use crate::graph::{
    BaseGraph, Directed, Direction, Graph, GraphExt, GraphMut, GraphMutExt, Undirected,
};
use crate::walk::GraphWalk;
use crate::{GraphError, GraphType, Result};
use indexmap::IndexSet;

#[derive(Clone)]
enum AugmentedEdge<V, E> {
    Original(E),
    Shortcut(GraphWalk<V, E>),
}

/// This class solves the Chinese Postman Problem (CPP), also known as the Route Inspection Problem.
/// The CPP asks to find a *closed walk* of minimum length that visits every edge of the graph
/// at least once. In weighted graphs, the *length* of the closed walk is defined as the sum of
/// its edge weights; in unweighted graphs, a closed walk with the least number of edges is returned
/// (the same result can be obtained for weighted graphs with uniform edge weights).
///
/// The algorithm works with directed and undirected graphs which may contain loops and/or multiple
/// edges. The runtime complexity is O(N^3) where N is the number of vertices in the graph. Mixed
/// graphs are currently not supported, as solving the CPP for mixed graphs is NP-hard. The graph on
/// which this algorithm is invoked must be strongly connected; invoking this algorithm on a graph
/// which is not strongly connected may result in undefined behavior. In case of weighted graphs, all
/// edge weights must be positive.
///
/// If the input graph is Eulerian (see `GraphTests#isEulerian(Graph)` for details) use
/// `HierholzerEulerianCycle` instead.
///
/// The implementation is based on the following paper:
/// Edmonds, J., Johnson, E.L. Matching, Euler tours and the Chinese postman, Mathematical
/// Programming (1973) 5: 88. doi:10.1007/BF01580113
///
/// More concise descriptions of the algorithms can be found here:
///
/// - [CPP for Undirected graphs](http://www.suffolkmaths.co.uk/pages/Maths%20Projects/Projects/Topology%20and%20Graph%20Theory/Chinese%20Postman%20Problem.pdf)
/// - [CPP for Directed graphs](https://www-m9.ma.tum.de/graph-algorithms/directed-chinese-postman/index_en.html)
///
/// # Rust notes
///
/// Edmonds and Johnson's solution to the directed and undirected Chinese Postman Problem.
///
/// The input graph must be strongly connected. Weighted inputs must have non-negative weights.
#[derive(Default)]
pub struct ChinesePostman;

impl ChinesePostman {
    /// Constructs a new Chinese Postman solver.
    pub fn new() -> Self {
        Self
    }

    /// Returns a minimum-weight closed walk which visits every edge at least once.
    pub fn get_cpp_solution<G: IncidenceGraph>(
        &mut self,
        graph: &G,
    ) -> Result<GraphWalk<G::Vertex, G::Edge>> {
        if graph.graph_type().mixed {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::NotMixed));
        }
        if graph.vertex_count() == 0 || graph.edge_count() == 0 {
            return HierholzerEulerianCycle::new().get_eulerian_cycle(graph);
        }
        debug_assert!(crate::graph::props::is_strongly_connected(graph));

        if graph.graph_type().directed {
            self.solve_directed(graph)
        } else {
            self.solve_undirected(graph)
        }
    }

    fn solve_undirected<G: IncidenceGraph>(
        &mut self,
        graph: &G,
    ) -> Result<GraphWalk<G::Vertex, G::Edge>> {
        let mut odd = Vec::new();
        for vertex in graph.vertices() {
            if graph.degree_of(vertex) % 2 == 1 {
                odd.push(vertex.clone());
            }
        }
        let count = odd.len();
        let shortest_path = DijkstraShortestPath::new(graph);
        let mut paths = vec![vec![None; count]; count];
        for source in 0..count.saturating_sub(1) {
            let from_source = shortest_path.single_source(&odd[source])?;
            for target in source + 1..count {
                paths[source][target] = Some(
                    from_source
                        .path(&odd[target])
                        .ok_or(GraphError::InvalidGraph(crate::GraphProperty::Connected))?,
                );
            }
        }

        let mut auxiliary = crate::UGraph::<usize>::simple().weighted();
        auxiliary.add_vertices(0..count);
        for source in 0..count {
            for target in source + 1..count {
                auxiliary.add_weighted_edge(
                    &source,
                    &target,
                    paths[source][target].as_ref().unwrap().weight(),
                )?;
            }
        }
        let mut matching = KolmogorovWeightedPerfectMatching::new(&auxiliary)?;
        let matching = matching.get_matching()?;
        let mut shortcuts = Vec::with_capacity(matching.len());
        for edge in matching.edges() {
            let (source, target) = auxiliary.edge_endpoints(edge).unwrap();
            let (low, high) = if source < target {
                (*source, *target)
            } else {
                (*target, *source)
            };
            shortcuts.push(paths[low][high].as_ref().unwrap().clone());
        }
        self.eulerize::<G, Undirected>(graph, shortcuts)
    }

    fn solve_directed<G: IncidenceGraph>(
        &mut self,
        graph: &G,
    ) -> Result<GraphWalk<G::Vertex, G::Edge>> {
        let mut negative = Vec::new();
        let mut positive = Vec::new();
        for vertex in graph.vertices() {
            let imbalance =
                graph.out_degree_of(vertex) as isize - graph.in_degree_of(vertex) as isize;
            if imbalance < 0 {
                negative.push((vertex.clone(), (-imbalance) as usize));
            } else if imbalance > 0 {
                positive.push((vertex.clone(), imbalance as usize));
            }
        }

        let shortest_path = DijkstraShortestPath::new(graph);
        let mut paths = vec![vec![None; positive.len()]; negative.len()];
        for (source_index, (source, _)) in negative.iter().enumerate() {
            let from_source = shortest_path.single_source(source)?;
            for (target_index, (target, _)) in positive.iter().enumerate() {
                paths[source_index][target_index] = Some(
                    from_source
                        .path(target)
                        .ok_or(GraphError::InvalidGraph(crate::GraphProperty::Connected))?,
                );
            }
        }

        let mut auxiliary = crate::UGraph::<usize>::simple().weighted();
        let mut duplicate_map = Vec::new();
        let mut negative_partition = IndexSet::new();
        let mut positive_partition = IndexSet::new();
        for (index, (_, copies)) in negative.iter().enumerate() {
            for _ in 0..*copies {
                let vertex = duplicate_map.len();
                auxiliary.add_vertex(vertex);
                duplicate_map.push((false, index));
                negative_partition.insert(vertex);
            }
        }
        for (index, (_, copies)) in positive.iter().enumerate() {
            for _ in 0..*copies {
                let vertex = duplicate_map.len();
                auxiliary.add_vertex(vertex);
                duplicate_map.push((true, index));
                positive_partition.insert(vertex);
            }
        }
        for &source in &negative_partition {
            for &target in &positive_partition {
                let source_index = duplicate_map[source].1;
                let target_index = duplicate_map[target].1;
                auxiliary.add_weighted_edge(
                    &source,
                    &target,
                    paths[source_index][target_index].as_ref().unwrap().weight(),
                )?;
            }
        }

        let mut matching = KuhnMunkresMinimalWeightBipartitePerfectMatching::new(
            &auxiliary,
            negative_partition.clone(),
            positive_partition.clone(),
        );
        let matching = matching.get_matching()?;
        let mut shortcuts = Vec::with_capacity(matching.len());
        for edge in matching.edges() {
            let (first, second) = auxiliary.edge_endpoints(edge).unwrap();
            let (negative_vertex, positive_vertex) = if negative_partition.contains(first) {
                (*first, *second)
            } else {
                (*second, *first)
            };
            let source_index = duplicate_map[negative_vertex].1;
            let target_index = duplicate_map[positive_vertex].1;
            shortcuts.push(paths[source_index][target_index].as_ref().unwrap().clone());
        }
        self.eulerize::<G, Directed>(graph, shortcuts)
    }

    fn eulerize<G: Graph, D: Direction>(
        &mut self,
        graph: &G,
        shortcuts: Vec<GraphWalk<G::Vertex, G::Edge>>,
    ) -> Result<GraphWalk<G::Vertex, G::Edge>> {
        let mut euler_graph = BaseGraph::<G::Vertex, usize, D>::with_edge_factory(
            GraphType::pseudograph().weighted(graph.graph_type().weighted),
            |id| id as usize,
        );
        euler_graph.add_vertices(graph.vertices().cloned());
        let mut edge_map = Vec::with_capacity(graph.edge_count() + shortcuts.len());
        for edge in graph.edges() {
            let (source, target) = graph.edge_endpoints(edge).unwrap();
            let synthetic = euler_graph.add_edge(source, target)?.unwrap();
            assert_eq!(synthetic, edge_map.len());
            edge_map.push(AugmentedEdge::Original(edge.clone()));
        }
        for path in shortcuts {
            let source = path.start_vertex().unwrap();
            let target = path.end_vertex().unwrap();
            let synthetic = euler_graph.add_edge(source, target)?.unwrap();
            assert_eq!(synthetic, edge_map.len());
            edge_map.push(AugmentedEdge::Shortcut(path));
        }

        let augmented = HierholzerEulerianCycle::new().get_eulerian_cycle(&euler_graph)?;
        let mut vertices = vec![augmented.vertices[0].clone()];
        let mut edges = Vec::new();
        for (step, synthetic) in augmented.edges.iter().enumerate() {
            match &edge_map[*synthetic] {
                AugmentedEdge::Original(edge) => {
                    edges.push(edge.clone());
                    vertices.push(augmented.vertices[step + 1].clone());
                }
                AugmentedEdge::Shortcut(path) => {
                    if vertices.last() == path.start_vertex() {
                        edges.extend(path.edges.iter().cloned());
                        vertices.extend(path.vertices.iter().skip(1).cloned());
                    } else {
                        edges.extend(path.edges.iter().rev().cloned());
                        vertices.extend(
                            path.vertices
                                .iter()
                                .take(path.vertices.len() - 1)
                                .rev()
                                .cloned(),
                        );
                    }
                    debug_assert!(vertices.last() == Some(&augmented.vertices[step + 1]));
                }
            }
        }
        let weight = edges.iter().map(|edge| graph.weight_of(edge)).sum();
        Ok(GraphWalk::new_unchecked(vertices, edges, weight))
    }
}
