// (C) Copyright 2003-2026, by John V Sichi and JGraphT Contributors.
// Rust port (C) Copyright 2026, by Dimitrios Michail and jgrapht-rs Contributors.
//
// See the CONTRIBUTORS.md file distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the GNU Lesser General Public
// License v2.1 or later which is available at
// http://www.gnu.org/licenses/old-licenses/lgpl-2.1-standalone.html.
//
// SPDX-License-Identifier: EPL-2.0 OR LGPL-2.1-or-later

//! Steiner trees in undirected graphs with non-negative weights.

use super::shortestpath::{Dijkstra, ShortestPathAlgorithm};
use super::util::Indexed;
use crate::graph::Graph;
use crate::graph::OutgoingGraph;
use crate::util::UnionFind;
use crate::{GraphError, Result};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
/// The `SteinerTree` struct.
pub struct SteinerTree<E> {
    /// The edges.
    pub edges: Vec<E>,
    /// The weight.
    pub weight: f64,
}

fn check<G: Graph>(g: &G, terminals: &[G::Vertex]) -> Result<()> {
    if g.graph_type().directed {
        return Err(GraphError::InvalidGraph(crate::GraphProperty::Undirected));
    }
    if terminals.is_empty() || terminals.iter().any(|t| !g.contains_vertex(t)) {
        return Err(if terminals.is_empty() {
            GraphError::InvalidParameter {
                name: "terminals",
                requirement: crate::ParameterRequirement::NonEmpty,
            }
        } else {
            GraphError::VertexNotFound
        });
    }
    Ok(())
}

/// Kou–Markowsky–Berman 2-approximation: MST of the terminal metric closure, expanded into
/// graph paths, re-spanned and pruned of non-terminal leaves.
///
/// # Errors
///
/// Returns a [`GraphError`] if the graph lacks a property the algorithm requires; errors of the underlying graph operations are propagated.
///
/// # Panics
///
/// Panics only if an internal invariant is violated, which indicates a bug in this library.
pub fn kmb_steiner_tree<G: OutgoingGraph>(
    g: &G,
    terminals: &[G::Vertex],
) -> Result<SteinerTree<G::Edge>> {
    check(g, terminals)?;
    let mut terms: Vec<&G::Vertex> = Vec::new();
    for t in terminals {
        if !terms.contains(&t) {
            terms.push(t);
        }
    }
    let k = terms.len();
    let sp: Vec<_> = terms
        .iter()
        .map(|t| Dijkstra::new(g).single_source(t))
        .collect::<Result<_>>()?;
    // Prim on the metric closure
    let mut in_tree = vec![false; k];
    in_tree[0] = true;
    let mut chosen: HashSet<G::Edge> = HashSet::new();
    for _ in 1..k {
        let mut best = (f64::INFINITY, 0, 0);
        for a in (0..k).filter(|&a| in_tree[a]) {
            for b in (0..k).filter(|&b| !in_tree[b]) {
                let d = sp[a].weight(terms[b]).unwrap_or(f64::INFINITY);
                if d < best.0 {
                    best = (d, a, b);
                }
            }
        }
        if best.0.is_infinite() {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::Connected));
        }
        in_tree[best.2] = true;
        let path = sp[best.1].path(terms[best.2]).expect("finite distance");
        chosen.extend(path.edges);
    }
    // MST of the chosen subgraph, then drop non-terminal leaves
    let ix = Indexed::new(g);
    let mut idx: Vec<usize> = (0..ix.es.len())
        .filter(|&e| chosen.contains(&ix.es[e]))
        .collect();
    idx.sort_by(|&a, &b| ix.w[a].total_cmp(&ix.w[b]));
    let mut uf = UnionFind::new(ix.n());
    let mut tree: Vec<usize> = idx
        .into_iter()
        .filter(|&e| uf.union(ix.ends[e].0, ix.ends[e].1))
        .collect();
    let term_set: HashSet<usize> = terms.iter().map(|t| ix.idx[*t]).collect();
    loop {
        let mut deg: HashMap<usize, usize> = HashMap::new();
        for &e in &tree {
            *deg.entry(ix.ends[e].0).or_default() += 1;
            *deg.entry(ix.ends[e].1).or_default() += 1;
        }
        let before = tree.len();
        tree.retain(|&e| {
            let (a, b) = ix.ends[e];
            !((deg[&a] == 1 && !term_set.contains(&a)) || (deg[&b] == 1 && !term_set.contains(&b)))
        });
        if tree.len() == before {
            break;
        }
    }
    Ok(SteinerTree {
        weight: tree.iter().map(|&e| ix.w[e]).sum(),
        edges: tree.iter().map(|&e| ix.es[e].clone()).collect(),
    })
}

/// Exact minimum Steiner tree weight by the Dreyfus–Wagner dynamic programme,
/// `O(3^k n + 2^k n^2)`; limited to 16 terminals.
///
/// # Errors
///
/// Returns a [`GraphError`] if a numeric or collection parameter is out of range; the graph lacks a property the algorithm requires; errors of the underlying graph operations are propagated.
pub fn dreyfus_wagner_weight<G: Graph>(g: &G, terminals: &[G::Vertex]) -> Result<f64> {
    check(g, terminals)?;
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut terms: Vec<usize> = terminals.iter().map(|t| ix.idx[t]).collect();
    terms.sort_unstable();
    terms.dedup();
    let k = terms.len();
    if k > 16 {
        return Err(GraphError::InvalidParameter {
            name: "terminals",
            requirement: crate::ParameterRequirement::AtMost(16),
        });
    }
    if k == 1 {
        return Ok(0.0);
    }
    let mut d = vec![vec![f64::INFINITY; n]; n];
    for v in 0..n {
        d[v][v] = 0.0;
    }
    for (e, &(a, b)) in ix.ends.iter().enumerate() {
        d[a][b] = d[a][b].min(ix.w[e]);
        d[b][a] = d[b][a].min(ix.w[e]);
    }
    for m in 0..n {
        for a in 0..n {
            for b in 0..n {
                if d[a][m] + d[m][b] < d[a][b] {
                    d[a][b] = d[a][m] + d[m][b];
                }
            }
        }
    }
    // dp[S][v] = cheapest tree connecting terminal subset S and vertex v
    let full = 1usize << k;
    let mut dp = vec![vec![f64::INFINITY; n]; full];
    for (i, &t) in terms.iter().enumerate() {
        for v in 0..n {
            dp[1 << i][v] = d[t][v];
        }
    }
    for s in 1..full {
        if s.count_ones() < 2 {
            continue;
        }
        let mut sub = (s - 1) & s;
        while sub > 0 {
            if sub < (s ^ sub) {
                for v in 0..n {
                    dp[s][v] = dp[s][v].min(dp[sub][v] + dp[s ^ sub][v]);
                }
            }
            sub = (sub - 1) & s;
        }
        let base = dp[s].clone();
        for v in 0..n {
            for u in 0..n {
                dp[s][v] = dp[s][v].min(base[u] + d[u][v]);
            }
        }
    }
    let w = dp[full - 1][terms[0]];
    if w.is_infinite() {
        return Err(GraphError::InvalidGraph(crate::GraphProperty::Connected));
    }
    Ok(w)
}
