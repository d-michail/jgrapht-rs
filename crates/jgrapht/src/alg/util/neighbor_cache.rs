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

use crate::graph::{IncidenceGraph, IncomingGraph, OutgoingGraph};
use crate::event::GraphListener;
use crate::graph::{Graph, GraphExt};
use crate::{GraphError, Result};
use indexmap::IndexMap;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

/// Neighbours of one vertex with multiplicities (parallel edges), in insertion order.
#[derive(Debug, Clone, Default)]
pub struct Neighbors<V: Eq + Hash> {
    counts: IndexMap<V, usize>,
}

impl<V: Eq + Hash + Clone> Neighbors<V> {
    fn from_list(list: impl IntoIterator<Item = V>) -> Self {
        let mut n = Neighbors {
            counts: IndexMap::new(),
        };
        for v in list {
            n.add(v);
        }
        n
    }
    fn add(&mut self, v: V) {
        *self.counts.entry(v).or_insert(0) += 1;
    }
    fn remove(&mut self, v: &V) -> Result<()> {
        match self.counts.get_mut(v) {
            None => Err(GraphError::InvalidArgument(
                "Attempting to remove a neighbor that wasn't present".into(),
            )),
            Some(c) => {
                *c -= 1;
                if *c == 0 {
                    self.counts.shift_remove(v);
                }
                Ok(())
            }
        }
    }
    /// The distinct neighbours.
    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.counts.keys()
    }
    pub fn len(&self) -> usize {
        self.counts.len()
    }
    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }
    pub fn contains(&self, v: &V) -> bool {
        self.counts.contains_key(v)
    }
    /// Neighbours repeated once per parallel edge.
    pub fn list(&self) -> Vec<V> {
        self.counts
            .iter()
            .flat_map(|(v, &c)| std::iter::repeat_n(v.clone(), c))
            .collect()
    }
}

/// Maintains a cache of each vertex's neighbors. While lists of neighbors can be obtained from
/// `Graphs`, they are re-calculated at each invocation by walking a vertex's incident edges,
/// which becomes inordinately expensive when performed often.
///
/// The cache also keeps track of successors and predecessors for each vertex. This means that the
/// result of the union of calling predecessorsOf(v) and successorsOf(v) is equal to the result of
/// calling neighborsOf(v) for a given vertex v.
///
/// # Rust notes
///
/// Caches successor / predecessor / neighbour sets of a graph (`NeighborCache`).
///
/// Sets are computed on first request. Keep the cache up to date by passing graph changes to
/// [`edge_added`](Self::edge_added) etc., or register `Rc<RefCell<NeighborCache>>` as a listener of
/// a [`Listenable`](crate::event::Listenable) graph. Unlike in Java, the returned sets are not live
/// views: query again after changing the graph.
#[derive(Debug, Clone)]
pub struct NeighborCache<V: Eq + Hash> {
    successors: HashMap<V, Neighbors<V>>,
    predecessors: HashMap<V, Neighbors<V>>,
    neighbors: HashMap<V, Neighbors<V>>,
}

impl<V: Eq + Hash + Clone> Default for NeighborCache<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Eq + Hash + Clone> NeighborCache<V> {
    /// Constructor
    pub fn new() -> Self {
        NeighborCache {
            successors: HashMap::new(),
            predecessors: HashMap::new(),
            neighbors: HashMap::new(),
        }
    }

    /// A cache that can be registered as a graph listener and queried afterwards.
    pub fn shared() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new()))
    }

    fn check<G: Graph<Vertex = V>>(g: &G, v: &V) -> Result<()> {
        if g.contains_vertex(v) {
            Ok(())
        } else {
            Err(GraphError::VertexNotFound)
        }
    }

    /// Returns the unique predecessors of the given vertex if it exists in the cache, otherwise it
    /// is initialized.
    pub fn predecessors_of<G: Graph<Vertex = V> + IncomingGraph>(&mut self, g: &G, v: &V) -> Result<&Neighbors<V>> {
        Self::check(g, v)?;
        if !self.predecessors.contains_key(v) {
            let neighbors = Neighbors::from_list(
                g.in_edges_of(v)
                    .filter_map(|e| g.opposite_vertex(e, v).cloned()),
            );
            self.predecessors.insert(v.clone(), neighbors);
        }
        Ok(&self.predecessors[v])
    }

    /// Returns the unique successors of the given vertex if it exists in the cache, otherwise it is
    /// initialized.
    pub fn successors_of<G: Graph<Vertex = V> + OutgoingGraph>(&mut self, g: &G, v: &V) -> Result<&Neighbors<V>> {
        Self::check(g, v)?;
        if !self.successors.contains_key(v) {
            let neighbors = Neighbors::from_list(
                g.out_edges_of(v)
                    .filter_map(|e| g.opposite_vertex(e, v).cloned()),
            );
            self.successors.insert(v.clone(), neighbors);
        }
        Ok(&self.successors[v])
    }

    /// Returns the unique neighbors of the given vertex if it exists in the cache, otherwise it is
    /// initialized.
    pub fn neighbors_of<G: Graph<Vertex = V> + IncidenceGraph>(&mut self, g: &G, v: &V) -> Result<&Neighbors<V>> {
        Self::check(g, v)?;
        if !self.neighbors.contains_key(v) {
            let neighbors = Neighbors::from_list(
                g.edges_of(v)
                    .filter_map(|e| g.opposite_vertex(e, v).cloned()),
            );
            self.neighbors.insert(v.clone(), neighbors);
        }
        Ok(&self.neighbors[v])
    }

    /// Neighbours repeated once per parallel edge.
    pub fn neighbor_list_of<G: Graph<Vertex = V> + IncidenceGraph>(&mut self, g: &G, v: &V) -> Result<Vec<V>> {
        Ok(self.neighbors_of(g, v)?.list())
    }

    /// Notifies that an edge has been added to the graph.
    pub fn edge_added(&mut self, source: &V, target: &V) {
        if let Some(n) = self.successors.get_mut(source) {
            n.add(target.clone());
        }
        if let Some(n) = self.predecessors.get_mut(target) {
            n.add(source.clone());
        }
        if let Some(n) = self.neighbors.get_mut(source) {
            n.add(target.clone());
        }
        if let Some(n) = self.neighbors.get_mut(target) {
            n.add(source.clone());
        }
    }

    /// Notifies that an edge has been removed from the graph.
    pub fn edge_removed(&mut self, source: &V, target: &V) -> Result<()> {
        if let Some(n) = self.successors.get_mut(source) {
            n.remove(target)?;
        }
        if let Some(n) = self.predecessors.get_mut(target) {
            n.remove(source)?;
        }
        if let Some(n) = self.neighbors.get_mut(source) {
            n.remove(target)?;
        }
        if let Some(n) = self.neighbors.get_mut(target) {
            n.remove(source)?;
        }
        Ok(())
    }

    /// Notifies that a vertex has been removed from the graph.
    pub fn vertex_removed(&mut self, v: &V) {
        self.successors.remove(v);
        self.predecessors.remove(v);
        self.neighbors.remove(v);
    }
}

impl<V: Eq + Hash + Clone, E> GraphListener<V, E> for Rc<RefCell<NeighborCache<V>>> {
    fn vertex_removed(&mut self, v: &V) {
        self.borrow_mut().vertex_removed(v);
    }
    fn edge_added(&mut self, _: &E, source: &V, target: &V) {
        self.borrow_mut().edge_added(source, target);
    }
    fn edge_removed(&mut self, _: &E, source: &V, target: &V) {
        let _ = self.borrow_mut().edge_removed(source, target);
    }
}
