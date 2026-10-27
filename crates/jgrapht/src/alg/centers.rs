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

//! Facility location: k-center.

use super::interfaces::CentersLocationAlgorithm;
use super::util::Indexed;
use crate::graph::{Graph, props};
use crate::util::OrderedF64;
use crate::util::java::RandomSource;
use crate::{GraphError, Result};
use indexmap::{IndexMap, IndexSet};
use rand::rngs::StdRng;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Gonzalez' farthest-first traversal: a 2-approximation of the k-center problem under
/// shortest-path distances. Returns the centers and the covering radius.
pub fn greedy_k_center<G: Graph>(g: &G, k: usize, weighted: bool) -> Result<(Vec<G::Vertex>, f64)> {
    let ix = Indexed::new(g);
    let n = ix.n();
    if k == 0 || k > n {
        return Err(GraphError::InvalidArgument(
            "k must be between 1 and the number of vertices".into(),
        ));
    }
    let dists = |s: usize| -> Vec<f64> {
        let mut dist = vec![f64::INFINITY; n];
        dist[s] = 0.0;
        let mut heap = BinaryHeap::from([Reverse((OrderedF64(0.0), s))]);
        while let Some(Reverse((OrderedF64(d), v))) = heap.pop() {
            if d > dist[v] {
                continue;
            }
            for &(u, e) in &ix.out[v] {
                let nd = d + if weighted { ix.w[e] } else { 1.0 };
                if nd < dist[u] {
                    dist[u] = nd;
                    heap.push(Reverse((OrderedF64(nd), u)));
                }
            }
        }
        dist
    };
    let mut centers = vec![0];
    let mut nearest = dists(0);
    while centers.len() < k {
        let far = (0..n)
            .max_by(|&a, &b| nearest[a].total_cmp(&nearest[b]))
            .unwrap();
        centers.push(far);
        let d = dists(far);
        for v in 0..n {
            nearest[v] = nearest[v].min(d[v]);
        }
    }
    let radius = nearest.iter().copied().fold(0.0, f64::max);
    Ok((ix.vertices_of(&centers), radius))
}

/// The Gon heuristic algorithm for the vertex $k$-center problem.
///
/// The vertex $k$-center problem is an NP-hard combinatorial optimization
/// problem that receives a complete edge-weighted undirected graph $G = (V, E,
/// w)$, and a positive integer $k$. The goal is to find a subset $C$ of $V$ such
/// that $|C| = k$ and the maximum distance from any vertex in $V$ to the nearest
/// vertex in $C$ is minimized. $C$ is called the set of centers. The vertex
/// $k$-center problem has applications in clustering and facility location.
///
/// The Gon heuristic is a classic heuristic approximation algorithm for the
/// vertex $k$-center problem. It works in a straightforward way. First, a vertex
/// from the input graph is chosen randomly and added to the set of centers $C$.
/// Then, iteratively, the farthest vertex from $V$ to $C$ is chosen and added to
/// $C$. This process is repeated until $|C| = k$.
///
/// This algorithm provides a guarantee to compute solutions for the vertex
/// $k$-center problem no more than 2-times optimum. According to the literature,
/// this is the best approximation factor (under $P \neq NP$). The implementation
/// chooses the first vertex randomly. Alternatively, an existing set of centers
/// $C$ with fewer than $k$ centers can be provided to be augmented. In this
/// implementation, ties are broken by choosing the vertex with the lowest index.
///
/// The description of this algorithm can be consulted on:
///
/// T. F. Gonzalez Clustering to minimize the maximum intercluster distance.
/// Theor. Comput. Sci. 1985, 38, 293-306.
///
/// J. Garcia-Diaz, R. Menchaca-Mendez, R. Menchaca-Mendez, S. Pomares Hernández,
/// J. C. Pérez-Sansalvador and N. Lakouari, "Approximation Algorithms for the
/// Vertex K-Center Problem: Survey and Experimental Evaluation," in IEEE Access,
/// vol. 7, pp. 109228-109245, 2019, doi: 10.1109/ACCESS.2019.2933875.
///
/// This implementation can also be used to augment an existing partial set of
/// centers. See constructor `GonHeuristic(Set)`.
///
/// The runtime complexity of the Gon algorithm is $O(k\cdot |V|)$.
///
/// This algorithm requires that the graph is complete, undirected, and
/// edge-weighted.
///
/// # Rust notes
///
/// Gonzalez' farthest-first heuristic on a *complete* undirected graph whose edge weights are the
/// distances (`GonHeuristic`): a 2-approximation of the k-center problem.
///
/// Like in JGraphT, the initial center chosen on the first call is kept for later calls.
pub struct GonHeuristic<V: Eq + Hash + Clone, R = StdRng> {
    initial_centers: Option<HashSet<V>>,
    rng: Option<R>,
    assignment: Option<IndexMap<V, V>>,
    covering_radius: f64,
}

