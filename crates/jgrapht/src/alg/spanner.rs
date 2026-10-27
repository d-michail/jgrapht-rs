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

//! Graph spanners.

use super::util::Indexed;
use crate::graph::Graph;
use crate::util::OrderedF64;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Greedy `(2k-1)`-spanner of an undirected weighted graph (Althöfer et al.): scan edges by
/// increasing weight and keep an edge only if the spanner built so far offers no path of
/// length at most `(2k-1) * w`. Returns the kept edges.
pub fn greedy_spanner<G: Graph>(g: &G, k: usize) -> Result<Vec<G::Edge>> {
    if g.graph_type().directed {
        return Err(GraphError::InvalidGraph(crate::GraphProperty::Undirected));
    }
    if k == 0 {
        return Err(GraphError::must_be_positive("k"));
    }
    let ix = Indexed::new(g);
    let stretch = (2 * k - 1) as f64;
    let mut order: Vec<usize> = (0..ix.es.len()).collect();
    order.sort_by(|&a, &b| ix.w[a].total_cmp(&ix.w[b]));
    let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); ix.n()];
    let mut kept = Vec::new();
    for e in order {
        let (a, b) = ix.ends[e];
        if a == b {
            continue;
        }
        let limit = stretch * ix.w[e];
        // bounded Dijkstra from a towards b inside the current spanner
        let mut dist = vec![f64::INFINITY; ix.n()];
        dist[a] = 0.0;
        let mut heap = BinaryHeap::from([Reverse((OrderedF64(0.0), a))]);
        let mut reached = false;
        while let Some(Reverse((OrderedF64(d), v))) = heap.pop() {
            if v == b {
                reached = true;
                break;
            }
            if d > dist[v] {
                continue;
            }
            for &(u, w) in &adj[v] {
                let nd = d + w;
                if nd <= limit && nd < dist[u] {
                    dist[u] = nd;
                    heap.push(Reverse((OrderedF64(nd), u)));
                }
            }
        }
        if !reached {
            adj[a].push((b, ix.w[e]));
            adj[b].push((a, ix.w[e]));
            kept.push(ix.es[e].clone());
        }
    }
    Ok(kept)
}

/// The result of a spanner algorithm (`SpannerAlgorithm.Spanner`).
#[derive(Debug, Clone, PartialEq)]
pub struct Spanner<E: Eq + std::hash::Hash> {
    pub edges: indexmap::IndexSet<E>,
    pub weight: f64,
}

/// Greedy algorithm for $(2k-1)$-multiplicative spanner construction (for any integer
/// `k >= 1`).
///
/// The spanner is guaranteed to contain $O(n^{1+1/k})$ edges and the shortest path distance between
/// any two vertices in the spanner is at most $2k-1$ times the corresponding shortest path distance
/// in the original graph. Here n denotes the number of vertices of the graph.
///
/// The algorithm is described in: Althoefer, Das, Dobkin, Joseph, Soares.
/// [On Sparse Spanners of Weighted Graphs](https://doi.org/10.1007/BF02189308). Discrete
/// Computational Geometry 9(1):81-100, 1993.
///
/// If the graph is unweighted the algorithm runs in $O(m n^{1+1/k})$ time. Setting $k$ to infinity
/// will result in a slow version of Kruskal's algorithm where cycle detection is performed by a BFS
/// computation. In such a case use the implementation of Kruskal with union-find. Here n and m are
/// the number of vertices and edges of the graph respectively.
///
/// If the graph is weighted the algorithm runs in $O(m (n^{1+1/k} + n \log n))$ time by using
/// Dijkstra's algorithm. Edge weights must be non-negative.
///
/// # Rust notes
///
/// Greedy `(2k-1)`-spanner (`GreedyMultiplicativeSpanner`): edges are scanned by increasing
/// weight and kept if the spanner so far has no path within `(2k-1)` times the edge weight.
/// Unweighted graphs use hop counts.
pub struct GreedyMultiplicativeSpanner<'g, G: Graph> {
    graph: &'g G,
    k: usize,
}

impl<'g, G: Graph> GreedyMultiplicativeSpanner<'g, G> {
    const MAX_K: usize = 1 << 29;

    /// Constructs instance to compute a $(2k-1)$-spanner of an undirected graph.
    pub fn new(graph: &'g G, k: usize) -> Result<Self> {
        if graph.graph_type().directed {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::Undirected));
        }
        if k == 0 {
            return Err(GraphError::InvalidArgument(
                "k should be positive in (2k-1)-spanner construction".into(),
            ));
        }
        Ok(GreedyMultiplicativeSpanner {
            graph,
            k: k.min(Self::MAX_K),
        })
    }

    /// Computes a graph spanner.
    pub fn spanner(&self) -> Result<Spanner<G::Edge>> {
        let weighted = self.graph.graph_type().weighted;
        let ix = Indexed::new(self.graph);
        let mut order: Vec<usize> = (0..ix.es.len()).collect();
        order.sort_by(|&a, &b| ix.w[a].total_cmp(&ix.w[b]));
        if let Some(&first) = order.first() {
            if ix.w[first] < 0.0 {
                return Err(GraphError::InvalidWeight(crate::WeightProblem::Negative));
            }
        }
        let stretch = (2 * self.k - 1) as f64;
        let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); ix.n()];
        let mut edges = indexmap::IndexSet::new();
        let mut weight = 0.0;
        for e in order {
            let (s, t) = ix.ends[e];
            if s == t {
                continue;
            }
            let w = ix.w[e];
            let reachable = if weighted {
                dijkstra_within(&adj, s, t, stretch * w)
            } else {
                bfs_within(&adj, s, t, stretch * w)
            };
            if !reachable {
                edges.insert(ix.es[e].clone());
                weight += w;
                adj[s].push((t, w));
                adj[t].push((s, w));
            }
        }
        Ok(Spanner { edges, weight })
    }
}

/// Is `t` within `hops` edges of `s`?
fn bfs_within(adj: &[Vec<(usize, f64)>], s: usize, t: usize, hops: f64) -> bool {
    let mut dist = vec![usize::MAX; adj.len()];
    dist[s] = 0;
    let mut queue = std::collections::VecDeque::from([s]);
    while let Some(u) = queue.pop_front() {
        if u == t {
            return dist[u] as f64 <= hops;
        }
        for &(v, _) in &adj[u] {
            if dist[v] == usize::MAX {
                dist[v] = dist[u] + 1;
                queue.push_back(v);
            }
        }
    }
    false
}

/// Is `t` within weighted distance `limit` of `s`?
fn dijkstra_within(adj: &[Vec<(usize, f64)>], s: usize, t: usize, limit: f64) -> bool {
    let mut dist = vec![f64::INFINITY; adj.len()];
    dist[s] = 0.0;
    let mut heap = BinaryHeap::from([Reverse((OrderedF64(0.0), s))]);
    while let Some(Reverse((OrderedF64(d), u))) = heap.pop() {
        if d > limit {
            return false;
        }
        if u == t {
            return true;
        }
        if d > dist[u] {
            continue;
        }
        for &(v, w) in &adj[u] {
            let nd = d + w;
            if nd < dist[v] {
                dist[v] = nd;
                heap.push(Reverse((OrderedF64(nd), v)));
            }
        }
    }
    false
}
