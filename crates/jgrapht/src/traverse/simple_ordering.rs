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

//! Vertex orderings that ignore edge direction.

use crate::graph::IncidenceGraph;
use crate::graph::{Graph, GraphExt};
use std::collections::{HashMap, HashSet};

fn neighbor_set<'g, G: IncidenceGraph>(g: &'g G, v: &'g G::Vertex) -> HashSet<&'g G::Vertex> {
    g.neighbors_of(v).filter(|n| *n != v).collect()
}

/// Lexicographic breadth-first search (partition refinement); the building block for chordality
/// testing and perfect elimination orderings.
pub struct LexBfs<'g, G: Graph> {
    g: &'g G,
    parts: Vec<Vec<&'g G::Vertex>>,
}

impl<'g, G: Graph> LexBfs<'g, G> {
    pub fn new(g: &'g G) -> Self {
        LexBfs {
            g,
            parts: vec![g.vertices().collect()],
        }
    }
}

impl<'g, G: IncidenceGraph> Iterator for LexBfs<'g, G> {
    type Item = &'g G::Vertex;
    fn next(&mut self) -> Option<Self::Item> {
        let first = self.parts.iter_mut().find(|p| !p.is_empty())?;
        let v = first.remove(0);
        let nb = neighbor_set(self.g, v);
        let mut next = Vec::with_capacity(self.parts.len() * 2);
        for p in self.parts.drain(..) {
            let (inn, out): (Vec<_>, Vec<_>) = p.into_iter().partition(|x| nb.contains(x));
            for part in [inn, out] {
                if !part.is_empty() {
                    next.push(part);
                }
            }
        }
        self.parts = next;
        Some(v)
    }
}

/// Maximum cardinality search: always picks the vertex with most already-visited neighbours.
pub struct MaximumCardinality<'g, G: Graph> {
    g: &'g G,
    remaining: Vec<&'g G::Vertex>,
    count: HashMap<&'g G::Vertex, usize>,
}

impl<'g, G: Graph> MaximumCardinality<'g, G> {
    pub fn new(g: &'g G) -> Self {
        MaximumCardinality {
            g,
            remaining: g.vertices().collect(),
            count: g.vertices().map(|v| (v, 0)).collect(),
        }
    }
}

impl<'g, G: IncidenceGraph> Iterator for MaximumCardinality<'g, G> {
    type Item = &'g G::Vertex;
    fn next(&mut self) -> Option<Self::Item> {
        // first maximum wins, keeping the order deterministic
        let idx = (0..self.remaining.len()).fold(None::<usize>, |best, i| match best {
            Some(b) if self.count[self.remaining[b]] >= self.count[self.remaining[i]] => Some(b),
            _ => Some(i),
        })?;
        let v = self.remaining.remove(idx);
        for n in neighbor_set(self.g, v) {
            if let Some(c) = self.count.get_mut(n) {
                *c += 1;
            }
        }
        Some(v)
    }
}

/// Repeatedly removes a minimum-degree vertex (degeneracy ordering). Self-loops and parallel
/// edges are ignored.
pub struct DegeneracyOrdering<'g, G: Graph> {
    g: &'g G,
    remaining: Vec<&'g G::Vertex>,
    degree: HashMap<&'g G::Vertex, usize>,
    degeneracy: usize,
}

impl<'g, G: IncidenceGraph> DegeneracyOrdering<'g, G> {
    pub fn new(g: &'g G) -> Self {
        DegeneracyOrdering {
            g,
            remaining: g.vertices().collect(),
            degree: g
                .vertices()
                .map(|v| (v, neighbor_set(g, v).len()))
                .collect(),
            degeneracy: 0,
        }
    }
    /// Largest minimum degree seen so far; the graph's degeneracy once exhausted.
    pub fn degeneracy(&self) -> usize {
        self.degeneracy
    }
}

impl<'g, G: IncidenceGraph> Iterator for DegeneracyOrdering<'g, G> {
    type Item = &'g G::Vertex;
    fn next(&mut self) -> Option<Self::Item> {
        let idx = (0..self.remaining.len()).min_by_key(|&i| self.degree[self.remaining[i]])?;
        let v = self.remaining.remove(idx);
        self.degeneracy = self.degeneracy.max(self.degree[v]);
        for n in neighbor_set(self.g, v) {
            if let Some(d) = self.degree.get_mut(n) {
                *d = d.saturating_sub(1);
            }
        }
        Some(v)
    }
}