impl<V: Eq + Hash + Clone> GonHeuristic<V, StdRng> {
    /// The first center is the given one(s); with an empty set a random vertex is used.
    pub fn with_initial_centers(initial_centers: HashSet<V>) -> Self {
        GonHeuristic {
            initial_centers: Some(initial_centers),
            rng: None,
            assignment: None,
            covering_radius: 0.0,
        }
    }
}

impl<V: Eq + Hash + Clone, R: RandomSource> GonHeuristic<V, R> {
    /// The first center is chosen with `rng`.
    pub fn with_rng(rng: R) -> Self {
        GonHeuristic {
            initial_centers: None,
            rng: Some(rng),
            assignment: None,
            covering_radius: 0.0,
        }
    }
}

fn check_graph<G: Graph>(graph: &G) -> Result<()> {
    props::require_undirected(graph)?;
    if graph.vertex_count() == 0 {
        return Err(GraphError::InvalidArgument(
            "Graph contains no vertices".into(),
        ));
    }
    if !props::is_complete(graph) {
        return Err(GraphError::InvalidArgument("Graph is not complete".into()));
    }
    Ok(())
}

impl<V: Eq + Hash + Clone, R: RandomSource> CentersLocationAlgorithm<V> for GonHeuristic<V, R> {
    fn centers<G: Graph<Vertex = V>>(&mut self, graph: &G, k: usize) -> Result<IndexSet<V>> {
        check_graph(graph)?;
        let n = graph.vertex_count();
        if n == k {
            return Ok(graph.vertices().cloned().collect());
        }
        if n < k {
            return Err(GraphError::InvalidArgument(
                "The number of vertices in the graph must be at least k".into(),
            ));
        }
        if k == 0 {
            return Err(GraphError::InvalidArgument("k must be at least 1".into()));
        }
        if self.initial_centers.as_ref().is_none_or(|c| c.is_empty()) {
            // no initial set of centers: choose the first center randomly
            let rng = self.rng.as_mut().ok_or_else(|| {
                GraphError::InvalidArgument("no random source and no initial centers".into())
            })?;
            let v = graph.vertices().nth(rng.next_int_bound(n)).unwrap().clone();
            self.initial_centers = Some(HashSet::from([v]));
        }
        let weight = |a: &V, b: &V| -> f64 {
            graph
                .get_edge(a, b)
                .and_then(|e| graph.edge_weight(e))
                .unwrap_or(1.0)
        };
        let mut centers: IndexSet<V> = graph
            .vertices()
            .filter(|v| self.initial_centers.as_ref().unwrap().contains(*v))
            .cloned()
            .collect();
        let mut clients: IndexSet<V> = graph
            .vertices()
            .filter(|v| !centers.contains(*v))
            .cloned()
            .collect();
        // distance from each client to the set of centers
        let mut distances: HashMap<V, f64> = clients
            .iter()
            .map(|c| {
                let d = centers
                    .iter()
                    .map(|x| weight(c, x))
                    .fold(f64::MAX, f64::min);
                (c.clone(), d)
            })
            .collect();
        while centers.len() < k {
            // the farthest client; the first one wins ties
            let mut far: Option<&V> = None;
            for c in &clients {
                if far.is_none_or(|f| distances[c] > distances[f]) {
                    far = Some(c);
                }
            }
            let v = far.unwrap().clone();
            clients.shift_remove(&v);
            centers.insert(v.clone());
            for c in &clients {
                let w = weight(c, &v);
                let d = distances.get_mut(c).unwrap();
                if w < *d {
                    *d = w;
                }
            }
        }
        // assign the clients to their closest centers
        let mut assignment = IndexMap::new();
        let mut radius = 0.0f64;
        for client in &clients {
            let mut closest: Option<&V> = None;
            let mut min_dist = f64::MAX;
            for center in &centers {
                let w = weight(client, center);
                if w < min_dist {
                    min_dist = w;
                    closest = Some(center);
                }
            }
            assignment.insert(client.clone(), closest.unwrap().clone());
            radius = radius.max(min_dist);
        }
        self.assignment = Some(assignment);
        self.covering_radius = radius;
        Ok(centers)
    }

    fn assignment(&self) -> Result<&IndexMap<V, V>> {
        self.assignment.as_ref().ok_or_else(|| {
            GraphError::Unsupported("You must call centers() before calling assignment()".into())
        })
    }

    fn covering_radius(&self) -> Result<f64> {
        if self.assignment.is_none() {
            return Err(GraphError::Unsupported(
                "You must call centers() before calling covering_radius()".into(),
            ));
        }
        Ok(self.covering_radius)
    }
}
