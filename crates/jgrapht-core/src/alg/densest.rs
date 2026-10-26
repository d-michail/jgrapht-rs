//! Densest subgraph (density = edges / vertices, direction and multiplicity ignored).

use super::flow::{MaxFlowAlgorithm, max_flow};
use super::util::Indexed;
use crate::graph::Graph;
use crate::graph::GraphMutExt;
use crate::{DiGraph, Result};
use std::collections::HashSet;

fn simple_edges<G: Graph>(ix: &Indexed<'_, G>) -> Vec<(usize, usize)> {
    let set: HashSet<(usize, usize)> = ix
        .ends
        .iter()
        .filter(|&&(a, b)| a != b)
        .map(|&(a, b)| (a.min(b), a.max(b)))
        .collect();
    let mut v: Vec<_> = set.into_iter().collect();
    v.sort_unstable();
    v
}

fn density_of(edges: &[(usize, usize)], set: &[bool]) -> f64 {
    let n = set.iter().filter(|&&x| x).count();
    if n == 0 {
        return 0.0;
    }
    edges.iter().filter(|&&(a, b)| set[a] && set[b]).count() as f64 / n as f64
}

/// Charikar's greedy peeling: a 1/2-approximation of the densest subgraph.
pub fn greedy_densest_subgraph<G: Graph>(g: &G) -> (Vec<G::Vertex>, f64) {
    let ix = Indexed::new(g);
    let n = ix.n();
    let edges = simple_edges(&ix);
    let mut adj = vec![HashSet::new(); n];
    for &(a, b) in &edges {
        adj[a].insert(b);
        adj[b].insert(a);
    }
    let mut alive = vec![true; n];
    let (mut m, mut count) = (edges.len(), n);
    let (mut best, mut best_set) = (if n > 0 { m as f64 / n as f64 } else { 0.0 }, alive.clone());
    while count > 1 {
        let v = (0..n)
            .filter(|&v| alive[v])
            .min_by_key(|&v| adj[v].iter().filter(|&&u| alive[u]).count())
            .unwrap();
        m -= adj[v].iter().filter(|&&u| alive[u]).count();
        alive[v] = false;
        count -= 1;
        let d = m as f64 / count as f64;
        if d > best {
            best = d;
            best_set = alive.clone();
        }
    }
    (
        (0..n)
            .filter(|&v| best_set[v])
            .map(|v| ix.vertex(v))
            .collect(),
        best,
    )
}

/// Exact maximum density subgraph (Goldberg's reduction to a sequence of minimum cuts).
pub fn goldberg_densest_subgraph<G: Graph>(g: &G) -> Result<(Vec<G::Vertex>, f64)> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let edges = simple_edges(&ix);
    let m = edges.len() as f64;
    if edges.is_empty() {
        return Ok((ix.vs.iter().take(1).map(|v| (*v).clone()).collect(), 0.0));
    }
    let mut deg = vec![0.0; n];
    for &(a, b) in &edges {
        deg[a] += 1.0;
        deg[b] += 1.0;
    }
    let (mut lo, mut hi) = (0.0f64, m);
    let mut best = vec![true; n];
    let precision = 1.0 / (n as f64 * (n as f64 - 1.0)).max(1.0);
    while hi - lo >= precision {
        let guess = (lo + hi) / 2.0;
        let (s, t) = (n, n + 1);
        let mut net = DiGraph::<usize>::simple().weighted();
        net.add_vertices(0..n + 2);
        for v in 0..n {
            net.add_weighted_edge(&s, &v, m)?;
            net.add_weighted_edge(&v, &t, m + 2.0 * guess - deg[v])?;
        }
        for &(a, b) in &edges {
            net.add_weighted_edge(&a, &b, 1.0)?;
            net.add_weighted_edge(&b, &a, 1.0)?;
        }
        let f = max_flow(&net, &s, &t, MaxFlowAlgorithm::Dinic)?;
        let side: Vec<bool> = {
            let mut x = vec![false; n];
            f.source_side
                .iter()
                .filter(|&&v| v < n)
                .for_each(|&v| x[v] = true);
            x
        };
        if side.iter().any(|&b| b) {
            lo = guess;
            best = side;
        } else {
            hi = guess;
        }
    }
    let d = density_of(&edges, &best);
    Ok((
        (0..n).filter(|&v| best[v]).map(|v| ix.vertex(v)).collect(),
        d,
    ))
}
