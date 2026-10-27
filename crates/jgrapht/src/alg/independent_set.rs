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

//! Independent set algorithms (`org.jgrapht.alg.independentset`).

use super::cycle::{ChordalityInspector, IterationOrder};
use crate::graph::{GraphExt, UndirectedGraph};
use std::collections::HashSet;

/// Maximum independent set of a chordal graph (`ChordalGraphIndependentSetFinder`): the vertices
/// of a perfect elimination order are scanned from the end, taking each one that has no neighbour
/// in the set so far.
pub struct ChordalGraphIndependentSetFinder<'g, G: UndirectedGraph> {
    graph: &'g G,
    chordality_inspector: ChordalityInspector<'g, G>,
    maximum_independent_set: Option<HashSet<G::Vertex>>,
}

impl<'g, G: UndirectedGraph> ChordalGraphIndependentSetFinder<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self::with_order(graph, IterationOrder::Mcs)
    }

    pub fn with_order(graph: &'g G, iteration_order: IterationOrder) -> Self {
        ChordalGraphIndependentSetFinder {
            graph,
            chordality_inspector: ChordalityInspector::with_order(graph, iteration_order),
            maximum_independent_set: None,
        }
    }

    fn lazy_compute_maximum_independent_set(&mut self) {
        if self.maximum_independent_set.is_none() && self.chordality_inspector.is_chordal() {
            // iterate the order from the end to the beginning; choose vertices that don't have
            // neighbors in the current independent set; add all their neighbors to the
            // restricted set
            let mut restricted: HashSet<G::Vertex> = HashSet::new();
            let mut is: HashSet<G::Vertex> = HashSet::new();
            let perfect_elimination_order = self
                .chordality_inspector
                .get_perfect_elimination_order()
                .expect("chordal graph");
            for previous in perfect_elimination_order.iter().rev() {
                if !restricted.contains(previous) {
                    is.insert(previous.clone());
                    for edge in self.graph.edges_of(previous).unwrap() {
                        let opposite = self
                            .graph
                            .opposite_vertex(edge, previous)
                            .expect("incident edge");
                        if previous != opposite {
                            restricted.insert(opposite.clone());
                        }
                    }
                }
            }
            self.maximum_independent_set = Some(is);
        }
    }

    /// A maximum independent set, or `None` if the graph is not chordal.
    pub fn independent_set(&mut self) -> Option<HashSet<G::Vertex>> {
        self.lazy_compute_maximum_independent_set();
        self.maximum_independent_set.clone()
    }
}
