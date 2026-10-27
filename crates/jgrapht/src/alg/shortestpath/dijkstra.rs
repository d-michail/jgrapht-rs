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

use super::search::search;
use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::Result;
use crate::graph::Graph;
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use rheaps::DecreaseKeyHeap;
use rheaps::tree::PairingHeap;
use std::marker::PhantomData;

/// Dijkstra's algorithm. Fails with `InvalidArgument` on negative edge weights.
/// `H` selects the priority queue (pairing heap by default).
pub struct Dijkstra<'g, G, H = PairingHeap<OrderedF64, usize>> {
    g: &'g G,
    radius: f64,
    _heap: PhantomData<fn() -> H>,
}

/// Java-compatible class name for [`Dijkstra`].
pub type DijkstraShortestPath<'g, G, H = PairingHeap<OrderedF64, usize>> = Dijkstra<'g, G, H>;

impl<'g, G: Graph> Dijkstra<'g, G> {
    pub fn new(g: &'g G) -> Self {
        Self::with_heap(g)
    }
}

impl<'g, G: Graph, H: DecreaseKeyHeap<OrderedF64, usize> + Default> Dijkstra<'g, G, H> {
    /// Choose the heap explicitly, e.g. `Dijkstra::<_, FibonacciHeap<_, _>>::with_heap(&g)`.
    pub fn with_heap(g: &'g G) -> Self {
        Dijkstra {
            g,
            radius: f64::INFINITY,
            _heap: PhantomData,
        }
    }
    /// Ignore vertices farther than `radius` from the source.
    pub fn radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }
}

impl<G: Graph, H: DecreaseKeyHeap<OrderedF64, usize> + Default> ShortestPathAlgorithm<G>
    for Dijkstra<'_, G, H>
{
    fn single_source(&self, s: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        search::<G, H>(
            self.g,
            s,
            None,
            |e, _, _| self.g.edge_weight(e).unwrap_or(1.0),
            |_| 0.0,
            self.radius,
        )
    }
    fn path(&self, s: &G::Vertex, t: &G::Vertex) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        // stops as soon as `t` is settled
        let r = search::<G, H>(
            self.g,
            s,
            Some(t),
            |e, _, _| self.g.edge_weight(e).unwrap_or(1.0),
            |_| 0.0,
            self.radius,
        )?;
        Ok(r.path(t))
    }
}
