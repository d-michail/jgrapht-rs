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

use crate::graph::{IncidenceGraph, IncomingGraph, OutgoingGraph};
use super::{DirectedExt, DirectedGraph, Graph, GraphExt, UndirectedGraph};
use crate::{GraphError, Result};
use std::collections::{HashMap, HashSet, VecDeque};


/// `Ok(g)` if the graph is weighted, otherwise an error carrying JGraphT's message.
pub fn require_weighted<G: Graph>(g: &G) -> Result<&G> {
    if g.graph_type().weighted {
        Ok(g)
    } else {
        Err(GraphError::InvalidGraph(crate::GraphProperty::Weighted))
    }
}
pub fn require_directed<G: Graph>(g: &G) -> Result<&G> {
    if g.graph_type().directed {
        Ok(g)
    } else {
        Err(GraphError::InvalidGraph(crate::GraphProperty::Directed))
    }
}
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

pub fn has_self_loops<G: Graph>(g: &G) -> bool {
    g.graph_type().self_loops && g.edges().any(|e| g.edge_source(e) == g.edge_target(e))
}

pub fn has_multiple_edges<G: OutgoingGraph>(g: &G) -> bool {
    if !g.graph_type().multiple_edges {
        return false;
    }
    g.vertices().any(|v| {
        let mut seen = HashSet::new();
        g.outgoing_of(v).any(|(_, u)| !seen.insert(u))
    })
}

