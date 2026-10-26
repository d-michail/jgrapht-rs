//! Graph clustering.

use super::spanning::kruskal;
use super::util::Indexed;
use crate::graph::Graph;
use crate::util::UnionFind;
use crate::{GraphError, Result};
use rand::Rng;
use rand::seq::SliceRandom;
use std::collections::HashMap;

fn groups<G: Graph>(ix: &Indexed<'_, G>, label: &[usize]) -> Vec<Vec<G::Vertex>> {
    let mut by: HashMap<usize, Vec<usize>> = HashMap::new();
    for (v, &l) in label.iter().enumerate() {
        by.entry(l).or_default().push(v);
    }
    let mut cs: Vec<Vec<usize>> = by.into_values().collect();
    cs.sort_by_key(|c| c[0]);
    cs.iter().map(|c| ix.vertices_of(c)).collect()
}

/// Asynchronous label propagation (Raghavan et al.): every vertex repeatedly adopts the most
/// frequent label among its neighbours (edge weights count), ties broken at random.
pub fn label_propagation<G: Graph>(
    g: &G,
    max_iterations: usize,
    rng: &mut impl Rng,
) -> Vec<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut label: Vec<usize> = (0..n).collect();
    let mut order: Vec<usize> = (0..n).collect();
    for _ in 0..max_iterations {
        order.shuffle(rng);
        let mut changed = false;
        for &v in &order {
            let mut score: HashMap<usize, f64> = HashMap::new();
            for &(u, k) in ix.out[v]
                .iter()
                .chain(if ix.directed { &ix.inn[v][..] } else { &[] })
            {
                if u != v {
                    *score.entry(label[u]).or_default() += ix.w[k];
                }
            }
            let Some(best) = score.values().copied().max_by(|a, b| a.total_cmp(b)) else {
                continue;
            };
            let ties: Vec<usize> = score
                .iter()
                .filter(|(_, s)| **s == best)
                .map(|(l, _)| *l)
                .collect();
            if !ties.contains(&label[v]) {
                label[v] = ties[rng.random_range(0..ties.len())];
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    groups(&ix, &label)
}

/// k-spanning-tree clustering: build a minimum spanning forest and cut its heaviest edges
/// until `k` clusters remain.
pub fn k_spanning_tree_clustering<G: Graph>(g: &G, k: usize) -> Result<Vec<Vec<G::Vertex>>> {
    let ix = Indexed::new(g);
    if k == 0 || k > ix.n() {
        return Err(GraphError::InvalidArgument(
            "k must be between 1 and the number of vertices".into(),
        ));
    }
    let tree = kruskal(g);
    let mut edges: Vec<usize> = tree
        .edges
        .iter()
        .map(|e| ix.es.iter().position(|x| *x == e).unwrap())
        .collect();
    edges.sort_by(|&a, &b| ix.w[a].total_cmp(&ix.w[b]));
    let mut uf = UnionFind::new(ix.n());
    for &e in &edges {
        uf.union(ix.ends[e].0, ix.ends[e].1);
    }
    // the forest may already have more than one component; cut heaviest edges until k remain
    let mut keep = edges.len();
    while uf.set_count() < k && keep > 0 {
        keep -= 1;
        uf = UnionFind::new(ix.n());
        for &e in &edges[..keep] {
            uf.union(ix.ends[e].0, ix.ends[e].1);
        }
    }
    if uf.set_count() > k {
        return Err(GraphError::InvalidArgument(
            "graph has more components than k".into(),
        ));
    }
    let label: Vec<usize> = (0..ix.n()).map(|v| uf.find(v)).collect();
    Ok(groups(&ix, &label))
}
