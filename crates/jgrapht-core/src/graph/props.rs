//! Structural predicates (JGraphT's `GraphTests`).

use super::{DirectedExt, DirectedGraph, Graph, GraphExt};
use std::collections::{HashMap, HashSet, VecDeque};

pub fn is_empty<G: Graph>(g: &G) -> bool {
    g.edge_count() == 0
}

pub fn has_self_loops<G: Graph>(g: &G) -> bool {
    g.edges().any(|e| g.edge_source(e) == g.edge_target(e))
}

pub fn has_multiple_edges<G: Graph>(g: &G) -> bool {
    g.edges().any(|e| {
        let (s, t) = g.edge_endpoints(e).unwrap();
        g.edges_between(s, t).nth(1).is_some()
    })
}

pub fn is_simple<G: Graph>(g: &G) -> bool {
    !has_self_loops(g) && !has_multiple_edges(g)
}

/// Weak connectivity (edge direction ignored). The empty graph is not connected.
pub fn is_connected<G: Graph>(g: &G) -> bool {
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

pub fn is_weakly_connected<G: Graph>(g: &G) -> bool {
    is_connected(g)
}

/// Every pair of distinct vertices is adjacent.
pub fn is_complete<G: Graph>(g: &G) -> bool {
    let n = g.vertex_count();
    let vs: Vec<_> = g.vertices().collect();
    if g.graph_type().directed {
        vs.iter()
            .all(|u| vs.iter().all(|v| u == v || g.contains_edge_between(u, v)))
    } else {
        n < 2
            || vs
                .iter()
                .enumerate()
                .all(|(i, u)| vs[i + 1..].iter().all(|v| g.contains_edge_between(u, v)))
    }
}

/// Undirected-sense forest: no cycles when direction is ignored, no parallel edges or loops.
pub fn is_forest<G: Graph>(g: &G) -> bool {
    if has_self_loops(g) {
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
    for e in g.edges() {
        let (s, t) = g.edge_endpoints(e).unwrap();
        let (a, b) = (find(&mut parent, s), find(&mut parent, t));
        if a == b {
            return false;
        }
        parent.insert(a, b);
    }
    true
}

pub fn is_tree<G: Graph>(g: &G) -> bool {
    g.vertex_count() > 0 && g.edge_count() + 1 == g.vertex_count() && is_connected(g)
}

/// Kahn's algorithm.
pub fn is_dag<G: DirectedGraph>(g: &G) -> bool {
    let mut indeg: HashMap<&G::Vertex, usize> =
        g.vertices().map(|v| (v, g.in_degree_of(v))).collect();
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
