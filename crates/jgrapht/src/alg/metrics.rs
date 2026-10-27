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

//! Graph metrics (JGraphT's `GraphMetrics` and `GraphMeasurer`): eccentricity, diameter, radius,
//! center, periphery, girth, triangle counts. Distances are weighted shortest-path distances
//! (all `1.0` for unweighted graphs).

use super::shortestpath::floyd_warshall;
use super::util::Indexed;
use crate::graph::Graph;
use std::collections::{HashMap, HashSet, VecDeque};

const TOLERANCE: f64 = 1e-9;

fn same(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= TOLERANCE * a.abs().max(b.abs()).max(1.0)
}

/// Algorithm class which computes a number of distance related metrics. A summary of various
/// distance metrics can be found
/// [here](https://en.wikipedia.org/wiki/Distance_(graph_theory)).
///
/// # Rust notes
///
/// Computes eccentricities once (via Floyd–Warshall) and derives diameter, radius and the
/// center/periphery vertex sets.
pub struct GraphMeasurer<V> {
    vertices: Vec<V>,
    eccentricity: Vec<f64>,
    pair_weight: Vec<Vec<f64>>,
    diameter: f64,
    radius: f64,
}

impl<V: Clone + Eq + std::hash::Hash> GraphMeasurer<V> {
    /// Constructs a new instance of GraphMeasurer. `FloydWarshallShortestPaths` is used as the
    /// default shortest path algorithm.
    pub fn new<G: Graph<Vertex = V>>(g: &G) -> crate::Result<Self> {
        let ap = floyd_warshall(g)?;
        let vertices: Vec<V> = g.vertices().cloned().collect();
        let n = vertices.len();
        let pair_weight: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| ap.weight(&vertices[i], &vertices[j]))
                    .collect()
            })
            .collect();
        let directed = g.graph_type().directed;
        let mut ecc = vec![0.0f64; n];
        for i in 0..n {
            for j in 0..n {
                if i != j && (directed || i < j) {
                    ecc[i] = ecc[i].max(pair_weight[i][j]);
                    if !directed {
                        ecc[j] = ecc[j].max(pair_weight[i][j]);
                    }
                }
            }
        }
        let (diameter, radius) = if n == 0 {
            (0.0, 0.0)
        } else {
            (
                ecc.iter().copied().fold(0.0, f64::max),
                ecc.iter().copied().fold(f64::INFINITY, f64::min),
            )
        };
        Ok(GraphMeasurer {
            vertices,
            eccentricity: ecc,
            pair_weight,
            diameter,
            radius,
        })
    }
    /// Compute the [diameter](http://mathworld.wolfram.com/GraphDiameter.html) of the
    /// graph. The diameter of a graph is defined as $\max_{v\in V}\epsilon(v)$, where $\epsilon(v)$
    /// is the eccentricity of vertex $v$. In other words, this method computes the 'longest shortest
    /// path'. Two special cases exist. If the graph has no vertices, the diameter is 0. If the graph
    /// is disconnected, the diameter is `Double#POSITIVE_INFINITY`.
    pub fn diameter(&self) -> f64 {
        self.diameter
    }
    /// Compute the [radius](http://mathworld.wolfram.com/GraphRadius.html) of the graph.
    /// The radius of a graph is defined as $\min_{v\in V}\epsilon(v)$, where $\epsilon(v)$ is the
    /// eccentricity of vertex $v$. Two special cases exist. If the graph has no vertices, the radius
    /// is 0. If the graph is disconnected, the diameter is `Double#POSITIVE_INFINITY`.
    pub fn radius(&self) -> f64 {
        self.radius
    }
    /// Eccentricity of every vertex, in vertex order.
    pub fn vertex_eccentricity_map(&self) -> Vec<(V, f64)> {
        self.vertices
            .iter()
            .cloned()
            .zip(self.eccentricity.iter().copied())
            .collect()
    }
    /// Vertices whose eccentricity equals the radius.
    pub fn graph_center(&self) -> Vec<V> {
        self.select(|i| same(self.eccentricity[i], self.radius))
    }
    /// Vertices whose eccentricity equals the diameter.
    pub fn graph_periphery(&self) -> Vec<V> {
        self.select(|i| same(self.eccentricity[i], self.diameter))
    }
    /// Vertices `u` that have some `v` with `d(u, v) = ecc(u) = ecc(v)`.
    pub fn graph_pseudo_periphery(&self) -> Vec<V> {
        self.select(|u| {
            (0..self.vertices.len()).any(|v| {
                same(self.pair_weight[u][v], self.eccentricity[u])
                    && same(self.eccentricity[u], self.eccentricity[v])
            })
        })
    }
    fn select(&self, f: impl Fn(usize) -> bool) -> Vec<V> {
        (0..self.vertices.len())
            .filter(|&i| f(i))
            .map(|i| self.vertices[i].clone())
            .collect()
    }
}

