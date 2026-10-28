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

use super::bellman_ford::relax_all;
use super::search::search;
use super::{AllPairsPaths, ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::Graph;
use crate::graph::GraphExt;
use crate::graph::OutgoingGraph;
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use rheaps::tree::PairingHeap;
use std::collections::HashMap;

/// Johnson's all-pairs algorithm: Bellman–Ford potentials, then Dijkstra from every vertex on
/// reweighted edges. Good for sparse graphs with negative weights; `Err(NegativeCycle)` if the
/// graph has a negative cycle.
pub(crate) fn johnson<G: OutgoingGraph>(g: &G) -> Result<AllPairsPaths<G::Vertex, G::Edge>> {
    // a virtual source reaching every vertex at cost 0 == all potentials start at 0
    let mut h: HashMap<&G::Vertex, f64> = g.vertices().map(|v| (v, 0.0)).collect();
    relax_all(g, &mut h, &mut HashMap::new())?;
    let mut by_source = HashMap::new();
    for s in g.vertices() {
        let r = search::<G, PairingHeap<OrderedF64, usize>>(
            g,
            s,
            None,
            |e, u, v| (g.weight_of(e) + h[u] - h[v]).max(0.0),
            |_| 0.0,
            f64::INFINITY,
        )?;
        let dist = r
            .dist
            .iter()
            .map(|(t, d)| (t.clone(), d - h[s] + h[t]))
            .collect();
        by_source.insert(
            s.clone(),
            SingleSourcePaths {
                source: r.source,
                dist,
                pred: r.pred,
            },
        );
    }
    Ok(AllPairsPaths { by_source })
}

/// Johnson's all pairs shortest paths algorithm.
///
/// Finds the shortest paths between all pairs of vertices in a sparse graph. Edge weights can be
/// negative, but no negative-weight cycles may exist. It first executes the Bellman-Ford algorithm
/// to compute a transformation of the input graph that removes all negative weights, allowing
/// Dijkstra's algorithm to be used on the transformed graph.
///
/// Running time is $O(n m + n^2 \log n)$.
///
/// Since Johnson's algorithm creates additional vertices, this implementation requires the user to
/// provide a graph which is initialized with a vertex supplier.
///
/// In case the algorithm detects a negative weight cycle it fails with
/// [`GraphError::NegativeCycle`].
///
/// # Rust notes
///
/// Johnson all-pairs shortest paths.
pub struct JohnsonShortestPaths<'g, G: Graph> {
    graph: &'g G,
    paths: AllPairsPaths<G::Vertex, G::Edge>,
}

impl<'g, G: OutgoingGraph> JohnsonShortestPaths<'g, G> {
    /// Construct a new instance.
    pub fn new(graph: &'g G) -> Result<Self> {
        Ok(Self {
            graph,
            paths: johnson(graph)?,
        })
    }

    /// Returns the graph over which this set of paths is defined.
    /// The shortest paths between every pair of vertices.
    pub fn all_pairs(&self) -> &AllPairsPaths<G::Vertex, G::Edge> {
        &self.paths
    }

    pub fn graph(&self) -> &G {
        self.graph
    }
}

impl<G: Graph> ShortestPathAlgorithm<G> for JohnsonShortestPaths<'_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        self.paths
            .from_source(source)
            .cloned()
            .ok_or(GraphError::VertexNotFound)
    }

    /// Get a shortest path from a source vertex to a sink vertex.
    fn path(
        &self,
        source: &G::Vertex,
        target: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        if !self.graph.contains_vertex(source) || !self.graph.contains_vertex(target) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(self.paths.path(source, target))
    }

    /// Get the weight of the shortest path from a source vertex to a sink vertex; `None` if no
    /// path exists.
    fn path_weight(&self, source: &G::Vertex, target: &G::Vertex) -> Result<Option<f64>> {
        if !self.graph.contains_vertex(source) || !self.graph.contains_vertex(target) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(self.paths.weight(source, target))
    }
}
