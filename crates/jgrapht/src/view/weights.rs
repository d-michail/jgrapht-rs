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

use super::{forward_graph, forward_graph_mut};
use crate::graph::{DirectedGraph, Graph, GraphMut, UndirectedGraph};
use crate::{GraphError, GraphType, Result};
use std::cell::RefCell;
use std::collections::HashMap;

/// Overlays edge weights on any graph (JGraphT's `AsWeightedGraph`).
///
/// Weights come from a map or a function (optionally cached). Edges without an overlay weight
/// report the base graph's weight. [`GraphMut::set_edge_weight`] records the weight in the overlay
/// and, with `write_through`, also in the base graph.
pub struct AsWeighted<'f, G: Graph> {
    graph: G,
    weights: RefCell<HashMap<G::Edge, f64>>,
    function: Option<Box<dyn Fn(&G::Edge) -> f64 + 'f>>,
    cache: bool,
    write_through: bool,
}

impl<'f, G: Graph> AsWeighted<'f, G> {
    /// Weights from a map. Writes go through to the base iff the base graph is weighted.
    pub fn new(graph: G, weights: HashMap<G::Edge, f64>) -> Self {
        let write_through = graph.graph_type().weighted;
        AsWeighted {
            graph,
            weights: RefCell::new(weights),
            function: None,
            cache: false,
            write_through,
        }
    }

    /// Weights from a map with explicit write-through; requires a weighted base if enabled.
    pub fn with_write_through(
        graph: G,
        weights: HashMap<G::Edge, f64>,
        write_through: bool,
    ) -> Result<Self> {
        if write_through {
            crate::graph::props::require_weighted(&graph)?;
        }
        Ok(AsWeighted {
            graph,
            weights: RefCell::new(weights),
            function: None,
            cache: false,
            write_through,
        })
    }

    /// Weights from a function; with `cache` the computed weights are remembered (and may be
    /// overwritten by `set_edge_weight`).
    pub fn from_function(
        graph: G,
        function: impl Fn(&G::Edge) -> f64 + 'f,
        cache: bool,
        write_through: bool,
    ) -> Result<Self> {
        if write_through {
            crate::graph::props::require_weighted(&graph)?;
        }
        Ok(AsWeighted {
            graph,
            weights: RefCell::new(HashMap::new()),
            function: Some(Box::new(function)),
            cache,
            write_through,
        })
    }

    /// Records a weight in the overlay only (never in the base), usable on read-only bases.
    pub fn set_overlay_weight(&self, e: G::Edge, w: f64) -> Result<()> {
        if self.function.is_some() && !self.cache {
            return Err(GraphError::Unsupported(
                "Cannot set an edge weight when a weight function is used and caching is disabled"
                    .into(),
            ));
        }
        self.weights.borrow_mut().insert(e, w);
        Ok(())
    }

    pub fn base(&self) -> &G {
        &self.graph
    }
}

impl<G: Graph> Graph for AsWeighted<'_, G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.graph.graph_type().as_weighted()
    }
    forward_graph!(self => self.graph; vertices, edges, vertex_count, edge_count, contains_vertex, contains_edge,
        edge_source, edge_target, is_edge_directed, edges_of, in_edges_of, out_edges_of, in_degree_of, out_degree_of,
        degree_of, edges_between);
    fn edge_weight(&self, e: &G::Edge) -> Option<f64> {
        if !self.graph.contains_edge(e) {
            return None;
        }
        let own = match &self.function {
            Some(f) if self.cache => Some(
                *self
                    .weights
                    .borrow_mut()
                    .entry(e.clone())
                    .or_insert_with(|| f(e)),
            ),
            Some(f) => Some(f(e)),
            None => self.weights.borrow().get(e).copied(),
        };
        own.or_else(|| self.graph.edge_weight(e))
    }
}
impl<G: DirectedGraph> DirectedGraph for AsWeighted<'_, G> {}
impl<G: UndirectedGraph> UndirectedGraph for AsWeighted<'_, G> {}

impl<G: GraphMut> GraphMut for AsWeighted<'_, G> {
    forward_graph_mut!(self => self.graph; add_vertex, remove_vertex, add_edge_with, remove_edge, new_edge, new_vertex);
    fn set_edge_weight(&mut self, e: &G::Edge, w: f64) -> Result<()> {
        self.set_overlay_weight(e.clone(), w)?;
        if self.write_through {
            self.graph.set_edge_weight(e, w)?;
        }
        Ok(())
    }
}

/// Treats every edge weight as `1.0`; setting weights is unsupported.
#[derive(Debug, Clone)]
pub struct AsUnweighted<G>(pub G);

impl<G: Graph> Graph for AsUnweighted<G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.0.graph_type().as_unweighted()
    }
    forward_graph!(self => self.0; vertices, edges, vertex_count, edge_count, contains_vertex, contains_edge,
        edge_source, edge_target, is_edge_directed, edges_of, in_edges_of, out_edges_of, in_degree_of, out_degree_of,
        degree_of, edges_between);
    fn edge_weight(&self, e: &G::Edge) -> Option<f64> {
        self.0.contains_edge(e).then_some(1.0)
    }
}
impl<G: DirectedGraph> DirectedGraph for AsUnweighted<G> {}
impl<G: UndirectedGraph> UndirectedGraph for AsUnweighted<G> {}
impl<G: GraphMut> GraphMut for AsUnweighted<G> {
    forward_graph_mut!(self => self.0; add_vertex, remove_vertex, add_edge_with, remove_edge, new_edge, new_vertex);
    fn set_edge_weight(&mut self, _: &G::Edge, _: f64) -> Result<()> {
        Err(GraphError::Unsupported(
            "Edge weight is not supported".into(),
        ))
    }
}
