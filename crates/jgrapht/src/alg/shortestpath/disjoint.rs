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

use crate::alg::util::Indexed;
use crate::graph::OutgoingGraph;
use crate::graph::{DirectedGraph, props};
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

#[derive(Clone)]
struct ResidualArc {
    tail: usize,
    head: usize,
    cost: f64,
    residual: bool,
    original_edge: Option<usize>,
    forward: bool,
}

struct ResidualNetwork {
    arcs: Vec<ResidualArc>,
    outgoing: Vec<Vec<usize>>,
}

impl ResidualNetwork {
    fn new(vertices: usize) -> Self {
        Self {
            arcs: Vec::new(),
            outgoing: vec![Vec::new(); vertices],
        }
    }

    fn add_arc(&mut self, tail: usize, head: usize, cost: f64, original_edge: Option<usize>) {
        let f = self.arcs.len();
        let r = f + 1;
        self.arcs.push(ResidualArc {
            tail,
            head,
            cost,
            residual: true,
            original_edge,
            forward: true,
        });
        self.arcs.push(ResidualArc {
            tail: head,
            head: tail,
            cost: -cost,
            residual: false,
            original_edge,
            forward: false,
        });
        self.outgoing[tail].push(f);
        self.outgoing[head].push(r);
    }

    fn augment(&mut self, source: usize, target: usize, pred: &[usize]) {
        let mut vertex = target;
        while vertex != source {
            let arc = pred[vertex];
            self.arcs[arc].residual = false;
            self.arcs[arc ^ 1].residual = true;
            vertex = self.arcs[arc].tail;
        }
    }

    fn dijkstra(
        &self,
        source: usize,
        target: usize,
        potential: &mut [f64],
    ) -> Result<Option<Vec<usize>>> {
        let n = self.outgoing.len();
        let mut dist = vec![f64::INFINITY; n];
        let mut pred = vec![usize::MAX; n];
        let mut heap = BinaryHeap::new();
        dist[source] = 0.0;
        heap.push((Reverse(OrderedF64(0.0)), source));
        while let Some((Reverse(OrderedF64(d)), vertex)) = heap.pop() {
            if d > dist[vertex] {
                continue;
            }
            for &arc_index in &self.outgoing[vertex] {
                let arc = &self.arcs[arc_index];
                if !arc.residual {
                    continue;
                }
                let mut reduced = arc.cost + potential[vertex] - potential[arc.head];
                if reduced < 0.0 && reduced > -1e-10 {
                    reduced = 0.0;
                }
                if reduced.is_nan() || reduced < 0.0 {
                    return Err(GraphError::InvalidWeight(crate::WeightProblem::Negative));
                }
                let candidate = d + reduced;
                if candidate < dist[arc.head] {
                    dist[arc.head] = candidate;
                    pred[arc.head] = arc_index;
                    heap.push((Reverse(OrderedF64(candidate)), arc.head));
                }
            }
        }
        if !dist[target].is_finite() {
            return Ok(None);
        }
        for (vertex, &distance) in dist.iter().enumerate() {
            if distance.is_finite() {
                potential[vertex] += distance;
            }
        }
        Ok(Some(pred))
    }

    fn bellman_ford(&self, source: usize, target: usize) -> Result<Option<Vec<usize>>> {
        let n = self.outgoing.len();
        let mut dist = vec![f64::INFINITY; n];
        let mut pred = vec![usize::MAX; n];
        dist[source] = 0.0;
        for round in 0..n {
            let mut changed = false;
            for (arc_index, arc) in self.arcs.iter().enumerate() {
                if !arc.residual || !dist[arc.tail].is_finite() {
                    continue;
                }
                let candidate = dist[arc.tail] + arc.cost;
                if candidate < dist[arc.head] {
                    dist[arc.head] = candidate;
                    pred[arc.head] = arc_index;
                    changed = true;
                    if round + 1 == n {
                        return Err(GraphError::NegativeCycle);
                    }
                }
            }
            if !changed {
                break;
            }
        }
        Ok(dist[target].is_finite().then_some(pred))
    }
}

