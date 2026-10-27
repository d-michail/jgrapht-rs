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

use super::base::{
    HamiltonianCycleAlgorithm, check_graph, closed_vertex_list_to_tour, singleton_tour,
    vertex_list_to_tour, weight_between,
};
use crate::error::{GraphError, Result};
use crate::graph::{Graph, GraphExt};
use crate::walk::GraphWalk;
use indexmap::{IndexMap, IndexSet};

type Tour<G> = Result<Option<GraphWalk<<G as Graph>::Vertex, <G as Graph>::Edge>>>;

struct Closest<V> {
    tour_vertex: V,
    unvisited_vertex: Option<V>,
    distance: f64,
}

/// The nearest insertion heuristic algorithm for the TSP problem.
///
/// The travelling salesman problem (TSP) asks the following question: "Given a list of cities and
/// the distances between each pair of cities, what is the shortest possible route that visits each
/// city exactly once and returns to the origin city?".
///
/// Insertion heuristics are quite straightforward, and there are many variants to choose from. The
/// basics of insertion heuristics is to start with a tour of a subset of all cities, and then
/// inserting the rest by some heuristic. The initial sub-tour is often a triangle or the convex
/// hull. One can also start with a single edge as sub-tour. This implementation uses the shortest
/// edge by default as the initial sub-tour.
///
/// The implementation of this class is based on:
/// Nilsson, Christian. "Heuristics for the traveling salesman problem." Linkoping University 38
/// (2003)
///
/// This implementation can also be used in order to augment an existing partial tour. See
/// constructor `NearestInsertionHeuristicTSP(GraphPath)`.
///
/// The runtime complexity of this class is $O(V^2)$.
///
/// This algorithm requires that the graph is complete.
///
/// # Rust notes
///
/// `NearestInsertionHeuristicTSP`: grows a sub-tour by inserting the unvisited vertex nearest to
/// it next to the tour vertex it is nearest to.
pub struct NearestInsertionHeuristicTSP<G: Graph> {
    subtour: Option<GraphWalk<G::Vertex, G::Edge>>,
}

impl<G: Graph> Default for NearestInsertionHeuristicTSP<G> {
    fn default() -> Self {
        NearestInsertionHeuristicTSP { subtour: None }
    }
}

impl<G: Graph> NearestInsertionHeuristicTSP<G> {
    /// Starts from the shortest edge.
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts from the given sub-tour.
    pub fn with_subtour(subtour: GraphWalk<G::Vertex, G::Edge>) -> Self {
        NearestInsertionHeuristicTSP {
            subtour: Some(subtour),
        }
    }

    fn subtour(&self, graph: &G) -> Result<Vec<G::Vertex>> {
        let mut subtour_vertices = Vec::new();
        if let Some(subtour) = &self.subtour {
            if !subtour
                .vertex_list()
                .iter()
                .all(|v| graph.contains_vertex(v))
            {
                return Err(GraphError::InvalidArgument(
                    "Graph does not contain specified sub-tour vertices".into(),
                ));
            }
            if !subtour.edge_list().iter().all(|e| graph.contains_edge(e)) {
                return Err(GraphError::InvalidArgument(
                    "Graph does not contain specified sub-tour edges".into(),
                ));
            }
            if subtour.start_vertex() == subtour.end_vertex() {
                subtour_vertices.extend(subtour.vertex_list().iter().skip(1).cloned());
            } else {
                subtour_vertices.extend(subtour.vertex_list().iter().cloned());
            }
        }
        if subtour_vertices.is_empty() {
            let mut shortest: Option<(&G::Edge, f64)> = None;
            for e in graph.edges() {
                let w = graph.edge_weight(e).unwrap_or(1.0);
                if shortest.is_none_or(|(_, best)| w.total_cmp(&best).is_lt()) {
                    shortest = Some((e, w));
                }
            }
            let (e, _) = shortest.ok_or_else(|| GraphError::InvalidArgument("no edges".into()))?;
            let (s, t) = graph.edge_endpoints(e).expect("edge of the graph");
            subtour_vertices.push(s.clone());
            subtour_vertices.push(t.clone());
        }
        Ok(subtour_vertices)
    }

    fn closest_of(
        tour_vertex: &G::Vertex,
        unvisited: &IndexSet<G::Vertex>,
        graph: &G,
    ) -> Closest<G::Vertex> {
        let mut closest = None;
        let mut min_dist = f64::MAX;
        for unvisited_vertex in unvisited {
            let dist = weight_between(graph, tour_vertex, unvisited_vertex);
            if dist < min_dist {
                closest = Some(unvisited_vertex.clone());
                min_dist = dist;
            }
        }
        Closest {
            tour_vertex: tour_vertex.clone(),
            unvisited_vertex: closest,
            distance: min_dist,
        }
    }

