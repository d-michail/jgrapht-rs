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

use crate::graph::OutgoingGraph;
use super::search::search;
use crate::graph::Graph;
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use rheaps::DecreaseKeyHeap;
use rheaps::tree::PairingHeap;
use std::marker::PhantomData;

/// A* shortest path.
///
/// An implementation of [A* shortest path algorithm](http://en.wikipedia.org/wiki/A*_search_algorithm). This class works for directed and undirected graphs, as well as multi-graphs and
/// mixed-graphs. The graph can also change between invocations of the
/// `getPath(Object, Object)` method; no new instance of this class has to be created. The
/// heuristic is implemented using a PairingHeap data structure by default to maintain the set of
/// open nodes. However, there still exist several approaches in literature to improve the
/// performance of this heuristic which one could consider to implement. Custom heap implementation
/// can be specified during the construction time. Another issue to take into consideration is the
/// following: given two candidate nodes, $i$, $j$ to expand, where $f(i)=f(j)$, $g(i)$ > $g(j)$,
/// $h(i)$ < $g(j)$, $f(i)=g(i)+h(i)$, $g(i)$ is the actual distance from the source node to $i$,
/// $h(i)$ is the estimated distance from $i$ to the target node. Usually a depth-first search is
/// desired, so ideally we would expand node $i$ first. Using the PairingHeap, this is not
/// necessarily the case though. This could be improved in a later version.
///
/// Note: This implementation works with both consistent and inconsistent admissible heuristics. For
/// details on consistency, refer to the description of the method
/// `AStarAdmissibleHeuristic#isConsistent(Graph)`. However, this class is *not* optimized
/// for inconsistent heuristics. Several opportunities to improve both worst case and average runtime
/// complexities for A* with inconsistent heuristics described in literature can be used to improve
/// this implementation!
///
/// # Rust notes
///
/// A* search. `heuristic(v, target)` must never overestimate the remaining distance
/// (admissible); vertices are re-opened if it is not consistent.
pub struct AStar<'g, G, F, H = PairingHeap<OrderedF64, usize>> {
    g: &'g G,
    heuristic: F,
    _heap: PhantomData<fn() -> H>,
}

/// Java-compatible class name for [`AStar`].
pub type AStarShortestPath<'g, G, F, H = PairingHeap<OrderedF64, usize>> = AStar<'g, G, F, H>;

impl<'g, G: Graph, F: Fn(&G::Vertex, &G::Vertex) -> f64> AStar<'g, G, F> {
    pub fn new(g: &'g G, heuristic: F) -> Self {
        AStar {
            g,
            heuristic,
            _heap: PhantomData,
        }
    }
}

impl<
    'g,
    G: OutgoingGraph,
    F: Fn(&G::Vertex, &G::Vertex) -> f64,
    H: DecreaseKeyHeap<OrderedF64, usize> + Default,
> AStar<'g, G, F, H>
{
    pub fn with_heap(g: &'g G, heuristic: F) -> Self {
        AStar {
            g,
            heuristic,
            _heap: PhantomData,
        }
    }

    pub fn path(
        &self,
        s: &G::Vertex,
        t: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        if !self.g.contains_vertex(t) {
            return Err(GraphError::VertexNotFound);
        }
        let r = search::<G, H>(
            self.g,
            s,
            Some(t),
            |e, _, _| self.g.edge_weight(e).unwrap_or(1.0),
            |v| (self.heuristic)(v, t),
            f64::INFINITY,
        )?;
        Ok(r.path(t))
    }
}
