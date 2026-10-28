// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::GraphExt;
use crate::graph::OutgoingGraph;
use crate::{GraphError, Result};
use std::collections::{HashMap, VecDeque};

/// The BFS Shortest Path algorithm.
///
/// An implementation of [BFS shortest path algorithm](https://en.wikipedia.org/wiki/Breadth-first_search) to compute shortest paths from a single source vertex to all other vertices in
/// an unweighted graph.
///
/// The running time is $O(|V|+|E|)$.
///
/// # Rust notes
///
/// Fewest-edges paths (edge weights are ignored; a path's weight is its edge count).
pub struct BfsShortestPath<'g, G>(pub &'g G);

impl<'g, G> BfsShortestPath<'g, G> {
    /// Construct a new instance.
    pub fn new(graph: &'g G) -> Self {
        Self(graph)
    }
}

impl<G: OutgoingGraph> ShortestPathAlgorithm<G> for BfsShortestPath<'_, G> {
    fn single_source(&self, s: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        let g = self.0;
        if !g.contains_vertex(s) {
            return Err(GraphError::VertexNotFound);
        }
        let mut out = SingleSourcePaths {
            source: s.clone(),
            dist: HashMap::from([(s.clone(), 0.0)]),
            pred: HashMap::new(),
        };
        let mut q = VecDeque::from([s]);
        while let Some(v) = q.pop_front() {
            let d = out.dist[v];
            for (e, n) in g.outgoing_of(v) {
                if !out.dist.contains_key(n) {
                    out.dist.insert(n.clone(), d + 1.0);
                    out.pred.insert(n.clone(), (e.clone(), v.clone()));
                    q.push_back(n);
                }
            }
        }
        Ok(out)
    }
}