    fn update_closest(
        current_closest: &mut IndexMap<G::Vertex, Closest<G::Vertex>>,
        chosen: &Closest<G::Vertex>,
        unvisited: &mut IndexSet<G::Vertex>,
        graph: &G,
    ) {
        let chosen_unvisited = chosen
            .unvisited_vertex
            .as_ref()
            .expect("an unvisited vertex");
        unvisited.shift_remove(chosen_unvisited);
        if unvisited.is_empty() {
            current_closest.clear();
            return;
        }
        for (v, c) in current_closest.iter_mut() {
            if chosen.tour_vertex == *v || c.unvisited_vertex.as_ref() == Some(chosen_unvisited) {
                *c = Self::closest_of(v, unvisited, graph);
            }
        }
        current_closest.insert(
            chosen_unvisited.clone(),
            Self::closest_of(chosen_unvisited, unvisited, graph),
        );
    }

    fn augment(&self, mut subtour: Vec<G::Vertex>, graph: &G) -> Vec<G::Vertex> {
        let mut unvisited: IndexSet<G::Vertex> = graph
            .vertices()
            .filter(|v| !subtour.contains(v))
            .cloned()
            .collect();
        let mut closest_vertices: IndexMap<G::Vertex, Closest<G::Vertex>> = subtour
            .iter()
            .map(|v| (v.clone(), Self::closest_of(v, &unvisited, graph)))
            .collect();
        while !unvisited.is_empty() {
            // Collections.min: the first of the minimal values
            let mut best: Option<&Closest<G::Vertex>> = None;
            for c in closest_vertices.values() {
                if best.is_none_or(|b| c.distance.total_cmp(&b.distance).is_lt()) {
                    best = Some(c);
                }
            }
            let chosen = best.expect("a closest vertex");
            let chosen = Closest {
                tour_vertex: chosen.tour_vertex.clone(),
                unvisited_vertex: chosen.unvisited_vertex.clone(),
                distance: chosen.distance,
            };
            let chosen_unvisited = chosen
                .unvisited_vertex
                .clone()
                .expect("an unvisited vertex");
            let i = subtour
                .iter()
                .position(|v| *v == chosen.tour_vertex)
                .expect("tour vertex");
            let vertex_before = &subtour[if i == 0 { subtour.len() - 1 } else { i - 1 }];
            let vertex_after = &subtour[if i == subtour.len() - 1 { 0 } else { i + 1 }];
            let insertion_cost_before = weight_between(graph, vertex_before, &chosen_unvisited)
                + chosen.distance
                - weight_between(graph, vertex_before, &chosen.tour_vertex);
            let insertion_cost_after = weight_between(graph, vertex_after, &chosen_unvisited)
                + chosen.distance
                - weight_between(graph, vertex_after, &chosen.tour_vertex);
            if insertion_cost_before < insertion_cost_after {
                subtour.insert(i, chosen_unvisited);
            } else {
                subtour.insert(i + 1, chosen_unvisited);
            }
            Self::update_closest(&mut closest_vertices, &chosen, &mut unvisited, graph);
        }
        subtour
    }
}

impl<G: Graph> HamiltonianCycleAlgorithm<G> for NearestInsertionHeuristicTSP<G> {
    fn get_tour(&mut self, graph: &G) -> Tour<G> {
        check_graph(graph)?;
        if graph.vertex_count() == 1 {
            return Ok(Some(singleton_tour(graph)));
        }
        let subtour = self.subtour(graph)?;
        vertex_list_to_tour(self.augment(subtour, graph), graph).map(Some)
    }
}

/// The farthest insertion heuristic algorithm for the TSP problem.
///
/// The travelling salesman problem (TSP) asks the following question: "Given a list of cities and
/// the distances between each pair of cities, what is the shortest possible route that visits each
/// city exactly once and returns to the origin city?".
///
/// Insertion heuristics are quite straightforward, and there are many variants to choose from. The
/// basics of insertion heuristics is to start with a partial tour of a subset of all cities, and
/// then iteratively an unvisited vertex (a vertex whichis not in the tour) is chosen given a
/// criterion and inserted in the best position of the partial tour. Per each iteration, the farthest
/// insertion heuristic selects the farthest unvisited vertex from the partial tour. This algorithm
/// provides a guarantee to compute tours no more than 0(log N) times optimum (assuming the triangle
/// inequality). However, regarding practical results, some references refer to this heuristic as one
/// of the best among the category of insertion heuristics. This implementation uses the longest edge
/// by default as the initial sub-tour if one is not provided.
///
/// The description of this algorithm can be consulted on:
/// Johnson, D. S., & McGeoch, L. A. (2007). Experimental Analysis of Heuristics for the STSP. In
/// G. Gutin & A. P. Punnen (Eds.), The Traveling Salesman Problem and Its Variations (pp.
/// 369–443). Springer US. <https://doi.org/10.1007/0-306-48213-4_9>
///
/// This implementation can also be used in order to augment an existing partial tour. See
/// constructor `FarthestInsertionHeuristicTSP(GraphPath)`.
///
/// The runtime complexity of this class is $O(V^2)$.
///
/// This algorithm requires that the graph is complete.
///
/// # Rust notes
///
/// `FarthestInsertionHeuristicTSP`: inserts the vertex farthest from the sub-tour at the position
/// that lengthens the tour least. Starts from the longest edge unless given a sub-tour.
pub struct FarthestInsertionHeuristicTSP<G: Graph> {
    initial_subtour: Option<GraphWalk<G::Vertex, G::Edge>>,
}

