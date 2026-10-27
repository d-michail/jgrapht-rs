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

//! Graph change notification (JGraphT's `ListenableGraph`).

use crate::graph::{DirectedGraph, Graph, GraphExt, GraphMut, UndirectedGraph};
use crate::{GraphType, Result};

/// Receives change events. All methods default to no-ops.
#[allow(unused_variables)]
pub trait GraphListener<V, E> {
    fn vertex_added(&mut self, v: &V) {}
    fn vertex_removed(&mut self, v: &V) {}
    fn edge_added(&mut self, e: &E, source: &V, target: &V) {}
    fn edge_removed(&mut self, e: &E, source: &V, target: &V) {}
    fn edge_weight_changed(&mut self, e: &E, old: f64, new: f64) {}
}

/// Wraps a mutable graph and notifies listeners about every successful change.
pub struct Listenable<G: Graph> {
    graph: G,
    listeners: Vec<(usize, Box<dyn GraphListener<G::Vertex, G::Edge>>)>,
    next_listener: usize,
}

impl<G: Graph> Listenable<G> {
    pub fn new(graph: G) -> Self {
        Listenable {
            graph,
            listeners: Vec::new(),
            next_listener: 0,
        }
    }
    /// Registers a listener and returns an id for [`Listenable::remove_listener`].
    pub fn add_listener(&mut self, l: impl GraphListener<G::Vertex, G::Edge> + 'static) -> usize {
        let id = self.next_listener;
        self.next_listener += 1;
        self.listeners.push((id, Box::new(l)));
        id
    }
    /// Unregisters a listener; returns whether it was registered.
    pub fn remove_listener(&mut self, id: usize) -> bool {
        let before = self.listeners.len();
        self.listeners.retain(|(i, _)| *i != id);
        self.listeners.len() != before
    }
    pub fn clear_listeners(&mut self) {
        self.listeners.clear();
    }
    pub fn inner(&self) -> &G {
        &self.graph
    }
    pub fn into_inner(self) -> G {
        self.graph
    }
}

impl<G: Graph> Graph for Listenable<G> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    fn graph_type(&self) -> GraphType {
        self.graph.graph_type()
    }
    fn vertices(&self) -> impl Iterator<Item = &G::Vertex> {
        self.graph.vertices()
    }
    fn edges(&self) -> impl Iterator<Item = &G::Edge> {
        self.graph.edges()
    }
    fn vertex_count(&self) -> usize {
        self.graph.vertex_count()
    }
    fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
    fn contains_vertex(&self, v: &G::Vertex) -> bool {
        self.graph.contains_vertex(v)
    }
    fn contains_edge(&self, e: &G::Edge) -> bool {
        self.graph.contains_edge(e)
    }
    fn edge_source(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.graph.edge_source(e)
    }
    fn edge_target(&self, e: &G::Edge) -> Option<&G::Vertex> {
        self.graph.edge_target(e)
    }
    fn edge_weight(&self, e: &G::Edge) -> Option<f64> {
        self.graph.edge_weight(e)
    }
    fn edges_of(&self, v: &G::Vertex) -> Result<impl Iterator<Item = &G::Edge>> {
        self.graph.edges_of(v)
    }
    fn in_edges_of(&self, v: &G::Vertex) -> Result<impl Iterator<Item = &G::Edge>> {
        self.graph.in_edges_of(v)
    }
    fn out_edges_of(&self, v: &G::Vertex) -> Result<impl Iterator<Item = &G::Edge>> {
        self.graph.out_edges_of(v)
    }
    fn in_degree_of(&self, v: &G::Vertex) -> Result<usize> {
        self.graph.in_degree_of(v)
    }
    fn out_degree_of(&self, v: &G::Vertex) -> Result<usize> {
        self.graph.out_degree_of(v)
    }
    fn degree_of(&self, v: &G::Vertex) -> Result<usize> {
        self.graph.degree_of(v)
    }
    fn edges_between<'a>(
        &'a self,
        u: &'a G::Vertex,
        v: &'a G::Vertex,
    ) -> impl Iterator<Item = &'a G::Edge> {
        self.graph.edges_between(u, v)
    }
}
crate::view::forward_incidence!([G: Graph] Listenable<G>, G, self => self.graph);
impl<G: DirectedGraph> DirectedGraph for Listenable<G> {}
impl<G: UndirectedGraph> UndirectedGraph for Listenable<G> {}

impl<G: GraphMut> GraphMut for Listenable<G> {
    fn add_vertex(&mut self, v: G::Vertex) -> bool {
        let added = self.graph.add_vertex(v.clone());
        if added {
            self.listeners
                .iter_mut()
                .for_each(|(_, l)| l.vertex_added(&v));
        }
        added
    }

    fn remove_vertex(&mut self, v: &G::Vertex) -> bool {
        if !self.graph.contains_vertex(v) {
            return false;
        }
        let incident: Vec<_> = self.graph.edges_of(v).unwrap().cloned().collect();
        for e in &incident {
            self.remove_edge(e);
        }
        self.graph.remove_vertex(v);
        self.listeners
            .iter_mut()
            .for_each(|(_, l)| l.vertex_removed(v));
        true
    }

    fn add_edge_with(&mut self, u: &G::Vertex, v: &G::Vertex, e: G::Edge) -> Result<bool> {
        let added = self.graph.add_edge_with(u, v, e.clone())?;
        if added {
            self.listeners
                .iter_mut()
                .for_each(|(_, l)| l.edge_added(&e, u, v));
        }
        Ok(added)
    }

    fn remove_edge(&mut self, e: &G::Edge) -> bool {
        let Some((s, t)) = self
            .graph
            .edge_endpoints(e)
            .map(|(s, t)| (s.clone(), t.clone()))
        else {
            return false;
        };
        self.graph.remove_edge(e);
        self.listeners
            .iter_mut()
            .for_each(|(_, l)| l.edge_removed(e, &s, &t));
        true
    }

    fn set_edge_weight(&mut self, e: &G::Edge, w: f64) -> Result<()> {
        let old = self.graph.edge_weight(e);
        self.graph.set_edge_weight(e, w)?;
        if let Some(old) = old {
            self.listeners
                .iter_mut()
                .for_each(|(_, l)| l.edge_weight_changed(e, old, w));
        }
        Ok(())
    }

    fn new_edge(&mut self) -> Result<G::Edge> {
        self.graph.new_edge()
    }

    fn new_vertex(&mut self) -> Result<G::Vertex> {
        self.graph.new_vertex()
    }
}
