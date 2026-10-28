// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

/// Receives traversal events (JGraphT's `TraversalListener`). All methods default to no-ops.
#[allow(unused_variables)]
pub trait TraversalListener<V, E> {
    /// A new connected component is about to be traversed.
    fn connected_component_started(&mut self) {}
    /// The traversal of the current connected component is complete.
    fn connected_component_finished(&mut self) {}
    /// The edge `e` is traversed.
    fn edge_traversed(&mut self, e: &E) {}
    /// The vertex `v` is visited.
    fn vertex_traversed(&mut self, v: &V) {}
    /// All edges of `v` have been explored.
    fn vertex_finished(&mut self, v: &V) {}
}

/// A set of listeners with ids, used by the iterators.
pub struct TraversalListeners<'g, V, E> {
    list: Vec<(usize, Box<dyn TraversalListener<V, E> + 'g>)>,
    next_id: usize,
}

impl<V, E> Default for TraversalListeners<'_, V, E> {
    fn default() -> Self {
        TraversalListeners {
            list: Vec::new(),
            next_id: 0,
        }
    }
}

impl<'g, V, E> TraversalListeners<'g, V, E> {
    /// Registers a listener and returns the id to pass to [`remove`](Self::remove).
    pub fn add(&mut self, l: impl TraversalListener<V, E> + 'g) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.list.push((id, Box::new(l)));
        id
    }
    /// Unregisters the listener with the given id; `false` if there is none.
    pub fn remove(&mut self, id: usize) -> bool {
        let before = self.list.len();
        self.list.retain(|(i, _)| *i != id);
        before != self.list.len()
    }
    /// Whether no listener is registered.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
    pub(crate) fn component_started(&mut self) {
        self.list
            .iter_mut()
            .for_each(|(_, l)| l.connected_component_started());
    }
    pub(crate) fn component_finished(&mut self) {
        self.list
            .iter_mut()
            .for_each(|(_, l)| l.connected_component_finished());
    }
    pub(crate) fn edge_traversed(&mut self, e: &E) {
        self.list.iter_mut().for_each(|(_, l)| l.edge_traversed(e));
    }
    pub(crate) fn vertex_traversed(&mut self, v: &V) {
        self.list
            .iter_mut()
            .for_each(|(_, l)| l.vertex_traversed(v));
    }
    pub(crate) fn vertex_finished(&mut self, v: &V) {
        self.list.iter_mut().for_each(|(_, l)| l.vertex_finished(v));
    }
}