/// Eccentricities (`INFINITY` if some vertex is unreachable).
pub fn eccentricities<G: Graph>(g: &G) -> HashMap<G::Vertex, f64> {
    GraphMeasurer::new(g)
        .map(|m| m.vertex_eccentricity_map().into_iter().collect())
        .unwrap_or_default()
}

/// Largest eccentricity; `0.0` for the empty graph.
pub fn diameter<G: Graph>(g: &G) -> f64 {
    GraphMeasurer::new(g).map_or(f64::NEG_INFINITY, |m| m.diameter())
}

/// Smallest eccentricity; `0.0` for the empty graph.
pub fn radius<G: Graph>(g: &G) -> f64 {
    GraphMeasurer::new(g).map_or(f64::NEG_INFINITY, |m| m.radius())
}

pub fn center<G: Graph>(g: &G) -> Vec<G::Vertex> {
    GraphMeasurer::new(g)
        .map(|m| m.graph_center())
        .unwrap_or_default()
}

pub fn periphery<G: Graph>(g: &G) -> Vec<G::Vertex> {
    GraphMeasurer::new(g)
        .map(|m| m.graph_periphery())
        .unwrap_or_default()
}

/// Length of the shortest cycle, `None` for acyclic graphs. Self-loops give 1; parallel edges in
/// undirected graphs and 2-cycles in directed graphs give 2.
pub fn girth<G: Graph>(g: &G) -> Option<usize> {
    let ix = Indexed::new(g);
    let n = ix.n();
    if ix.ends.iter().any(|&(a, b)| a == b) {
        return Some(1);
    }
    let mut best = usize::MAX;
    if !ix.directed {
        for s in 0..n {
            let (mut dist, mut parent_edge) = (vec![usize::MAX; n], vec![usize::MAX; n]);
            dist[s] = 0;
            let mut q = VecDeque::from([s]);
            while let Some(v) = q.pop_front() {
                if 2 * dist[v] + 1 >= best {
                    break;
                }
                for &(u, k) in &ix.out[v] {
                    if k == parent_edge[v] {
                        continue;
                    }
                    if dist[u] == usize::MAX {
                        dist[u] = dist[v] + 1;
                        parent_edge[u] = k;
                        q.push_back(u);
                    } else {
                        best = best.min(dist[u] + dist[v] + 1);
                    }
                }
            }
        }
    } else {
        for s in 0..n {
            let mut dist = vec![usize::MAX; n];
            dist[s] = 0;
            let mut q = VecDeque::from([s]);
            'bfs: while let Some(v) = q.pop_front() {
                if dist[v] + 1 >= best {
                    break;
                }
                for &(u, _) in &ix.out[v] {
                    if u == s {
                        best = best.min(dist[v] + 1);
                        break 'bfs;
                    }
                    if dist[u] == usize::MAX {
                        dist[u] = dist[v] + 1;
                        q.push_back(u);
                    }
                }
            }
        }
    }
    (best != usize::MAX).then_some(best)
}

/// Triangle count of an undirected graph; with parallel edges every combination of one edge per
/// side counts (`GraphMetrics.getNumberOfTriangles`). Self-loops are ignored.
pub fn number_of_triangles<G: Graph>(g: &G) -> u64 {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut mult: HashMap<(usize, usize), u64> = HashMap::new();
    for &(a, b) in &ix.ends {
        if a != b {
            *mult.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    let mut higher: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    for &(a, b) in mult.keys() {
        higher[a].insert(b);
    }
    let mut total = 0;
    for (&(u, v), &m1) in &mult {
        for &w in &higher[v] {
            if let (Some(&m2), Some(&m3)) = (mult.get(&(v, w)), mult.get(&(u, w))) {
                total += m1 * m2 * m3;
            }
        }
    }
    total
}

/// Cubic-time reference triangle count over a vertex subset, for testing.
pub fn naive_count_triangles<G: Graph>(g: &G, subset: &[G::Vertex]) -> u64 {
    let count = |a: &G::Vertex, b: &G::Vertex| g.edges_between(a, b).count() as u64;
    let multi = g.graph_type().multiple_edges;
    let mut total = 0;
    for i in 0..subset.len() {
        for j in i + 1..subset.len() {
            for k in j + 1..subset.len() {
                let (u, v, w) = (&subset[i], &subset[j], &subset[k]);
                let (x, y, z) = (count(u, v), count(v, w), count(w, u));
                total += if multi {
                    x * y * z
                } else {
                    (x > 0 && y > 0 && z > 0) as u64
                };
            }
        }
    }
    total
}
