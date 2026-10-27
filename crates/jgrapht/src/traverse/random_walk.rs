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
use crate::graph::{Graph, GraphExt};
use crate::util::java::RandomSource;
use crate::{GraphError, Result};
use std::collections::HashMap;

/// Random walk over outgoing edges (JGraphT's `RandomWalkVertexIterator`): yields the start
/// vertex, then up to `max_hops` further vertices, stopping early at a sink.
pub struct RandomWalkVertexIterator<'g, G: Graph + 'g, R> {
    g: &'g G,
    weighted: bool,
    out_weight: HashMap<&'g G::Vertex, f64>,
    max_hops: u64,
    hops: u64,
    next_vertex: Option<&'g G::Vertex>,
    rng: R,
}

impl<'g, G: OutgoingGraph + 'g + OutgoingGraph, R: RandomSource> RandomWalkVertexIterator<'g, G, R> {
    pub fn new(
        g: &'g G,
        start: &'g G::Vertex,
        max_hops: u64,
        weighted: bool,
        rng: R,
    ) -> Result<Self> {
        if !g.contains_vertex(start) {
            return Err(GraphError::InvalidArgument(
                "Random walk must start at a graph vertex".into(),
            ));
        }
        Ok(RandomWalkVertexIterator {
            g,
            weighted,
            out_weight: HashMap::new(),
            max_hops,
            hops: 0,
            next_vertex: Some(start),
            rng,
        })
    }

    fn compute_next(&mut self) {
        let Some(cur) = self.next_vertex else { return };
        if self.hops >= self.max_hops {
            self.next_vertex = None;
            return;
        }
        self.hops += 1;
        let g = self.g;
        if g.out_degree_of(cur) == 0 {
            self.next_vertex = None;
            return;
        }
        let edge = if self.weighted {
            let total = *self.out_weight.entry(cur).or_insert_with(|| {
                g.out_edges_of(cur)
                    .map(|e| g.edge_weight(e).unwrap_or(1.0))
                    .sum()
            });
            let p = total * self.rng.next_double();
            let mut cumulative = 0.0;
            let mut picked = None;
            for e in g.out_edges_of(cur) {
                cumulative += g.edge_weight(e).unwrap_or(1.0);
                if p <= cumulative {
                    picked = Some(e);
                    break;
                }
            }
            picked
        } else {
            let edges: Vec<_> = g.out_edges_of(cur).collect();
            Some(edges[self.rng.next_int_bound(edges.len())])
        };
        self.next_vertex = edge.and_then(|e| g.opposite_vertex(e, cur));
    }
}

impl<'g, G: OutgoingGraph + 'g, R: RandomSource> Iterator for RandomWalkVertexIterator<'g, G, R> {
    type Item = &'g G::Vertex;
    fn next(&mut self) -> Option<&'g G::Vertex> {
        let v = self.next_vertex?;
        self.compute_next();
        Some(v)
    }
}
