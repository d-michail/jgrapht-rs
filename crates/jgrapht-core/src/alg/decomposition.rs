//! Tree decompositions via elimination-ordering heuristics.

use super::util::Indexed;
use crate::graph::Graph;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EliminationHeuristic {
    MinDegree,
    MinFill,
}

#[derive(Debug, Clone)]
pub struct TreeDecomposition<V> {
    pub bags: Vec<Vec<V>>,
    /// Edges between bag indices; together they form a tree (or a path of trees' roots).
    pub tree_edges: Vec<(usize, usize)>,
}

impl<V> TreeDecomposition<V> {
    /// Largest bag size minus one: an upper bound on the treewidth.
    pub fn width(&self) -> usize {
        self.bags
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(1)
            .saturating_sub(1)
    }
}

/// Tree decomposition from a greedy elimination ordering (direction ignored).
pub fn tree_decomposition<G: Graph>(
    g: &G,
    heuristic: EliminationHeuristic,
) -> TreeDecomposition<G::Vertex> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut nb: Vec<HashSet<usize>> = (0..n)
        .map(|v| {
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v)
                .collect()
        })
        .collect();
    let mut alive = vec![true; n];
    let mut order_pos = vec![usize::MAX; n];
    let mut bags: Vec<Vec<usize>> = Vec::with_capacity(n);
    let mut bag_nb: Vec<Vec<usize>> = Vec::with_capacity(n);
    for step in 0..n {
        let cost = |v: usize| -> usize {
            match heuristic {
                EliminationHeuristic::MinDegree => nb[v].len(),
                EliminationHeuristic::MinFill => {
                    let l: Vec<_> = nb[v].iter().copied().collect();
                    (0..l.len())
                        .map(|i| {
                            (i + 1..l.len())
                                .filter(|&j| !nb[l[i]].contains(&l[j]))
                                .count()
                        })
                        .sum()
                }
            }
        };
        let v = (0..n)
            .filter(|&v| alive[v])
            .min_by_key(|&v| cost(v))
            .unwrap();
        let neighbors: Vec<usize> = nb[v].iter().copied().collect();
        for &a in &neighbors {
            for &b in &neighbors {
                if a != b {
                    nb[a].insert(b);
                }
            }
            nb[a].remove(&v);
        }
        alive[v] = false;
        order_pos[v] = step;
        let mut bag = vec![v];
        bag.extend(&neighbors);
        bags.push(bag);
        bag_nb.push(neighbors);
    }
    // bag `i` (eliminating vertex order[i]) hangs below the bag of its earliest-eliminated neighbour
    let mut tree_edges = Vec::new();
    let mut roots = Vec::new();
    for i in 0..n {
        match bag_nb[i].iter().min_by_key(|&&u| order_pos[u]) {
            Some(&u) => tree_edges.push((i, order_pos[u])),
            None => roots.push(i),
        }
    }
    for w in roots.windows(2) {
        tree_edges.push((w[0], w[1]));
    }
    TreeDecomposition {
        bags: bags.iter().map(|b| ix.vertices_of(b)).collect(),
        tree_edges,
    }
}

/// Upper bound on the treewidth (min-fill decomposition).
pub fn treewidth_upper_bound<G: Graph>(g: &G) -> usize {
    tree_decomposition(g, EliminationHeuristic::MinFill).width()
}
