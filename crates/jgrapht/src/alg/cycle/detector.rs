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

use crate::graph::IncidenceGraph;
use crate::graph::OutgoingGraph;
use crate::alg::connectivity::KosarajuStrongConnectivityInspector;
use crate::alg::interfaces::StrongConnectivityAlgorithm;
use crate::error::{GraphError, Result};
use crate::graph::{DirectedGraph, Graph, GraphExt};
use crate::traverse::{Core, CrossComponentIterator, DfsStrategy, Strategy};
use crate::walk::GraphWalk;
use indexmap::IndexSet;
use std::collections::HashMap;

/// Performs cycle detection on a graph. The *inspected graph* is specified at construction time
/// and cannot be modified. Currently, the detector supports only directed graphs.
///
/// # Rust notes
///
/// `CycleDetector`: finds cycles of a directed graph, either anywhere or through one vertex.
pub struct CycleDetector<'g, G: DirectedGraph> {
    graph: &'g G,
}

/// The depth-first traversal of `ProbeIterator`: keeps the current path and reports every
/// back-reference that closes a cycle.
struct ProbeStrategy<'g, G: Graph + 'g + IncidenceGraph> {
    inner: DfsStrategy<'g, G>,
    path: Vec<&'g G::Vertex>,
    cycle_set: Option<IndexSet<G::Vertex>>,
    root: Option<&'g G::Vertex>,
    detected: bool,
}

impl<'g, G: Graph + 'g + IncidenceGraph> Strategy<'g, G> for ProbeStrategy<'g, G> {
    type Data = <DfsStrategy<'g, G> as Strategy<'g, G>>::Data;

    fn is_exhausted(&mut self, c: &mut Core<'g, G, Self::Data>) -> bool {
        self.inner.is_exhausted(c)
    }

    fn encounter(
        &mut self,
        c: &mut Core<'g, G, Self::Data>,
        v: &'g G::Vertex,
        e: Option<&'g G::Edge>,
    ) {
        self.inner.encounter(c, v, e)
    }

    fn encounter_again(
        &mut self,
        c: &mut Core<'g, G, Self::Data>,
        v: &'g G::Vertex,
        e: &'g G::Edge,
    ) {
        self.inner.encounter_again(c, v, e);
        let mut i = if let Some(root) = self.root {
            if v == root || self.cycle_set.as_ref().is_some_and(|s| s.contains(v)) {
                0
            } else {
                return;
            }
        } else {
            match self.path.iter().position(|p| *p == v) {
                Some(i) => i,
                None => return,
            }
        };
        match &mut self.cycle_set {
            None => self.detected = true,
            Some(set) => {
                while i < self.path.len() {
                    set.insert(self.path[i].clone());
                    i += 1;
                }
            }
        }
    }

    fn provide_next(&mut self, c: &mut Core<'g, G, Self::Data>) -> &'g G::Vertex {
        let v = self.inner.provide_next(c);
        while let Some(last) = self.path.last() {
            if c.g.contains_edge_between(last, v) {
                break;
            }
            self.path.pop();
        }
        self.path.push(v);
        v
    }
}

impl<'g, G: DirectedGraph + 'g + OutgoingGraph + IncidenceGraph> CycleDetector<'g, G> {
    /// Creates a cycle detector for the specified graph. Currently only directed graphs are
    /// supported.
    pub fn new(graph: &'g G) -> Self {
        CycleDetector { graph }
    }

    /// Runs the probe; with no `cycle_set` it stops at the first cycle and returns `true`.
    fn execute(
        &self,
        cycle_set: Option<IndexSet<G::Vertex>>,
        v: Option<&'g G::Vertex>,
    ) -> Result<(bool, Option<IndexSet<G::Vertex>>)> {
        let strategy = ProbeStrategy {
            inner: DfsStrategy::new(),
            path: Vec::new(),
            cycle_set,
            root: v,
            detected: false,
        };
        let starts = v.map(|v| vec![v]);
        let mut it = CrossComponentIterator::with_strategy(self.graph, starts, strategy)?;
        while it.next().is_some() {
            if it.strategy().detected {
                return Ok((true, None));
            }
        }
        Ok((false, it.into_strategy().cycle_set))
    }

