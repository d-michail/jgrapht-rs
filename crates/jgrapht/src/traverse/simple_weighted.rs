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
use crate::util::OrderedF64;
use crate::{GraphError, Result, WeightProblem};
use rand::Rng;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Visits vertices in order of increasing distance from the start (Dijkstra order), yielding
/// `(vertex, distance)`. Edge weights must be non-negative.
///
/// A negative or NaN weight is reported as one `Err(GraphError::InvalidWeight(..))` item in place
/// of the vertex whose outgoing edge carries it, after which the iterator ends; collect with
/// `collect::<Result<Vec<_>>>()`.
pub struct ClosestFirst<'g, G: Graph> {
    g: &'g G,
    heap: BinaryHeap<Reverse<(OrderedF64, usize)>>,
    nodes: Vec<&'g G::Vertex>,
    dist: HashMap<&'g G::Vertex, f64>,
    done: HashSet<&'g G::Vertex>,
    radius: f64,
    failed: bool,
}

impl<'g, G: Graph> ClosestFirst<'g, G> {
    /// Visits every vertex reachable from `start`.
    ///
    /// # Errors
    ///
    /// [`GraphError::VertexNotFound`] if `start` is not a vertex of `g`.
    pub fn new(g: &'g G, start: &'g G::Vertex) -> Result<Self> {
        Self::with_radius(g, start, f64::INFINITY)
    }
    /// Only vertices within `radius` of `start` are visited.
    ///
    /// # Errors
    ///
    /// [`GraphError::VertexNotFound`] if `start` is not a vertex of `g`, and
    /// [`GraphError::InvalidParameter`] if `radius` is negative or NaN.
    pub fn with_radius(g: &'g G, start: &'g G::Vertex, radius: f64) -> Result<Self> {
        if !g.contains_vertex(start) {
            return Err(GraphError::VertexNotFound);
        }
        if radius.is_nan() || radius < 0.0 {
            return Err(GraphError::must_be_non_negative("radius"));
        }
        let mut c = ClosestFirst {
            g,
            heap: BinaryHeap::new(),
            nodes: vec![start],
            dist: HashMap::from([(start, 0.0)]),
            done: HashSet::new(),
            radius,
            failed: false,
        };
        c.heap.push(Reverse((OrderedF64(0.0), 0)));
        Ok(c)
    }
}

impl<'g, G: OutgoingGraph> Iterator for ClosestFirst<'g, G> {
    type Item = Result<(&'g G::Vertex, f64)>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        let g = self.g;
        while let Some(Reverse((OrderedF64(d), i))) = self.heap.pop() {
            let v = self.nodes[i];
            if !self.done.insert(v) {
                continue;
            }
            for (e, n) in g.outgoing_of(v) {
                let w = g.weight_of(e);
                if w.is_nan() || w < 0.0 {
                    self.failed = true;
                    return Some(Err(GraphError::InvalidWeight(WeightProblem::NegativeOrNaN)));
                }
                let nd = d + w;
                if nd <= self.radius && self.dist.get(n).is_none_or(|&old| nd < old) {
                    self.dist.insert(n, nd);
                    self.nodes.push(n);
                    self.heap
                        .push(Reverse((OrderedF64(nd), self.nodes.len() - 1)));
                }
            }
            return Some(Ok((v, d)));
        }
        None
    }
}

/// Random walk of at most `max_steps` edges; yields the start vertex first and stops early at
/// a dead end. With `weighted`, edges are picked proportionally to their weight.
///
/// A negative or NaN weight on the edges leaving the current vertex is reported as one
/// `Err(GraphError::InvalidWeight(..))` item, after which the walk ends.
pub struct RandomWalk<'g, G: Graph, R: Rng> {
    g: &'g G,
    cur: Option<&'g G::Vertex>,
    steps_left: usize,
    weighted: bool,
    rng: R,
}

impl<'g, G: Graph, R: Rng> RandomWalk<'g, G, R> {
    /// # Errors
    ///
    /// [`GraphError::VertexNotFound`] if `start` is not a vertex of `g`.
    pub fn new(
        g: &'g G,
        start: &'g G::Vertex,
        max_steps: usize,
        weighted: bool,
        rng: R,
    ) -> Result<Self> {
        if !g.contains_vertex(start) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(RandomWalk {
            g,
            cur: Some(start),
            steps_left: max_steps,
            weighted,
            rng,
        })
    }
}

impl<'g, G: OutgoingGraph, R: Rng> Iterator for RandomWalk<'g, G, R> {
    type Item = Result<&'g G::Vertex>;
    fn next(&mut self) -> Option<Self::Item> {
        let v = self.cur?;
        self.cur = None;
        if self.steps_left > 0 {
            let g = self.g;
            let options: Vec<_> = g.outgoing_of(v).collect();
            if !options.is_empty() {
                self.steps_left -= 1;
                let pick = if self.weighted {
                    if options.iter().any(|(e, _)| {
                        g.edge_weight(e)
                            .is_some_and(|w| w.is_nan() || w < 0.0)
                    }) {
                        return Some(Err(GraphError::InvalidWeight(WeightProblem::NegativeOrNaN)));
                    }
                    let total: f64 = options
                        .iter()
                        .map(|(e, _)| g.weight_of(e))
                        .sum();
                    let mut x = self.rng.random::<f64>() * total;
                    options
                        .iter()
                        .find(|(e, _)| {
                            x -= g.weight_of(e);
                            x < 0.0
                        })
                        .unwrap_or(options.last().unwrap())
                } else {
                    &options[self.rng.random_range(0..options.len())]
                };
                self.cur = Some(pick.1);
            }
        }
        Some(Ok(v))
    }
}
