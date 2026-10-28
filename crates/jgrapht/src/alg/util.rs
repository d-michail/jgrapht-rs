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

//! Index-based snapshot of a graph, used by algorithms that prefer dense arrays.

use crate::graph::IncidenceGraph;
use crate::graph::{Graph, GraphExt};
use std::borrow::Borrow;
use std::collections::HashMap;

mod neighbor_cache;
mod pair;
mod queue;
mod union_find;

pub use neighbor_cache::{NeighborCache, Neighbors};
pub use pair::{Pair, Triple, UnorderedPair};
pub use queue::FixedSizeIntegerQueue;
pub use union_find::UnionFind;

/// `VertexDegreeComparator.of(g)`: a comparison function ordering vertices by degree.
pub fn vertex_degree_comparator<G: IncidenceGraph>(
    g: &G,
) -> impl Fn(&G::Vertex, &G::Vertex) -> std::cmp::Ordering + '_ {
    move |a, b| g.degree_of(a).cmp(&g.degree_of(b))
}

/// Dense, index-based snapshot of a graph: vertices and edges numbered in iteration order, with
/// adjacency lists of `(neighbour, edge index)`.
///
/// Building it relies on the [`Graph`] integrity contract: every edge has endpoints that are
/// vertices and a weight. A graph that breaks the contract makes [`Indexed::new`] panic with a
/// message naming the contract; no valid graph can trigger it.
pub(crate) struct Indexed<'g, G: Graph> {
    pub vs: Vec<G::Vertex>,
    pub idx: HashMap<G::Vertex, usize>,
    pub es: Vec<G::Edge>,
    _g: std::marker::PhantomData<&'g G>,
    /// `(source, target)` per edge index.
    pub ends: Vec<(usize, usize)>,
    pub w: Vec<f64>,
    /// `out[v]` = `(neighbour, edge index)`; for undirected graphs every incident edge.
    pub out: Vec<Vec<(usize, usize)>>,
    /// Incoming adjacency (same as `out` for undirected graphs).
    pub inn: Vec<Vec<(usize, usize)>>,
    pub directed: bool,
}

impl<'g, G: Graph> Indexed<'g, G> {
    pub fn new(g: &'g G) -> Self {
        let vs: Vec<G::Vertex> = g.vertices_cloned().collect();
        let idx: HashMap<G::Vertex, usize> =
            vs.iter().enumerate().map(|(i, v)| (v.clone(), i)).collect();
        let directed = g.graph_type().directed;
        let n = vs.len();
        let (mut es, mut ends, mut w) = (Vec::new(), Vec::new(), Vec::new());
        let mut out = vec![Vec::new(); n];
        let mut inn = vec![Vec::new(); n];
        for e in g.edges() {
            let e = e.borrow();
            let (s, t) = g
                .edge_endpoints(e)
                .expect("Graph integrity: every edge has a source and a target");
            let (s, t) = (
                *idx.get(s)
                    .expect("Graph integrity: edge endpoints are vertices"),
                *idx.get(t)
                    .expect("Graph integrity: edge endpoints are vertices"),
            );
            let k = es.len();
            es.push(e.clone());
            ends.push((s, t));
            w.push(
                g.edge_weight(e)
                    .expect("Graph integrity: every edge has a weight"),
            );
            out[s].push((t, k));
            if directed {
                inn[t].push((s, k));
            } else if s != t {
                out[t].push((s, k));
            }
        }
        if !directed {
            inn = out.clone();
        }
        Indexed {
            vs,
            idx,
            es,
            ends,
            w,
            out,
            inn,
            directed,
            _g: std::marker::PhantomData,
        }
    }
    pub fn n(&self) -> usize {
        self.vs.len()
    }
    pub fn vertex(&self, i: usize) -> G::Vertex {
        self.vs[i].clone()
    }
    pub fn vertices_of(&self, ix: &[usize]) -> Vec<G::Vertex> {
        ix.iter().map(|&i| self.vs[i].clone()).collect()
    }
}
