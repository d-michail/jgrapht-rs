//! Facility location: k-center.

use super::util::Indexed;
use crate::graph::Graph;
use crate::util::OrderedF64;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

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
