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

//! Structural predicates (JGraphT's `GraphTests`).

use super::{DirectedExt, DirectedGraph, Graph, GraphExt, UndirectedGraph};
use crate::graph::{IncidenceGraph, IncomingGraph, OutgoingGraph};
use crate::{GraphError, Result};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet, VecDeque};

/// `Ok(g)` if the graph is weighted, otherwise `InvalidGraph(GraphProperty::Weighted)`.
///
/// # Errors
///
/// Returns a [`GraphError`] if the graph lacks a property the algorithm requires; errors of the underlying graph operations are propagated.
pub fn require_weighted<G: Graph>(g: &G) -> Result<&G> {
    if g.graph_type().weighted {
        Ok(g)
    } else {
        Err(GraphError::InvalidGraph(crate::GraphProperty::Weighted))
    }
}
/// `Ok(g)` if the graph is directed, otherwise `InvalidGraph(GraphProperty::Directed)`.
///
/// # Errors
///
/// Returns a [`GraphError`] if the graph lacks a property the algorithm requires; errors of the underlying graph operations are propagated.
pub fn require_directed<G: Graph>(g: &G) -> Result<&G> {
    if g.graph_type().directed {
        Ok(g)
    } else {
        Err(GraphError::InvalidGraph(crate::GraphProperty::Directed))
    }
}
/// `Ok(g)` if the graph is undirected, otherwise `InvalidGraph(GraphProperty::Undirected)`.
///
/// # Errors
///
/// Returns a [`GraphError`] if the graph lacks a property the algorithm requires; errors of the underlying graph operations are propagated.
pub fn require_undirected<G: Graph>(g: &G) -> Result<&G> {
    if !g.graph_type().directed {
        Ok(g)
    } else {
        Err(GraphError::InvalidGraph(crate::GraphProperty::Undirected))
    }
}

/// No edges (vertices may exist).
pub fn is_empty<G: Graph>(g: &G) -> bool {
    g.edge_count() == 0
}

/// Returns whether there is self loops.
pub fn has_self_loops<G: Graph>(g: &G) -> bool {
    g.graph_type().self_loops
        && g.edges().any(|e| {
            let e = e.borrow();
            g.edge_source(e) == g.edge_target(e)
        })
}

/// Returns whether there is multiple edges.
pub fn has_multiple_edges<G: OutgoingGraph>(g: &G) -> bool {
    if !g.graph_type().multiple_edges {
        return false;
    }
    g.vertices().any(|v| {
        let v = v.borrow();
        {
            let mut seen = HashSet::new();
            g.outgoing_of(v).any(|(_, u)| !seen.insert(u))
        }
    })
}

/// No self-loops and no parallel edges.
pub fn is_simple<G: OutgoingGraph>(g: &G) -> bool {
    if g.graph_type().is_simple() {
        return true;
    }
    g.vertices().all(|v| {
        let v = v.borrow();
        {
            let mut seen = HashSet::new();
            g.outgoing_of(v).all(|(_, u)| u != v && seen.insert(u))
        }
    })
}

/// Simple and every pair of distinct vertices adjacent (in both directions if directed).
pub fn is_complete<G: OutgoingGraph>(g: &G) -> bool {
    let n = g.vertex_count() as u128;
    let all = if g.graph_type().directed {
        n * n.saturating_sub(1)
    } else {
        n * n.saturating_sub(1) / 2
    };
    g.edge_count() as u128 == all && is_simple(g)
}

/// Weak connectivity (edge direction ignored). The empty graph is not connected.
pub fn is_connected<G: IncidenceGraph>(g: &G) -> bool {
    let Some(start) = g.vertices_cloned().next() else {
        return false;
    };
    let mut seen = HashSet::new();
    let mut q = VecDeque::from([start.clone()]);
    seen.insert(start);
    while let Some(x) = q.pop_front() {
        for y in g.neighbors_of(&x) {
            if seen.insert(y.clone()) {
                q.push_back(y.clone());
            }
        }
    }
    seen.len() == g.vertex_count()
}

/// Returns whether weakly connected.
pub fn is_weakly_connected<G: IncidenceGraph>(g: &G) -> bool {
    is_connected(g)
}

/// Strong connectivity (plain connectivity for undirected graphs).
pub fn is_strongly_connected<G: IncidenceGraph>(g: &G) -> bool {
    if !g.graph_type().directed {
        return is_connected(g);
    }
    let Some(start) = g.vertices_cloned().next() else {
        return false;
    };
    let reach = |forward: bool| {
        let mut seen = HashSet::from([start.clone()]);
        let mut q = VecDeque::from([start.clone()]);
        while let Some(x) = q.pop_front() {
            let next: Vec<G::Vertex> = if forward {
                g.outgoing_of(&x).map(|(_, y)| y.clone()).collect()
            } else {
                g.incoming_of(&x).map(|(_, y)| y.clone()).collect()
            };
            for y in next {
                if seen.insert(y.clone()) {
                    q.push_back(y);
                }
            }
        }
        seen.len() == g.vertex_count()
    };
    reach(true) && reach(false)
}

