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
    HamiltonianCycleAlgorithm, check_graph, closed_vertex_list_to_tour, edge_set_to_tour,
    require_not_empty, singleton_tour, vertex_list_to_tour,
};
use crate::alg::spanning::kruskal;
use crate::error::{GraphError, Result};
use crate::graph::props::has_ore_property;
use crate::graph::{Graph, GraphExt, GraphMut, GraphMutExt, UGraph};
use crate::traverse::DepthFirstIterator;
use crate::util::UnionFind;
use crate::util::java::{RandomSource, java_shuffle};
use crate::walk::GraphWalk;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::{HashMap, HashSet, VecDeque};

type Tour<G> = Result<Option<GraphWalk<<G as Graph>::Vertex, <G as Graph>::Edge>>>;

/// `RandomTourTSP`: a uniformly random permutation of the vertices.
pub struct RandomTourTSP<R = StdRng> {
    rng: R,
}

impl RandomTourTSP<StdRng> {
    pub fn new() -> Self {
        RandomTourTSP {
            rng: StdRng::from_os_rng(),
        }
    }
}

impl Default for RandomTourTSP<StdRng> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: RandomSource> RandomTourTSP<R> {
    pub fn with_rng(rng: R) -> Self {
        RandomTourTSP { rng }
    }
}

impl<G: Graph, R: RandomSource> HamiltonianCycleAlgorithm<G> for RandomTourTSP<R> {
    fn get_tour(&mut self, graph: &G) -> Tour<G> {
        check_graph(graph)?;
        let mut vertices: Vec<G::Vertex> = graph.vertices().cloned().collect();
        if vertices.len() == 1 {
            return Ok(Some(singleton_tour(graph)));
        }
        java_shuffle(&mut vertices, &mut self.rng);
        vertex_list_to_tour(vertices, graph).map(Some)
    }
}

/// `NearestNeighborHeuristicTSP`: starts at a given or random vertex and repeatedly walks to the
/// nearest unvisited vertex.
pub struct NearestNeighborHeuristicTSP<V, R = StdRng> {
    rng: R,
    /// initial vertices still to be used, one per call
    initial_vertices: VecDeque<V>,
}

impl<V> NearestNeighborHeuristicTSP<V, StdRng> {
    pub fn new() -> Self {
        Self::with_rng(StdRng::from_os_rng())
    }

    pub fn with_seed(seed: u64) -> Self {
        Self::with_rng(StdRng::seed_from_u64(seed))
    }

    /// The first call starts at `first`; later calls start at random vertices.
    pub fn with_first(first: V) -> Self {
        Self::with_initial_vertices(vec![first])
    }

    /// Call `i` starts at the `i`-th given vertex; once they are used up, at random vertices.
    pub fn with_initial_vertices(initial_vertices: impl IntoIterator<Item = V>) -> Self {
        NearestNeighborHeuristicTSP {
            rng: StdRng::from_os_rng(),
            initial_vertices: initial_vertices.into_iter().collect(),
        }
    }
}

impl<V> Default for NearestNeighborHeuristicTSP<V, StdRng> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, R> NearestNeighborHeuristicTSP<V, R> {
    pub fn with_rng(rng: R) -> Self {
        NearestNeighborHeuristicTSP {
            rng,
            initial_vertices: VecDeque::new(),
        }
    }
}

impl<G: Graph, R: RandomSource> HamiltonianCycleAlgorithm<G>
    for NearestNeighborHeuristicTSP<G::Vertex, R>
{
    fn get_tour(&mut self, graph: &G) -> Tour<G> {
        check_graph(graph)?;
        if graph.vertex_count() == 1 {
            return Ok(Some(singleton_tour(graph)));
        }
        let n = graph.vertex_count();
        let mut path: Vec<G::Vertex> = graph.vertices().cloned().collect();
        let initial_index = match self.initial_vertices.pop_front() {
            Some(first) => path.iter().position(|v| *v == first).ok_or_else(|| {
                GraphError::InvalidArgument("Specified initial vertex is not in graph".into())
            })?,
            None => self.rng.next_int_bound(n),
        };
        path.swap(0, initial_index);
        let limit = n - 1; // the last vertex won't change
        for i in 1..limit {
            let nearest = nearest_neighbor(&path[i - 1], &path, i, graph);
            path.swap(i, nearest);
        }
        path.push(path[0].clone());
        closed_vertex_list_to_tour(path, graph).map(Some)
    }
}

fn nearest_neighbor<G: Graph>(
    current: &G::Vertex,
    vertices: &[G::Vertex],
    start: usize,
    g: &G,
) -> usize {
    let mut closest = usize::MAX;
    let mut min_dist = f64::MAX;
    for (i, v) in vertices.iter().enumerate().skip(start) {
        let dist = super::base::weight_between(g, current, v);
        if dist < min_dist {
            closest = i;
            min_dist = dist;
        }
    }
    closest
}