#[derive(Clone, Copy)]
enum SearchKind {
    Suurballe,
    Bhandari,
}

fn validate<G: DirectedGraph + OutgoingGraph>(
    graph: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
) -> Result<()> {
    if k == 0 {
        return Err(GraphError::must_be_positive("number_of_paths"));
    }
    if source == sink {
        return Err(GraphError::SourceEqualsSink);
    }
    if !graph.contains_vertex(source) || !graph.contains_vertex(sink) {
        return Err(GraphError::VertexNotFound);
    }
    if !props::is_simple(graph) {
        return Err(GraphError::InvalidGraph(crate::GraphProperty::Simple));
    }
    Ok(())
}

fn get_paths<G: DirectedGraph + OutgoingGraph>(
    graph: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
    kind: SearchKind,
) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
    validate(graph, source, sink, k)?;
    let indexed = Indexed::new(graph);
    let s = indexed.idx[source];
    let t = indexed.idx[sink];
    let mut residual = ResidualNetwork::new(indexed.n());
    for (edge, &(tail, head)) in indexed.ends.iter().enumerate() {
        let cost = indexed.w[edge];
        if matches!(kind, SearchKind::Suurballe) && (cost.is_nan() || cost < 0.0) {
            return Err(GraphError::InvalidWeight(crate::WeightProblem::Negative));
        }
        residual.add_arc(tail, head, cost, Some(edge));
    }

    let mut potential = vec![0.0; indexed.n()];
    let mut flow = 0;
    // Java toggles the unordered endpoint pair of every edge in each intermediate path in a
    // LinkedHashMap. Retain that insertion order for the final path decomposition.
    let mut valid_edge_order: Vec<Option<((usize, usize), usize)>> = Vec::new();
    let mut valid_edge_positions = HashMap::new();
    while flow < k {
        let predecessor = match kind {
            SearchKind::Suurballe => residual.dijkstra(s, t, &mut potential)?,
            SearchKind::Bhandari => residual.bellman_ford(s, t)?,
        };
        let Some(predecessor) = predecessor else {
            break;
        };
        let mut path_arcs = Vec::new();
        let mut vertex = t;
        while vertex != s {
            let arc = predecessor[vertex];
            path_arcs.push(arc);
            vertex = residual.arcs[arc].tail;
        }
        path_arcs.reverse();
        for arc in path_arcs {
            let arc = &residual.arcs[arc];
            let key = if arc.tail < arc.head {
                (arc.tail, arc.head)
            } else {
                (arc.head, arc.tail)
            };
            if let Some(position) = valid_edge_positions.remove(&key) {
                valid_edge_order[position] = None;
            } else {
                let position = valid_edge_order.len();
                let edge = arc
                    .original_edge
                    .expect("arc derived from an original edge");
                valid_edge_order.push(Some((key, edge)));
                valid_edge_positions.insert(key, position);
            }
        }
        residual.augment(s, t, &predecessor);
        flow += 1;
    }

    // A saturated original forward arc carries one unit of final flow. Decompose the integral
    // flow into source-target paths; DFS avoids accidentally following a detached flow cycle.
    let mut selected = vec![false; indexed.es.len()];
    for arc in residual.arcs.iter().filter(|arc| arc.forward) {
        if !arc.residual {
            if let Some(edge) = arc.original_edge {
                selected[edge] = true;
            }
        }
    }
    let mut adjacency = vec![Vec::new(); indexed.n()];
    for (_, edge) in valid_edge_order.into_iter().flatten() {
        if selected[edge] {
            adjacency[indexed.ends[edge].0].push(edge);
        }
    }

    fn find_path(
        vertex: usize,
        target: usize,
        adjacency: &[Vec<usize>],
        ends: &[(usize, usize)],
        selected: &[bool],
        seen: &mut [bool],
        path: &mut Vec<usize>,
    ) -> bool {
        if vertex == target {
            return true;
        }
        seen[vertex] = true;
        for &edge in &adjacency[vertex] {
            if !selected[edge] {
                continue;
            }
            let next = ends[edge].1;
            if seen[next] {
                continue;
            }
            path.push(edge);
            if find_path(next, target, adjacency, ends, selected, seen, path) {
                return true;
            }
            path.pop();
        }
        false
    }

    let mut result = Vec::new();
    for _ in 0..flow {
        let mut edge_path = Vec::new();
        let mut seen = vec![false; indexed.n()];
        if !find_path(
            s,
            t,
            &adjacency,
            &indexed.ends,
            &selected,
            &mut seen,
            &mut edge_path,
        ) {
            break;
        }
        for &edge in &edge_path {
            selected[edge] = false;
        }
        let mut vertices = vec![indexed.vertex(s)];
        vertices.extend(
            edge_path
                .iter()
                .map(|&edge| indexed.vertex(indexed.ends[edge].1)),
        );
        let weight = edge_path.iter().map(|&edge| indexed.w[edge]).sum();
        result.push(GraphWalk::new_unchecked(
            vertices,
            edge_path
                .iter()
                .map(|&edge| indexed.es[edge].clone())
                .collect(),
            weight,
        ));
    }
    result.sort_by(|left, right| left.weight.total_cmp(&right.weight));
    Ok(result)
}