/// Forest: at least one vertex and `|E| + #components == |V|`.
pub fn is_forest<G: Graph>(g: &G) -> bool {
    if g.vertex_count() == 0 {
        return false;
    }
    let mut parent: HashMap<G::Vertex, G::Vertex> =
        g.vertices_cloned().map(|v| (v.clone(), v)).collect();
    fn find<V: std::hash::Hash + Eq + Clone>(p: &mut HashMap<V, V>, x: &V) -> V {
        let mut r = x.clone();
        while p[&r] != r {
            r = p[&r].clone();
        }
        let mut c = x.clone();
        while p[&c] != r {
            let n = p[&c].clone();
            p.insert(c, r.clone());
            c = n;
        }
        r
    }
    let mut components = g.vertex_count();
    for e in g.edges() {
        let e = e.borrow();
        // an edge whose endpoints are missing or not vertices breaks the `Graph` integrity
        // contract; such a graph is not a forest
        let Some((s, t)) = g.edge_endpoints(e) else {
            return false;
        };
        if !parent.contains_key(s) || !parent.contains_key(t) {
            return false;
        }
        let (a, b) = (find(&mut parent, s), find(&mut parent, t));
        if a != b {
            parent.insert(a, b);
            components -= 1;
        }
    }
    g.edge_count() + components == g.vertex_count()
}

/// `|E| == |V| - 1` and connected.
pub fn is_tree<G: IncidenceGraph>(g: &G) -> bool {
    g.edge_count() + 1 == g.vertex_count() && is_connected(g)
}

/// Kahn's algorithm.
pub fn is_dag<G: DirectedGraph + IncomingGraph + OutgoingGraph>(g: &G) -> bool {
    let mut indeg: HashMap<G::Vertex, usize> = g
        .vertices_cloned()
        .map(|v| {
            let d = g.in_degree_of(&v);
            (v, d)
        })
        .collect();
    let mut q: VecDeque<G::Vertex> = indeg
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(v, _)| v.clone())
        .collect();
    let mut n = 0;
    while let Some(v) = q.pop_front() {
        n += 1;
        for s in g.successors_of(&v) {
            // a successor that is not a vertex violates the `Graph` integrity contract; skip it
            let Some(d) = indeg.get_mut(s) else {
                continue;
            };
            *d = d.saturating_sub(1);
            if *d == 0 {
                q.push_back(s.clone());
            }
        }
    }
    n == g.vertex_count()
}

/// More edges than `max_degree * floor(n / 2)` (a sufficient condition for chromatic index
/// `max_degree + 1`). An empty graph is not overfull.
pub fn is_overfull<G: IncidenceGraph>(g: &G) -> bool {
    let Some(max_degree) = g
        .vertices()
        .map(|v| {
            let v = v.borrow();
            g.degree_of(v)
        })
        .max()
    else {
        return false;
    };
    g.edge_count() as f64 > max_degree as f64 * (g.vertex_count() / 2) as f64
}

/// Split graph: the vertices partition into a clique and an independent set
/// (Hammer & Simeone degree-sequence characterisation). Requires a simple undirected graph.
pub fn is_split<G: IncidenceGraph>(g: &G) -> bool {
    if g.graph_type().directed || !is_simple(g) || g.vertex_count() == 0 {
        return false;
    }
    let mut degrees: Vec<usize> = g
        .vertices()
        .map(|v| {
            let v = v.borrow();
            g.degree_of(v)
        })
        .collect();
    degrees.sort_unstable_by(|a, b| b.cmp(a));
    let mut m = 1;
    while m < degrees.len() && degrees[m] >= m {
        m += 1;
    }
    m -= 1;
    let left: usize = degrees[..=m].iter().sum();
    let right: usize = m * (m + 1) + degrees[m + 1..].iter().sum::<usize>();
    left == right
}

/// Every vertex has degree three.
pub fn is_cubic<G: IncidenceGraph>(g: &G) -> bool {
    g.vertices().all(|v| {
        let v = v.borrow();
        g.degree_of(v) == 3
    })
}

/// Ore's condition: simple, undirected, `n >= 3` and `deg(u) + deg(v) >= n` for all
/// non-adjacent `u, v` (sufficient for a Hamiltonian cycle).
pub fn has_ore_property<G: IncidenceGraph>(g: &G) -> bool {
    let n = g.vertex_count();
    if g.graph_type().directed || !g.graph_type().is_simple() || n < 3 {
        return false;
    }
    let vs: Vec<G::Vertex> = g.vertices_cloned().collect();
    for (i, v) in vs.iter().enumerate() {
        for w in &vs[i + 1..] {
            if !g.contains_edge_between(v, w) && g.degree_of(v) + g.degree_of(w) < n {
                return false;
            }
        }
    }
    true
}

