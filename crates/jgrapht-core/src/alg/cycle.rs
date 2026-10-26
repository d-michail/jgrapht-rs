//! Cycle detection, enumeration and bases.

use super::util::Indexed;
use crate::graph::{Graph, props};
use crate::walk::GraphWalk;
use std::collections::{HashSet, VecDeque};

/// Whether the graph contains a cycle (loops and parallel undirected edges count).
pub fn has_cycle<G: Graph>(g: &G) -> bool {
    if g.graph_type().directed {
        let ix = Indexed::new(g);
        let mut indeg: Vec<usize> = ix.inn.iter().map(Vec::len).collect();
        let mut q: VecDeque<usize> = (0..ix.n()).filter(|&v| indeg[v] == 0).collect();
        let mut seen = 0;
        while let Some(v) = q.pop_front() {
            seen += 1;
            for &(u, _) in &ix.out[v] {
                indeg[u] -= 1;
                if indeg[u] == 0 {
                    q.push_back(u);
                }
            }
        }
        seen != ix.n()
    } else {
        !props::is_forest(g)
    }
}

fn circuit(
    v: usize,
    s: usize,
    adj: &[Vec<usize>],
    blocked: &mut [bool],
    b: &mut [Vec<usize>],
    stack: &mut Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) -> bool {
    fn unblock(u: usize, blocked: &mut [bool], b: &mut [Vec<usize>]) {
        blocked[u] = false;
        while let Some(w) = b[u].pop() {
            if blocked[w] {
                unblock(w, blocked, b);
            }
        }
    }
    let mut found = false;
    stack.push(v);
    blocked[v] = true;
    for &w in adj[v].iter().filter(|&&w| w >= s) {
        if w == s {
            out.push(stack.clone());
            found = true;
        } else if !blocked[w] && circuit(w, s, adj, blocked, b, stack, out) {
            found = true;
        }
    }
    if found {
        unblock(v, blocked, b);
    } else {
        for &w in adj[v].iter().filter(|&&w| w >= s) {
            if !b[w].contains(&v) {
                b[w].push(v);
            }
        }
    }
    stack.pop();
    found
}

/// All simple cycles (Johnson's algorithm), each as its vertex list. In undirected graphs each
/// cycle (length >= 3) is reported once; parallel edges are treated as one edge.
pub fn simple_cycles<G: Graph>(g: &G) -> Vec<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let adj: Vec<Vec<usize>> = (0..n)
        .map(|v| {
            let mut seen = HashSet::new();
            ix.out[v]
                .iter()
                .map(|&(u, _)| u)
                .filter(|u| seen.insert(*u))
                .collect()
        })
        .collect();
    let mut out = Vec::new();
    for s in 0..n {
        let mut blocked = vec![false; n];
        let mut b = vec![Vec::new(); n];
        circuit(s, s, &adj, &mut blocked, &mut b, &mut Vec::new(), &mut out);
    }
    if !ix.directed {
        out.retain(|c| c.len() >= 3 && c[1] < c[c.len() - 1]);
    }
    out.iter().map(|c| ix.vertices_of(c)).collect()
}

/// Fundamental cycle basis of an undirected graph (direction ignored): one cycle, given as its
/// edges, per non-tree edge of a BFS spanning forest.
pub fn cycle_basis<G: Graph>(g: &G) -> Vec<Vec<G::Edge>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let (mut depth, mut parent) = (vec![usize::MAX; n], vec![(usize::MAX, usize::MAX); n]);
    let mut tree = vec![false; ix.es.len()];
    for r in 0..n {
        if depth[r] != usize::MAX {
            continue;
        }
        depth[r] = 0;
        let mut q = VecDeque::from([r]);
        while let Some(v) = q.pop_front() {
            for &(u, k) in ix.out[v].iter().chain(&ix.inn[v]) {
                if depth[u] == usize::MAX {
                    depth[u] = depth[v] + 1;
                    parent[u] = (v, k);
                    tree[k] = true;
                    q.push_back(u);
                }
            }
        }
    }
    let mut basis = Vec::new();
    for (k, &(s, t)) in ix.ends.iter().enumerate() {
        if tree[k] {
            continue;
        }
        let (mut a, mut b) = (s, t);
        let (mut left, mut right) = (Vec::new(), Vec::new());
        while a != b {
            if depth[a] >= depth[b] {
                left.push(parent[a].1);
                a = parent[a].0;
            } else {
                right.push(parent[b].1);
                b = parent[b].0;
            }
        }
        let mut cycle = vec![k];
        cycle.extend(right);
        cycle.extend(left.into_iter().rev());
        basis.push(cycle.into_iter().map(|e| ix.es[e].clone()).collect());
    }
    basis
}

/// A cycle through an edge-list as a closed walk, if the edges are consecutive.
pub fn cycle_to_walk<G: Graph>(
    g: &G,
    start: G::Vertex,
    edges: Vec<G::Edge>,
) -> Option<GraphWalk<G::Vertex, G::Edge>> {
    let w = edges.iter().filter_map(|e| g.edge_weight(e)).sum();
    GraphWalk::from_edges(g, start, edges, w)
}