/// An implementation of Suurballe algorithm for finding K edge-*disjoint* shortest paths. The
/// algorithm determines the k disjoint shortest simple paths in increasing order of weight. Only
/// directed simple graphs are allowed.
///
/// The algorithm is running k sequential Dijkstra iterations to find the shortest path at each step.
/// Hence, yielding a complexity of k*O(Dijkstra).
///
/// For further reference see [Wikipedia page](https://en.wikipedia.org/wiki/Suurballe%27s_algorithm)
///
/// - Suurballe, J. W.; Tarjan, R. E. (1984), A quick method for finding shortest pairs of disjoint
///   paths.
///
/// # Rust notes
///
/// Suurballe's repeated reweight-and-reverse algorithm for edge-disjoint paths.
pub struct SuurballeKDisjointShortestPaths<'g, G> {
    graph: &'g G,
}

impl<'g, G: DirectedGraph + OutgoingGraph> SuurballeKDisjointShortestPaths<'g, G> {
    /// Creates a new instance of the algorithm.
    pub fn new(graph: &'g G) -> Self {
        Self { graph }
    }

    /// Returns the $k$ shortest simple paths in increasing order of weight.
    pub fn get_paths(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
        k: usize,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        get_paths(self.graph, source, sink, k, SearchKind::Suurballe)
    }
}

/// An implementation of Bhandari algorithm for finding $K$ edge-*disjoint* shortest paths.
/// The algorithm determines the $k$ edge-disjoint shortest simple paths in increasing order of
/// weight. Weights can be negative (but no negative cycle is allowed). Only directed simple graphs
/// are allowed.
///
/// The algorithm is running $k$ sequential Bellman-Ford iterations to find the shortest path at each
/// step. Hence, yielding a complexity of $k$*O(Bellman-Ford).
///
/// - Bhandari, Ramesh 1999. Survivable networks: algorithms for diverse routing. 477. Springer. p.
/// 46. ISBN 0-7923-8381-8.
/// - Iqbal, F. and Kuipers, F. A. 2015.
///   [Disjoint Paths in Networks](https://www.nas.ewi.tudelft.nl/people/Fernando/papers/Wiley.pdf). Wiley Encyclopedia of Electrical and Electronics Engineering. 1–11.
///
/// # Rust notes
///
/// Bhandari's repeated Bellman-Ford/reversed-edge algorithm for edge-disjoint paths.
pub struct BhandariKDisjointShortestPaths<'g, G> {
    graph: &'g G,
}

impl<'g, G: DirectedGraph + OutgoingGraph> BhandariKDisjointShortestPaths<'g, G> {
    /// Creates a new instance of the algorithm.
    pub fn new(graph: &'g G) -> Self {
        Self { graph }
    }

    /// Returns the $k$ shortest simple paths in increasing order of weight.
    pub fn get_paths(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
        k: usize,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        get_paths(self.graph, source, sink, k, SearchKind::Bhandari)
    }
}
