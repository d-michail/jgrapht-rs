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
use crate::graph::Graph;
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use rheaps::DecreaseKeyHeap;
use rheaps::tree::PairingHeap;
use std::marker::PhantomData;

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
    G: Graph,
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