/// Contains no triangle.
pub fn is_triangle_free<G: Graph>(g: &G) -> bool {
    crate::alg::scoring::count_triangles(g) == 0
}

/// Returns whether bipartite.
pub fn is_bipartite<G: Graph>(g: &G) -> bool {
    crate::alg::partition::is_bipartite(g)
}

/// Whether `first` and `second` are the two sides of a bipartition of `g`.
pub fn is_bipartite_partition<G: IncidenceGraph>(
    g: &G,
    first: &std::collections::HashSet<G::Vertex>,
    second: &std::collections::HashSet<G::Vertex>,
) -> bool {
    use crate::alg::interfaces::{Partitioning, PartitioningAlgorithm};
    let side = |side: &HashSet<G::Vertex>| -> indexmap::IndexSet<G::Vertex> {
        g.vertices_cloned()
            .filter(|v| side.contains(v))
            .chain(side.iter().filter(|v| !g.contains_vertex(v)).cloned())
            .collect()
    };
    let p = Partitioning::new(vec![side(first), side(second)]);
    crate::alg::partition::BipartitePartitioning::new(g).is_valid_partitioning(&p)
}

/// Returns whether biconnected.
pub fn is_biconnected<G: IncidenceGraph>(g: &G) -> bool {
    crate::alg::connectivity::is_biconnected(g)
}

/// Returns whether eulerian.
pub fn is_eulerian<G: IncidenceGraph>(g: &G) -> bool {
    crate::alg::cycle::HierholzerEulerianCycle::new().is_eulerian(g)
}

/// Returns whether chordal.
pub fn is_chordal<G: UndirectedGraph + IncidenceGraph>(g: &G) -> bool {
    crate::alg::cycle::ChordalityInspector::new(g).is_chordal()
}

/// No induced cycle of length five or more in the graph or its complement.
pub fn is_weakly_chordal<G: IncidenceGraph>(g: &G) -> bool {
    crate::alg::holes::is_weakly_chordal(g)
}

/// Perfect graph (Berge): no odd hole and no odd antihole.
pub fn is_perfect<G: Graph>(g: &G) -> bool {
    crate::alg::holes::is_perfect(g)
}

/// Returns whether planar.
pub fn is_planar<G: IncidenceGraph>(g: &G) -> bool {
    crate::alg::planar::BoyerMyrvoldPlanarityInspector::new(g).is_planar()
}

/// Vertices of degree `degree` reachable from `start` without passing through another vertex of
/// that degree.
fn reachable_with_degree<G: IncidenceGraph>(
    g: &G,
    start: &G::Vertex,
    degree: usize,
) -> HashSet<G::Vertex> {
    let mut visited = HashSet::new();
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::from([start.clone()]);
    while let Some(cur) = queue.pop_front() {
        visited.insert(cur.clone());
        for e in g.edges_of(&cur) {
            let Some(opp) = g.opposite_vertex(e, &cur) else {
                continue;
            };
            if visited.contains(opp) {
                continue;
            }
            if g.degree_of(opp) == degree {
                reachable.insert(opp.clone());
            } else {
                queue.push_back(opp.clone());
            }
        }
    }
    reachable
}

/// A subdivision of K3,3 (all vertices have degree 2 or 3, six of degree 3, forming K3,3).
pub fn is_k33_subdivision<G: IncidenceGraph>(g: &G) -> bool {
    let mut degree3: Vec<G::Vertex> = Vec::new();
    for v in g.vertices() {
        let v = v.borrow();
        match g.degree_of(v) {
            3 => degree3.push(v.clone()),
            2 => {}
            _ => return false,
        }
    }
    if degree3.len() != 6 {
        return false;
    }
    let Some(last) = degree3.pop() else {
        return false;
    };
    let reachable = reachable_with_degree(g, &last, 3);
    if reachable.len() != 3 {
        return false;
    }
    degree3.retain(|v| !reachable.contains(v));
    let [a, b] = &degree3[..] else {
        return false;
    };
    reachable == reachable_with_degree(g, a, 3) && reachable == reachable_with_degree(g, b, 3)
}

/// A subdivision of K5.
pub fn is_k5_subdivision<G: IncidenceGraph>(g: &G) -> bool {
    let mut deg4 = HashSet::new();
    for v in g.vertices() {
        let v = v.borrow();
        match g.degree_of(v) {
            4 => {
                deg4.insert(v.clone());
            }
            2 => {}
            _ => return false,
        }
    }
    if deg4.len() != 5 {
        return false;
    }
    deg4.iter().all(|v| {
        let r = reachable_with_degree(g, v, 4);
        r.len() == 4 && r.iter().all(|x| deg4.contains(x)) && !r.contains(v)
    })
}

/// Returns whether kuratowski subdivision.
pub fn is_kuratowski_subdivision<G: IncidenceGraph>(g: &G) -> bool {
    is_k33_subdivision(g) || is_k5_subdivision(g)
}
