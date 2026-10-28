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

//! Minimum spanning trees and arborescences.
//!
//! [`kruskal`], [`prim`] and [`boruvka`] return a [`SpanningTree`] (its edges and total weight) of an
//! undirected graph, or a spanning forest if the graph is disconnected. [`minimum_arborescence`] and
//! [`maximum_arborescence`] compute the Chu-Liu/Edmonds optimum branching of a directed graph.
//! [`SpanningTreeAlgorithm`] is the trait shared by the class versions.

use super::util::Indexed;
use crate::graph::{DirectedGraph, Graph};
use crate::util::{OrderedF64, UnionFind};
use crate::{GraphError, Result};
use rheaps::AddressableHeap;
use rheaps::tree::PairingHeap;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "E: serde::Serialize",
        deserialize = "E: serde::Deserialize<'de>"
    ))
)]
#[derive(Debug, Clone, PartialEq)]
/// A spanning tree (or forest): its edges and their total weight.
pub struct SpanningTree<E> {
    /// Edges of the tree (or forest).
    pub edges: Vec<E>,
    /// Total weight of the tree.
    pub weight: f64,
}

fn tree_of<G: Graph>(ix: &Indexed<'_, G>, picked: &[usize]) -> SpanningTree<G::Edge> {
    SpanningTree {
        edges: picked.iter().map(|&k| ix.es[k].clone()).collect(),
        weight: picked.iter().map(|&k| ix.w[k]).sum(),
    }
}

/// Kruskal's algorithm. Returns a minimum spanning forest (direction ignored).
pub fn kruskal<G: Graph>(g: &G) -> SpanningTree<G::Edge> {
    let ix = Indexed::new(g);
    let mut order: Vec<usize> = (0..ix.es.len()).collect();
    order.sort_by(|&a, &b| ix.w[a].total_cmp(&ix.w[b]));
    let mut uf = UnionFind::new(ix.n());
    let picked: Vec<usize> = order
        .into_iter()
        .filter(|&k| uf.union(ix.ends[k].0, ix.ends[k].1))
        .collect();
    tree_of(&ix, &picked)
}

/// Prim's algorithm with a decrease-key heap. Returns a minimum spanning forest.
pub fn prim<G: Graph>(g: &G) -> SpanningTree<G::Edge> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut in_tree = vec![false; n];
    let mut best = vec![f64::INFINITY; n];
    let mut via = vec![usize::MAX; n];
    let mut picked = Vec::new();
    for root in 0..n {
        if in_tree[root] {
            continue;
        }
        let mut heap = PairingHeap::<OrderedF64, usize>::new();
        let mut handle = vec![None; n];
        handle[root] = Some(heap.insert(OrderedF64(0.0), root));
        while let Some((_, v)) = heap.pop() {
            handle[v] = None;
            in_tree[v] = true;
            if via[v] != usize::MAX {
                picked.push(via[v]);
            }
            for &(u, k) in ix.out[v]
                .iter()
                .chain(if ix.directed { &ix.inn[v][..] } else { &[] })
            {
                if !in_tree[u] && ix.w[k] < best[u] {
                    best[u] = ix.w[k];
                    via[u] = k;
                    match handle[u] {
                        Some(h) => heap
                            .decrease_key(h, OrderedF64(ix.w[k]))
                            .expect("smaller key"),
                        None => handle[u] = Some(heap.insert(OrderedF64(ix.w[k]), u)),
                    }
                }
            }
        }
    }
    tree_of(&ix, &picked)
}

/// Borůvka's algorithm. Returns a minimum spanning forest.
pub fn boruvka<G: Graph>(g: &G) -> SpanningTree<G::Edge> {
    let ix = Indexed::new(g);
    let mut uf = UnionFind::new(ix.n());
    let mut picked = Vec::new();
    loop {
        let mut cheapest: Vec<Option<usize>> = vec![None; ix.n()];
        for (k, &(s, t)) in ix.ends.iter().enumerate() {
            let (a, b) = (uf.find(s), uf.find(t));
            if a == b {
                continue;
            }
            for c in [a, b] {
                // ties broken by edge index so the chosen edges never form a cycle
                if cheapest[c].is_none_or(|o| (ix.w[k], k) < (ix.w[o], o)) {
                    cheapest[c] = Some(k);
                }
            }
        }
        let mut merged = false;
        for k in cheapest.into_iter().flatten() {
            if uf.union(ix.ends[k].0, ix.ends[k].1) {
                picked.push(k);
                merged = true;
            }
        }
        if !merged {
            break;
        }
    }
    tree_of(&ix, &picked)
}

