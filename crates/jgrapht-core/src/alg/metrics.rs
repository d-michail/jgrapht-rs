//! Graph metrics: eccentricity, diameter, radius, center, periphery, girth.

use super::util::Indexed;
use crate::graph::Graph;
use crate::util::OrderedF64;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

/// Eccentricity of every vertex (`INFINITY` if some vertex is unreachable from it).
pub fn eccentricities<G: Graph>(g: &G, weighted: bool) -> HashMap<G::Vertex, f64> {
    let ix = Indexed::new(g);
    (0..ix.n())
        .map(|s| {
            let mut dist = vec![f64::INFINITY; ix.n()];
            dist[s] = 0.0;
            let mut heap = BinaryHeap::from([Reverse((OrderedF64(0.0), s))]);
            while let Some(Reverse((OrderedF64(d), v))) = heap.pop() {
                if d > dist[v] {
                    continue;
                }
                for &(u, k) in &ix.out[v] {
                    let nd = d + if weighted { ix.w[k] } else { 1.0 };
                    if nd < dist[u] {
                        dist[u] = nd;
                        heap.push(Reverse((OrderedF64(nd), u)));
                    }
                }
            }
            (ix.vertex(s), dist.into_iter().fold(0.0, f64::max))
        })
        .collect()
}

/// Largest eccentricity (`0.0` for the empty graph).
pub fn diameter<G: Graph>(g: &G, weighted: bool) -> f64 {
    eccentricities(g, weighted)
        .values()
        .copied()
        .fold(0.0, f64::max)
}

/// Smallest eccentricity.
pub fn radius<G: Graph>(g: &G, weighted: bool) -> f64 {
    eccentricities(g, weighted)
        .values()
        .copied()
        .fold(f64::INFINITY, f64::min)
}

/// Vertices whose eccentricity equals the radius.
pub fn center<G: Graph>(g: &G, weighted: bool) -> Vec<G::Vertex> {
    let e = eccentricities(g, weighted);
    let r = e.values().copied().fold(f64::INFINITY, f64::min);
    g.vertices().filter(|v| e[*v] == r).cloned().collect()
}

/// Vertices whose eccentricity equals the diameter.
pub fn periphery<G: Graph>(g: &G, weighted: bool) -> Vec<G::Vertex> {
    let e = eccentricities(g, weighted);
    let d = e.values().copied().fold(0.0, f64::max);
    g.vertices().filter(|v| e[*v] == d).cloned().collect()
}

/// Length of the shortest cycle in an undirected simple graph (`None` for forests).
pub fn girth<G: Graph>(g: &G) -> Option<usize> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut best: Option<usize> = None;
    for s in 0..n {
        let (mut dist, mut parent_edge) = (vec![usize::MAX; n], vec![usize::MAX; n]);
        dist[s] = 0;
        let mut q = VecDeque::from([s]);
        while let Some(v) = q.pop_front() {
            let nbrs = ix.out[v]
                .iter()
                .chain(if ix.directed { &ix.inn[v][..] } else { &[] });
            for &(u, k) in nbrs {
                if u == v {
                    return Some(1);
                }
                if dist[u] == usize::MAX {
                    dist[u] = dist[v] + 1;
                    parent_edge[u] = k;
                    q.push_back(u);
                } else if parent_edge[v] != k {
                    let len = dist[u] + dist[v] + 1;
                    best = Some(best.map_or(len, |b| b.min(len)));
                }
            }
        }
    }
    best
}