impl<G: Graph> Default for FarthestInsertionHeuristicTSP<G> {
    fn default() -> Self {
        FarthestInsertionHeuristicTSP {
            initial_subtour: None,
        }
    }
}

impl<G: Graph> FarthestInsertionHeuristicTSP<G> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_subtour(subtour: GraphWalk<G::Vertex, G::Edge>) -> Self {
        FarthestInsertionHeuristicTSP {
            initial_subtour: Some(subtour),
        }
    }
}

impl<G: Graph> HamiltonianCycleAlgorithm<G> for FarthestInsertionHeuristicTSP<G> {
    fn get_tour(&mut self, graph: &G) -> Tour<G> {
        check_graph(graph)?;
        if graph.vertex_count() == 1 {
            return Ok(Some(singleton_tour(graph)));
        }
        let index_list: Vec<G::Vertex> = graph.vertices().cloned().collect();
        let n = index_list.len();
        let vertex_map: std::collections::HashMap<&G::Vertex, usize> =
            index_list.iter().enumerate().map(|(i, v)| (v, i)).collect();

        // computeDistanceMatrix
        let mut longest_edge: Option<&G::Edge> = None;
        let mut longest_edge_weight = f64::NEG_INFINITY;
        let mut all_dist = vec![vec![0.0f64; n]; n];
        for edge in graph.edges() {
            let (source, target) = graph.edge_endpoints(edge).expect("edge of the graph");
            if source != target {
                let (i, j) = (vertex_map[source], vertex_map[target]);
                if all_dist[i][j] == 0.0 {
                    let w = graph.edge_weight(edge).unwrap_or(1.0);
                    all_dist[i][j] = w;
                    all_dist[j][i] = w;
                    if longest_edge_weight < all_dist[i][j] {
                        longest_edge_weight = all_dist[i][j];
                        longest_edge = Some(edge);
                    }
                }
            }
        }

        if self
            .initial_subtour
            .as_ref()
            .is_none_or(|subtour| subtour.vertex_list().is_empty())
        {
            let longest = longest_edge.ok_or_else(|| {
                GraphError::InvalidArgument("graph contains no usable edge".into())
            })?;
            let (v, u) = graph.edge_endpoints(longest).expect("edge of the graph");
            self.initial_subtour = Some(GraphWalk::from_vertex_list(
                graph,
                vec![v.clone(), u.clone()],
                -1.0,
            )?);
        }
        let initial = self
            .initial_subtour
            .as_ref()
            .expect("initialized above")
            .vertex_list();

        // initPartialTour
        let mut tour = vec![0usize; n + 1];
        let mut visited = std::collections::HashSet::new();
        let mut i = 0;
        for v in initial {
            let iv = vertex_map[v];
            visited.insert(iv);
            tour[i] = iv;
            i += 1;
        }
        for v in 0..n {
            if !visited.contains(&v) {
                tour[i] = v;
                i += 1;
            }
        }

        // initDistances
        let start = initial.len();
        let mut distances = vec![0.0f64; n];
        for d in distances.iter_mut().skip(start) {
            *d = f64::INFINITY;
        }
        for i in start..n {
            for j in 0..start {
                distances[i] = distances[i].min(all_dist[tour[i]][tour[j]]);
            }
        }

        for i in start..n {
            // getFarthest
            let mut idx_farthest = usize::MAX;
            let mut max_dist = f64::NEG_INFINITY;
            for (k, &dist) in distances.iter().enumerate().skip(i) {
                if dist > max_dist {
                    idx_farthest = k;
                    max_dist = dist;
                }
            }
            let k = tour[idx_farthest];
            let mut saving = f64::INFINITY;
            let mut best_index = usize::MAX;
            for j in 0..=i {
                let x = if j == 0 { tour[i - 1] } else { tour[j - 1] };
                let y = if j == i { tour[0] } else { tour[j] };
                let dxk = all_dist[x][k];
                let dky = all_dist[k][y];
                let dxy = if x == y { 0.0 } else { all_dist[x][y] };
                let saving_tmp = dxk + dky - dxy;
                if saving_tmp < saving {
                    saving = saving_tmp;
                    best_index = j;
                }
            }
            tour.swap(i, idx_farthest);
            distances.swap(i, idx_farthest);
            for j in (best_index + 1..=i).rev() {
                tour[j] = tour[j - 1];
            }
            tour[best_index] = k;
            // updateDistances(k, i + 1)
            for m in i + 1..n {
                distances[m] = all_dist[k][m].min(distances[m]);
            }
        }
        tour[n] = tour[0];
        let tour_list: Vec<G::Vertex> = tour.iter().map(|&i| index_list[i].clone()).collect();
        closed_vertex_list_to_tour(tour_list, graph).map(Some)
    }
}