/// Chu–Liu/Edmonds on an edge list; returns indices of the chosen edges (one per non-root node).
fn chu_liu(n: usize, root: usize, edges: &[(usize, usize, f64)]) -> Option<Vec<usize>> {
    let mut in_e: Vec<Option<usize>> = vec![None; n];
    for (i, &(u, v, w)) in edges.iter().enumerate() {
        if u != v && v != root && in_e[v].is_none_or(|o| w < edges[o].2) {
            in_e[v] = Some(i);
        }
    }
    if (0..n).any(|v| v != root && in_e[v].is_none()) {
        return None;
    }
    let from = |v: usize| edges[in_e[v].unwrap()].0;
    let (mut id, mut vis, mut cnt) = (vec![usize::MAX; n], vec![usize::MAX; n], 0);
    for v in 0..n {
        let mut x = v;
        while vis[x] != v && id[x] == usize::MAX && x != root {
            vis[x] = v;
            x = from(x);
        }
        if x != root && id[x] == usize::MAX {
            let mut y = from(x);
            id[x] = cnt;
            while y != x {
                id[y] = cnt;
                y = from(y);
            }
            cnt += 1;
        }
    }
    if cnt == 0 {
        return Some(
            (0..n)
                .filter(|&v| v != root)
                .map(|v| in_e[v].unwrap())
                .collect(),
        );
    }
    for v in 0..n {
        if id[v] == usize::MAX {
            id[v] = cnt;
            cnt += 1;
        }
    }
    let mut sub = Vec::new();
    let mut origin = Vec::new();
    for (i, &(u, v, w)) in edges.iter().enumerate() {
        if id[u] != id[v] {
            let shift = if v != root {
                edges[in_e[v].unwrap()].2
            } else {
                0.0
            };
            sub.push((id[u], id[v], w - shift));
            origin.push(i);
        }
    }
    let chosen = chu_liu(cnt, id[root], &sub)?;
    let mut entering: Vec<Option<usize>> = vec![None; n];
    for c in chosen {
        let e = origin[c];
        entering[edges[e].1] = Some(e);
    }
    Some(
        (0..n)
            .filter(|&v| v != root)
            .map(|v| entering[v].or(in_e[v]).unwrap())
            .collect(),
    )
}

/// Minimum-weight spanning arborescence rooted at `root` (Chu–Liu/Edmonds).
/// Fails if some vertex is unreachable from the root.
///
/// # Errors
///
/// Errors of the graph operations it performs are propagated.
pub fn minimum_arborescence<G: DirectedGraph>(
    g: &G,
    root: &G::Vertex,
) -> Result<SpanningTree<G::Edge>> {
    arborescence(g, root, 1.0)
}

/// Maximum-weight spanning arborescence rooted at `root`.
///
/// # Errors
///
/// Errors of the graph operations it performs are propagated.
pub fn maximum_arborescence<G: DirectedGraph>(
    g: &G,
    root: &G::Vertex,
) -> Result<SpanningTree<G::Edge>> {
    arborescence(g, root, -1.0)
}

fn arborescence<G: DirectedGraph>(
    g: &G,
    root: &G::Vertex,
    sign: f64,
) -> Result<SpanningTree<G::Edge>> {
    let ix = Indexed::new(g);
    let r = *ix.idx.get(root).ok_or(GraphError::VertexNotFound)?;
    let edges: Vec<_> = ix
        .ends
        .iter()
        .zip(&ix.w)
        .map(|(&(s, t), &w)| (s, t, sign * w))
        .collect();
    let chosen = chu_liu(ix.n(), r, &edges)
        .ok_or(GraphError::InvalidGraph(crate::GraphProperty::Connected))?;
    Ok(tree_of(&ix, &chosen))
}

/// An algorithm which computes a [spanning tree](https://en.wikipedia.org/wiki/Spanning_tree) of a given connected graph. In the case of disconnected graphs it would rather derive a
/// spanning *forest*.
///
/// # Rust notes
///
/// An algorithm computing a spanning tree or forest (`SpanningTreeAlgorithm`).
pub trait SpanningTreeAlgorithm<G: Graph> {
    /// The spanning tree computed by the algorithm.
    fn spanning_tree(&self) -> SpanningTree<G::Edge>;
}

macro_rules! mst_algorithm {
    ($(#[$doc:meta])* $name:ident, $f:ident) => {
        $(#[$doc])*
        pub struct $name<'g, G: Graph> {
            graph: &'g G,
        }
        impl<'g, G: Graph> $name<'g, G> {
            /// The algorithm on `graph`.
            pub fn new(graph: &'g G) -> Self {
                $name { graph }
            }
        }
        impl<G: Graph> SpanningTreeAlgorithm<G> for $name<'_, G> {
            fn spanning_tree(&self) -> SpanningTree<G::Edge> {
                $f(self.graph)
            }
        }
    };
}

mst_algorithm!(
    /// `KruskalMinimumSpanningTree`.
    KruskalMinimumSpanningTree,
    kruskal
);
mst_algorithm!(
    /// `PrimMinimumSpanningTree`.
    PrimMinimumSpanningTree,
    prim
);
mst_algorithm!(
    /// `BoruvkaMinimumSpanningTree`.
    BoruvkaMinimumSpanningTree,
    boruvka
);
