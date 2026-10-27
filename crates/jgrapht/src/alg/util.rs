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

use crate::graph::{Graph, GraphExt};
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
pub fn vertex_degree_comparator<G: Graph>(
    g: &G,
) -> impl Fn(&G::Vertex, &G::Vertex) -> std::cmp::Ordering + '_ {
    move |a, b| g.degree_of(a).unwrap().cmp(&g.degree_of(b).unwrap())
}

pub struct Indexed<'g, G: Graph> {
    pub g: &'g G,
    pub vs: Vec<&'g G::Vertex>,
    pub idx: HashMap<&'g G::Vertex, usize>,
    pub es: Vec<&'g G::Edge>,
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
        let vs: Vec<_> = g.vertices().collect();
        let idx: HashMap<_, _> = vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let directed = g.graph_type().directed;
        let n = vs.len();
        let (mut es, mut ends, mut w) = (Vec::new(), Vec::new(), Vec::new());
        let mut out = vec![Vec::new(); n];
        let mut inn = vec![Vec::new(); n];
        for e in g.edges() {
            let (s, t) = g.edge_endpoints(e).expect("edge endpoints");
            let (s, t) = (idx[s], idx[t]);
            let k = es.len();
            es.push(e);
            ends.push((s, t));
            w.push(g.edge_weight(e).unwrap_or(1.0));
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
            g,
            vs,
            idx,
            es,
            ends,
            w,
            out,
            inn,
            directed,
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
