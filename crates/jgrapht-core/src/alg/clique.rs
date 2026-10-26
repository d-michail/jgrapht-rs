//! Cliques and independent sets.

use super::util::Indexed;
use crate::graph::Graph;
use std::collections::HashSet;

fn adjacency<G: Graph>(ix: &Indexed<'_, G>) -> Vec<HashSet<usize>> {
    (0..ix.n())
        .map(|v| {
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v)
                .collect()
        })
        .collect()
}

fn bron_kerbosch(
    adj: &[HashSet<usize>],
    r: &mut Vec<usize>,
    mut p: Vec<usize>,
    mut x: Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    if p.is_empty() {
        if x.is_empty() {
            out.push(r.clone());
        }
        return;
    }
    // pivot maximising |P ∩ N(u)|
    let pivot = p
        .iter()
        .chain(&x)
        .copied()
        .max_by_key(|&u| p.iter().filter(|v| adj[u].contains(v)).count())
        .unwrap();
    let candidates: Vec<usize> = p
        .iter()
        .copied()
        .filter(|v| !adj[pivot].contains(v))
        .collect();
    for v in candidates {
        r.push(v);
        let np = p.iter().copied().filter(|u| adj[v].contains(u)).collect();
        let nx = x.iter().copied().filter(|u| adj[v].contains(u)).collect();
        bron_kerbosch(adj, r, np, nx, out);
        r.pop();
        p.retain(|&u| u != v);
        x.push(v);
    }
}

/// All maximal cliques (Bron–Kerbosch with pivoting). Direction is ignored.
pub fn maximal_cliques<G: Graph>(g: &G) -> Vec<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let adj = adjacency(&ix);
    let mut out = Vec::new();
    bron_kerbosch(
        &adj,
        &mut Vec::new(),
        (0..ix.n()).collect(),
        Vec::new(),
        &mut out,
    );
    out.iter().map(|c| ix.vertices_of(c)).collect()
}

/// A maximum clique (largest maximal clique).
pub fn maximum_clique<G: Graph>(g: &G) -> Vec<G::Vertex> {
    maximal_cliques(g)
        .into_iter()
        .max_by_key(Vec::len)
        .unwrap_or_default()
}

/// Whether the vertices are pairwise adjacent.
pub fn is_clique<G: Graph>(g: &G, vs: &[G::Vertex]) -> bool {
    vs.iter().enumerate().all(|(i, a)| {
        vs[i + 1..]
            .iter()
            .all(|b| g.contains_edge_between(a, b) || g.contains_edge_between(b, a))
    })
}

/// Greedy independent set: repeatedly take a minimum-degree vertex and drop its neighbours.
pub fn greedy_independent_set<G: Graph>(g: &G) -> Vec<G::Vertex> {
    let ix = Indexed::new(g);
    let adj = adjacency(&ix);
    let mut alive = vec![true; ix.n()];
    let mut set = Vec::new();
    while let Some(v) = (0..ix.n())
        .filter(|&v| alive[v])
        .min_by_key(|&v| adj[v].iter().filter(|&&u| alive[u]).count())
    {
        set.push(v);
        alive[v] = false;
        adj[v].iter().for_each(|&u| alive[u] = false);
    }
    ix.vertices_of(&set)
}

/// A maximum independent set, computed exactly as a maximum clique of the complement
/// (exponential in the worst case; intended for small graphs).
pub fn maximum_independent_set<G: Graph>(g: &G) -> Vec<G::Vertex> {
    let ix = Indexed::new(g);
    let adj = adjacency(&ix);
    let co: Vec<HashSet<usize>> = (0..ix.n())
        .map(|v| {
            (0..ix.n())
                .filter(|&u| u != v && !adj[v].contains(&u))
                .collect()
        })
        .collect();
    let mut out = Vec::new();
    bron_kerbosch(
        &co,
        &mut Vec::new(),
        (0..ix.n()).collect(),
        Vec::new(),
        &mut out,
    );
    let best = out.into_iter().max_by_key(Vec::len).unwrap_or_default();
    ix.vertices_of(&best)
}

pub fn is_independent_set<G: Graph>(g: &G, vs: &[G::Vertex]) -> bool {
    vs.iter().enumerate().all(|(i, a)| {
        vs[i + 1..]
            .iter()
            .all(|b| !g.contains_edge_between(a, b) && !g.contains_edge_between(b, a))
    })
}