/// `TwoApproxMetricTSP`: shortcuts a depth-first walk of a minimum spanning tree.
#[derive(Default)]
pub struct TwoApproxMetricTSP;

impl TwoApproxMetricTSP {
    pub fn new() -> Self {
        TwoApproxMetricTSP
    }
}

impl<G: Graph> HamiltonianCycleAlgorithm<G> for TwoApproxMetricTSP {
    fn get_tour(&mut self, graph: &G) -> Tour<G> {
        check_graph(graph)?;
        if graph.vertex_count() == 1 {
            return Ok(Some(singleton_tour(graph)));
        }
        let mut mst = UGraph::<G::Vertex>::simple();
        mst.add_vertices(graph.vertices().cloned());
        for e in kruskal(graph).edges {
            let (u, v) = graph.edge_endpoints(&e).expect("edge of the graph");
            mst.add_edge(u, v)?;
        }
        let start = graph.vertices().next().expect("non-empty").clone();
        let mut found = HashSet::new();
        let mut tour = Vec::with_capacity(graph.vertex_count() + 1);
        for v in DepthFirstIterator::from_start(&mst, &start)? {
            if found.insert(v.clone()) {
                tour.push(v.clone());
            }
        }
        vertex_list_to_tour(tour, graph).map(Some)
    }
}

/// `GreedyHeuristicTSP`: repeatedly adds the cheapest edge that keeps a partial tour.
#[derive(Default)]
pub struct GreedyHeuristicTSP;

impl GreedyHeuristicTSP {
    pub fn new() -> Self {
        GreedyHeuristicTSP
    }
}

impl<G: Graph> HamiltonianCycleAlgorithm<G> for GreedyHeuristicTSP {
    fn get_tour(&mut self, graph: &G) -> Tour<G> {
        check_graph(graph)?;
        let n = graph.vertex_count();
        if n == 1 {
            return Ok(Some(singleton_tour(graph)));
        }
        let mut edges: Vec<&G::Edge> = graph.edges().collect();
        edges.sort_by(|a, b| {
            let (wa, wb) = (
                graph.edge_weight(a).unwrap_or(1.0),
                graph.edge_weight(b).unwrap_or(1.0),
            );
            wa.total_cmp(&wb)
        });
        let index: HashMap<&G::Vertex, usize> =
            graph.vertices().enumerate().map(|(i, v)| (v, i)).collect();
        let mut tour_edges: HashSet<G::Edge> = HashSet::with_capacity(n);
        let mut vertex_degree = vec![0usize; n];
        let mut tour_set = UnionFind::new(n);
        for edge in edges {
            if tour_edges.len() >= n {
                break;
            }
            let (v1, v2) = graph.edge_endpoints(edge).expect("edge of the graph");
            let (i1, i2) = (index[v1], index[v2]);
            let last_edge = tour_edges.len() == n - 1;
            let can_add = vertex_degree[i1] <= 1
                && vertex_degree[i2] <= 1
                && if tour_set.find(i1) == tour_set.find(i2) {
                    last_edge
                } else {
                    !last_edge
                };
            if can_add {
                tour_edges.insert(edge.clone());
                vertex_degree[i1] += 1;
                vertex_degree[i2] += 1;
                tour_set.union(i1, i2);
            }
        }
        edge_set_to_tour(&tour_edges, graph).map(Some)
    }
}

/// `HeldKarpTSP`: the exact dynamic program, `O(2^n n²)`; at most 31 vertices.
#[derive(Default)]
pub struct HeldKarpTSP;

/// `Double.MIN_VALUE`, the smallest positive double, marks an unset memo entry.
const MIN_VALUE: f64 = f64::from_bits(1);

impl HeldKarpTSP {
    pub fn new() -> Self {
        HeldKarpTSP
    }

    fn memo(previous_node: usize, state: usize, c: &mut [Vec<f64>], w: &[Vec<f64>]) -> f64 {
        if c[previous_node][state] != MIN_VALUE {
            return c[previous_node][state];
        }
        let mut total_cost = f64::MAX;
        if state == (1usize << w.len()) - 1 {
            if w[previous_node][0] != f64::MAX {
                total_cost = w[previous_node][0];
            }
        } else {
            for i in 0..w.len() {
                if (state >> i) & 1 == 0 && w[previous_node][i] != f64::MAX {
                    total_cost =
                        total_cost.min(w[previous_node][i] + Self::memo(i, state ^ (1 << i), c, w));
                }
            }
        }
        c[previous_node][state] = total_cost;
        total_cost
    }
}

