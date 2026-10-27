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

/// The result of a spanner algorithm (`SpannerAlgorithm.Spanner`).
#[derive(Debug, Clone, PartialEq)]
pub struct Spanner<E: Eq + std::hash::Hash> {
    pub edges: indexmap::IndexSet<E>,
    pub weight: f64,
}

/// Greedy `(2k-1)`-spanner (`GreedyMultiplicativeSpanner`): edges are scanned by increasing
/// weight and kept if the spanner so far has no path within `(2k-1)` times the edge weight.
/// Unweighted graphs use hop counts.
pub struct GreedyMultiplicativeSpanner<'g, G: Graph> {
    graph: &'g G,
    k: usize,
}

impl<'g, G: Graph> GreedyMultiplicativeSpanner<'g, G> {
    const MAX_K: usize = 1 << 29;

    pub fn new(graph: &'g G, k: usize) -> Result<Self> {
        if graph.graph_type().directed {
            return Err(GraphError::InvalidArgument(
                "graph is not undirected".into(),
            ));
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

    pub fn spanner(&self) -> Result<Spanner<G::Edge>> {
        let weighted = self.graph.graph_type().weighted;
        let ix = Indexed::new(self.graph);
        let mut order: Vec<usize> = (0..ix.es.len()).collect();
        order.sort_by(|&a, &b| ix.w[a].total_cmp(&ix.w[b]));
        if let Some(&first) = order.first() {
            if ix.w[first] < 0.0 {
                return Err(GraphError::InvalidArgument(
                    "Illegal edge weight: negative".into(),
                ));
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
