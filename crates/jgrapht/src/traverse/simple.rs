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

//! Lightweight traversal iterators (no listeners; neighbours are visited in a simple
//! first-neighbour-first order). See the sibling modules for the JGraphT-faithful iterators.

use crate::graph::{DirectedGraph, Graph, GraphExt};
use std::collections::{HashMap, HashSet, VecDeque};
use std::vec::IntoIter;

/// Breadth-first search.
pub struct Bfs<'g, G: Graph> {
    g: &'g G,
    queue: VecDeque<&'g G::Vertex>,
    seen: HashSet<&'g G::Vertex>,
    depth: HashMap<&'g G::Vertex, usize>,
    rest: IntoIter<&'g G::Vertex>,
}

impl<'g, G: Graph> Bfs<'g, G> {
    pub fn new(g: &'g G, start: &'g G::Vertex) -> Self {
        let mut b = Bfs {
            g,
            queue: VecDeque::new(),
            seen: HashSet::new(),
            depth: HashMap::new(),
            rest: Vec::new().into_iter(),
        };
        b.enqueue_root(start);
        b
    }
    /// Covers every component, starting from the vertices in graph order.
    pub fn all(g: &'g G) -> Self {
        Bfs {
            g,
            queue: VecDeque::new(),
            seen: HashSet::new(),
            depth: HashMap::new(),
            rest: g.vertices().collect::<Vec<_>>().into_iter(),
        }
    }
    fn enqueue_root(&mut self, v: &'g G::Vertex) {
        self.seen.insert(v);
        self.depth.insert(v, 0);
        self.queue.push_back(v);
    }
    /// Hop distance from the root of its search tree, for vertices discovered so far.
    pub fn depth(&self, v: &G::Vertex) -> Option<usize> {
        self.depth.get(v).copied()
    }
}

impl<'g, G: Graph> Iterator for Bfs<'g, G> {
    type Item = &'g G::Vertex;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(v) = self.queue.pop_front() {
                let g = self.g;
                let d = self.depth[v];
                for n in g.out_neighbors_of(v) {
                    if self.seen.insert(n) {
                        self.depth.insert(n, d + 1);
                        self.queue.push_back(n);
                    }
                }
                return Some(v);
            }
            let root = self.rest.find(|v| !self.seen.contains(v))?;
            self.enqueue_root(root);
        }
    }
}

/// Depth-first search, pre-order.
pub struct Dfs<'g, G: Graph> {
    g: &'g G,
    stack: Vec<&'g G::Vertex>,
    seen: HashSet<&'g G::Vertex>,
    rest: IntoIter<&'g G::Vertex>,
}

impl<'g, G: Graph> Dfs<'g, G> {
    pub fn new(g: &'g G, start: &'g G::Vertex) -> Self {
        Dfs {
            g,
            stack: vec![start],
            seen: HashSet::new(),
            rest: Vec::new().into_iter(),
        }
    }
    pub fn all(g: &'g G) -> Self {
        Dfs {
            g,
            stack: Vec::new(),
            seen: HashSet::new(),
            rest: g.vertices().collect::<Vec<_>>().into_iter(),
        }
    }
}

impl<'g, G: Graph> Iterator for Dfs<'g, G> {
    type Item = &'g G::Vertex;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            while let Some(v) = self.stack.pop() {
                if !self.seen.insert(v) {
                    continue;
                }
                let g = self.g;
                let nb: Vec<_> = g
                    .out_neighbors_of(v)
                    .filter(|n| !self.seen.contains(n))
                    .collect();
                self.stack.extend(nb.into_iter().rev());
                return Some(v);
            }
            let root = self.rest.find(|v| !self.seen.contains(v))?;
            self.stack.push(root);
        }
    }
}

/// Depth-first search, post-order (a vertex is yielded after all vertices below it).
pub struct DfsPostOrder<'g, G: Graph> {
    g: &'g G,
    stack: Vec<(&'g G::Vertex, IntoIter<&'g G::Vertex>)>,
    seen: HashSet<&'g G::Vertex>,
    rest: IntoIter<&'g G::Vertex>,
}

impl<'g, G: Graph> DfsPostOrder<'g, G> {
    pub fn new(g: &'g G, start: &'g G::Vertex) -> Self {
        let mut s = Self {
            g,
            stack: Vec::new(),
            seen: HashSet::new(),
            rest: Vec::new().into_iter(),
        };
        s.push(start);
        s
    }
    pub fn all(g: &'g G) -> Self {
        DfsPostOrder {
            g,
            stack: Vec::new(),
            seen: HashSet::new(),
            rest: g.vertices().collect::<Vec<_>>().into_iter(),
        }
    }
    fn push(&mut self, v: &'g G::Vertex) {
        self.seen.insert(v);
        let g = self.g;
        self.stack
            .push((v, g.out_neighbors_of(v).collect::<Vec<_>>().into_iter()));
    }
}

impl<'g, G: Graph> Iterator for DfsPostOrder<'g, G> {
    type Item = &'g G::Vertex;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((_, it)) = self.stack.last_mut() {
                match it.next() {
                    Some(n) => {
                        if !self.seen.contains(n) {
                            self.push(n);
                        }
                    }
                    None => return self.stack.pop().map(|(v, _)| v),
                }
                continue;
            }
            let root = self.rest.find(|v| !self.seen.contains(v))?;
            self.push(root);
        }
    }
}

/// Topological order (Kahn). If the graph has a cycle the iterator ends early;
/// check with [`TopologicalOrder::is_complete`] once exhausted.
pub struct TopologicalOrder<'g, G: DirectedGraph> {
    g: &'g G,
    queue: VecDeque<&'g G::Vertex>,
    indeg: HashMap<&'g G::Vertex, usize>,
    yielded: usize,
}

impl<'g, G: DirectedGraph> TopologicalOrder<'g, G> {
    pub fn new(g: &'g G) -> Self {
        let indeg: HashMap<_, _> = g.vertices().map(|v| (v, g.in_degree_of(v).unwrap())).collect();
        let queue = g.vertices().filter(|v| indeg[v] == 0).collect();
        TopologicalOrder {
            g,
            queue,
            indeg,
            yielded: 0,
        }
    }
    /// True if every vertex has been yielded (i.e. the graph is acyclic).
    pub fn is_complete(&self) -> bool {
        self.yielded == self.g.vertex_count()
    }
}

impl<'g, G: DirectedGraph> Iterator for TopologicalOrder<'g, G> {
    type Item = &'g G::Vertex;
    fn next(&mut self) -> Option<Self::Item> {
        let v = self.queue.pop_front()?;
        self.yielded += 1;
        let g = self.g;
        for n in g.out_neighbors_of(v) {
            let d = self.indeg.get_mut(n).expect("known vertex");
            *d -= 1;
            if *d == 0 {
                self.queue.push_back(n);
            }
        }
        Some(v)
    }
}

/// A topological ordering of `g`, or `None` if it contains a cycle.
pub fn topological_sort<G: DirectedGraph>(g: &G) -> Option<Vec<&G::Vertex>> {
    let mut it = TopologicalOrder::new(g);
    let order: Vec<_> = it.by_ref().collect();
    it.is_complete().then_some(order)
}