impl<G: Graph> HamiltonianCycleAlgorithm<G> for HeldKarpTSP {
    fn get_tour(&mut self, graph: &G) -> Tour<G> {
        require_not_empty(graph)?;
        let n = graph.vertex_count();
        if n == 1 {
            return Ok(Some(singleton_tour(graph)));
        }
        if n > 31 {
            return Err(GraphError::InvalidArgument(
                "The internal representation of the dynamic programming state space cannot \
                 represent graphs containing more than 31 vertices. The runtime complexity of this \
                 implementation, O(2^|V| x |V|^2),  makes it unsuitable for graphs with more than 31 \
                 vertices."
                    .into(),
            ));
        }
        let index_list: Vec<G::Vertex> = graph.vertices().cloned().collect();
        let vertex_map: HashMap<&G::Vertex, usize> =
            index_list.iter().enumerate().map(|(i, v)| (v, i)).collect();
        let mut w = vec![vec![f64::MAX; n]; n];
        for e in graph.edges() {
            let (source, target) = graph.edge_endpoints(e).expect("edge of the graph");
            let (u, v) = (vertex_map[source], vertex_map[target]);
            let weight = graph.edge_weight(e).unwrap_or(1.0);
            w[u][v] = w[u][v].min(weight);
            if !graph.graph_type().directed {
                w[v][u] = w[v][u].min(weight);
            }
        }
        let mut c = vec![vec![MIN_VALUE; 1usize << n]; n];
        let tour_weight = Self::memo(0, 1, &mut c, &w);
        if tour_weight == f64::MAX {
            return Ok(None);
        }
        // reconstruct
        let mut vertex_list = Vec::with_capacity(n + 1);
        let (mut last_node, mut last_state) = (0usize, 1usize);
        vertex_list.push(index_list[last_node].clone());
        for _ in 1..n {
            let mut next_node = usize::MAX;
            for node in 1..n {
                if last_state & (1 << node) == 0
                    && w[last_node][node] != f64::MAX
                    && c[node][last_state ^ (1 << node)] != MIN_VALUE
                    && (c[node][last_state ^ (1 << node)] + w[last_node][node])
                        .total_cmp(&c[last_node][last_state])
                        .is_eq()
                {
                    next_node = node;
                    break;
                }
            }
            debug_assert!(next_node != usize::MAX);
            vertex_list.push(index_list[next_node].clone());
            last_state ^= 1 << next_node;
            last_node = next_node;
        }
        vertex_list_to_tour(vertex_list, graph).map(Some)
    }
}

/// `PalmerHamiltonianCycle`: closes the gaps of an arbitrary vertex ordering in graphs that satisfy
/// Ore's property.
#[derive(Default)]
pub struct PalmerHamiltonianCycle;

impl PalmerHamiltonianCycle {
    pub fn new() -> Self {
        PalmerHamiltonianCycle
    }

    fn search_and_close_gap<G: Graph>(tour: &mut [G::Vertex], graph: &G) -> Result<bool> {
        let n = tour.len();
        let mut v = tour[n - 1].clone(); // "last to first" is checked first
        for i in 0..n {
            let v_n = tour[i].clone();
            if !graph.contains_edge_between(&v, &v_n) {
                let mut u = tour[n - 1].clone();
                for j in 0..n {
                    let u_n = tour[j].clone();
                    let distinct = v != u && v_n != u && v != u_n;
                    if distinct
                        && graph.contains_edge_between(&v, &u)
                        && graph.contains_edge_between(&v_n, &u_n)
                    {
                        Self::reverse_in_circle(tour, i as isize, j as isize - 1);
                        return Ok(true);
                    }
                    u = u_n;
                }
                return Err(GraphError::InvalidArgument(
                    "Found a gap but no mean to close it".into(),
                ));
            }
            v = v_n;
        }
        Ok(false)
    }

    fn reverse_in_circle<V>(array: &mut [V], start: isize, end: isize) {
        if start < end {
            reverse(array, start, end);
        } else {
            reverse(array, end + 1, start - 1);
        }
    }
}

/// `ArrayUtil.reverse`: reverses the inclusive range `from..=to` (nothing if `from >= to`).
pub(crate) fn reverse<T>(arr: &mut [T], from: isize, to: isize) {
    let (mut i, mut j) = (from, to);
    while i < j {
        arr.swap(i as usize, j as usize);
        i += 1;
        j -= 1;
    }
}

impl<G: Graph> HamiltonianCycleAlgorithm<G> for PalmerHamiltonianCycle {
    fn get_tour(&mut self, graph: &G) -> Tour<G> {
        if !has_ore_property(graph) {
            return Err(GraphError::InvalidArgument(
                "Graph doesn't have Ore's property".into(),
            ));
        }
        let mut tour: Vec<G::Vertex> = graph.vertices().cloned().collect();
        while Self::search_and_close_gap(&mut tour, graph)? {}
        tour.push(tour[0].clone());
        closed_vertex_list_to_tour(tour, graph).map(Some)
    }
}