    /// Whether the graph has a cycle.
    pub fn detect_cycles(&self) -> bool {
        self.execute(None, None).map(|r| r.0).unwrap_or(false)
    }

    /// Whether some cycle goes through `v`.
    pub fn detect_cycles_containing_vertex(&self, v: &'g G::Vertex) -> bool {
        self.execute(None, Some(v)).map(|r| r.0).unwrap_or(false)
    }

    /// All vertices on some cycle: the non-trivial strongly connected components and self-loops.
    pub fn find_cycles(&self) -> IndexSet<G::Vertex> {
        let inspector = KosarajuStrongConnectivityInspector::new(self.graph);
        let mut set = IndexSet::new();
        for component in inspector.strongly_connected_sets() {
            if component.len() > 1 {
                set.extend(
                    self.graph
                        .vertices()
                        .filter(|v| component.contains(*v))
                        .cloned(),
                );
            } else {
                let v = component.iter().next().expect("non-empty component");
                if self.graph.contains_edge_between(v, v) {
                    set.insert(v.clone());
                }
            }
        }
        set
    }

    /// The vertices on cycles through `v`.
    pub fn find_cycles_containing_vertex(&self, v: &'g G::Vertex) -> IndexSet<G::Vertex> {
        self.execute(Some(IndexSet::new()), Some(v))
            .ok()
            .and_then(|r| r.1)
            .unwrap_or_default()
    }
}

/// `Cycles.simpleCycleToGraphPath`: orders the edges of a simple cycle into a closed walk.
/// `None` for an empty cycle; an error if the edges are not a simple cycle.
pub fn simple_cycle_to_graph_path<G: Graph>(
    graph: &G,
    cycle: &[G::Edge],
) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
    if cycle.is_empty() {
        return Ok(None);
    }
    let not_simple = || GraphError::InvalidArgument("Not a simple cycle".into());
    let mut first_edge: HashMap<&G::Vertex, &G::Edge> = HashMap::new();
    let mut second_edge: HashMap<&G::Vertex, &G::Edge> = HashMap::new();
    for e in cycle {
        let (s, t) = graph.edge_endpoints(e).ok_or(GraphError::EdgeNotFound)?;
        for v in [s, t] {
            if let std::collections::hash_map::Entry::Vacant(slot) = first_edge.entry(v) {
                slot.insert(e);
            } else if let std::collections::hash_map::Entry::Vacant(slot) = second_edge.entry(v) {
                slot.insert(e);
            } else {
                return Err(not_simple());
            }
        }
    }
    let mut edges = Vec::new();
    let mut weight = 0.0;
    let mut e = &cycle[0];
    edges.push(e.clone());
    weight += graph.edge_weight(e).unwrap_or(1.0);
    let start = graph.edge_source(e).ok_or(GraphError::EdgeNotFound)?;
    let mut cur = graph
        .opposite_vertex(e, start)
        .ok_or(GraphError::EdgeNotFound)?;
    while cur != start {
        let fe = *first_edge.get(cur).ok_or_else(not_simple)?;
        let se = *second_edge.get(cur).ok_or_else(not_simple)?;
        e = if fe == e {
            se
        } else if se == e {
            fe
        } else {
            return Err(not_simple());
        };
        edges.push(e.clone());
        weight += graph.edge_weight(e).unwrap_or(1.0);
        cur = graph
            .opposite_vertex(e, cur)
            .ok_or(GraphError::EdgeNotFound)?;
    }
    Ok(Some(
        GraphWalk::from_edges(graph, start.clone(), edges, weight).ok_or_else(not_simple)?,
    ))
}
