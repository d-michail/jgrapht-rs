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
        return Err(GraphError::InvalidArgument(
            "spanners need an undirected graph".into(),
        ));
    }
    if k == 0 {
        return Err(GraphError::InvalidArgument("k must be positive".into()));
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