/// No self-loops and no parallel edges.
pub fn is_simple<G: OutgoingGraph>(g: &G) -> bool {
    if g.graph_type().is_simple() {
        return true;
    }
    g.vertices().all(|v| {
        let mut seen = HashSet::new();
        g.outgoing_of(v).all(|(_, u)| u != v && seen.insert(u))
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
    let Some(start) = g.vertices().next() else {
        return false;
    };
    let mut seen = HashSet::new();
    let mut q = VecDeque::from([start]);
    seen.insert(start);
    while let Some(x) = q.pop_front() {
        for y in g.neighbors_of(x) {
            if seen.insert(y) {
                q.push_back(y);
            }
        }
    }
    seen.len() == g.vertex_count()
}

pub fn is_weakly_connected<G: IncidenceGraph>(g: &G) -> bool {
    is_connected(g)
}

/// Strong connectivity (plain connectivity for undirected graphs).
pub fn is_strongly_connected<G: IncidenceGraph>(g: &G) -> bool {
    if !g.graph_type().directed {
        return is_connected(g);
    }
    let Some(start) = g.vertices().next() else {
        return false;
    };
    let reach = |forward: bool| {
        let mut seen = HashSet::from([start]);
        let mut q = VecDeque::from([start]);
        while let Some(x) = q.pop_front() {
            let next: Vec<_> = if forward {
                g.outgoing_of(x).map(|(_, y)| y).collect()
            } else {
                g.incoming_of(x).map(|(_, y)| y).collect()
            };
            for y in next {
                if seen.insert(y) {
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
    let mut parent: HashMap<&G::Vertex, &G::Vertex> = g.vertices().map(|v| (v, v)).collect();
    fn find<'a, V: std::hash::Hash + Eq>(p: &mut HashMap<&'a V, &'a V>, x: &'a V) -> &'a V {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        let mut c = x;
        while p[c] != r {
            let n = p[c];
            p.insert(c, r);
            c = n;
        }
        r
    }
    let mut components = g.vertex_count();
    for e in g.edges() {
        let (s, t) = g.edge_endpoints(e).unwrap();
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
    let mut indeg: HashMap<&G::Vertex, usize> = g
        .vertices()
        .map(|v| (v, g.in_degree_of(v)))
        .collect();
    let mut q: VecDeque<_> = indeg
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(v, _)| *v)
        .collect();
    let mut n = 0;
    while let Some(v) = q.pop_front() {
        n += 1;
        for s in g.successors_of(v) {
            let d = indeg.get_mut(s).unwrap();
            *d -= 1;
            if *d == 0 {
                q.push_back(s);
            }
        }
    }
    n == g.vertex_count()
}

/// More edges than `max_degree * floor(n / 2)` (a sufficient condition for chromatic index
/// `max_degree + 1`). Panics-free: an empty graph is not overfull.
pub fn is_overfull<G: IncidenceGraph>(g: &G) -> bool {
    let Some(max_degree) = g.vertices().map(|v| g.degree_of(v)).max() else {
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
    let mut degrees: Vec<usize> = g.vertices().map(|v| g.degree_of(v)).collect();
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
    g.vertices().all(|v| g.degree_of(v) == 3)
}

/// Ore's condition: simple, undirected, `n >= 3` and `deg(u) + deg(v) >= n` for all
/// non-adjacent `u, v` (sufficient for a Hamiltonian cycle).
pub fn has_ore_property<G: IncidenceGraph>(g: &G) -> bool {
    let n = g.vertex_count();
    if g.graph_type().directed || !g.graph_type().is_simple() || n < 3 {
        return false;
    }
    let vs: Vec<_> = g.vertices().collect();
    for (i, v) in vs.iter().enumerate() {
        for w in &vs[i + 1..] {
            if !g.contains_edge_between(v, w)
                && g.degree_of(v) + g.degree_of(w) < n
            {
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
    let p = Partitioning::new(vec![
        g.vertices()
            .filter(|v| first.contains(*v))
            .cloned()
            .chain(first.iter().filter(|v| !g.contains_vertex(v)).cloned())
            .collect(),
        g.vertices()
            .filter(|v| second.contains(*v))
            .cloned()
            .chain(second.iter().filter(|v| !g.contains_vertex(v)).cloned())
            .collect(),
    ]);
    crate::alg::partition::BipartitePartitioning::new(g).is_valid_partitioning(&p)
}

pub fn is_biconnected<G: IncidenceGraph>(g: &G) -> bool {
    crate::alg::connectivity::is_biconnected(g)
}

pub fn is_eulerian<G: IncidenceGraph>(g: &G) -> bool {
    crate::alg::cycle::HierholzerEulerianCycle::new().is_eulerian(g)
}

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

pub fn is_planar<G: IncidenceGraph>(g: &G) -> bool {
    crate::alg::planar::BoyerMyrvoldPlanarityInspector::new(g).is_planar()
}

/// Vertices of degree `degree` reachable from `start` without passing through another vertex of
/// that degree.
fn reachable_with_degree<'a, G: IncidenceGraph>(
    g: &'a G,
    start: &'a G::Vertex,
    degree: usize,
) -> HashSet<&'a G::Vertex> {
    let mut visited = HashSet::new();
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::from([start]);
    while let Some(cur) = queue.pop_front() {
        visited.insert(cur);
        for e in g.edges_of(cur) {
            let Some(opp) = g.opposite_vertex(e, cur) else {
                continue;
            };
            if visited.contains(opp) {
                continue;
            }
            if g.degree_of(opp) == degree {
                reachable.insert(opp);
            } else {
                queue.push_back(opp);
            }
        }
    }
    reachable
}

/// A subdivision of K3,3 (all vertices have degree 2 or 3, six of degree 3, forming K3,3).
pub fn is_k33_subdivision<G: IncidenceGraph>(g: &G) -> bool {
    let mut degree3 = Vec::new();
    for v in g.vertices() {
        match g.degree_of(v) {
            3 => degree3.push(v),
            2 => {}
            _ => return false,
        }
    }
    if degree3.len() != 6 {
        return false;
    }
    let last = degree3.pop().unwrap();
    let reachable = reachable_with_degree(g, last, 3);
    if reachable.len() != 3 {
        return false;
    }
    degree3.retain(|v| !reachable.contains(v));
    reachable == reachable_with_degree(g, degree3[0], 3)
        && reachable == reachable_with_degree(g, degree3[1], 3)
}

/// A subdivision of K5.
pub fn is_k5_subdivision<G: IncidenceGraph>(g: &G) -> bool {
    let mut deg4 = HashSet::new();
    for v in g.vertices() {
        match g.degree_of(v) {
            4 => {
                deg4.insert(v);
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

pub fn is_kuratowski_subdivision<G: IncidenceGraph>(g: &G) -> bool {
    is_k33_subdivision(g) || is_k5_subdivision(g)
}
