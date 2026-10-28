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

use crate::graph::Graph;
use crate::graph::IncidenceGraph;
use std::borrow::Cow;
use std::cell::{Cell, OnceCell};
use std::collections::HashMap;

/// This class represents the order on the graph vertices. There are also some helper-functions for
/// receiving outgoing/incoming edges, etc.
///
/// # Rust notes
///
/// `GraphOrdering`: numbers the vertices of a graph `0..n` (optionally by ascending degree) and
/// answers adjacency queries on those numbers, caching lazily as the Java class does.
pub struct GraphOrdering<'g, G: Graph> {
    graph: &'g G,
    order: HashMap<&'g G::Vertex, usize>,
    vertices: Vec<&'g G::Vertex>,
    outgoing: Vec<OnceCell<Vec<usize>>>,
    incoming: Vec<OnceCell<Vec<usize>>>,
    edge_cache: Vec<Cell<Option<&'g G::Edge>>>,
    /// 0 = unknown, 1 = edge, -1 = no edge
    adj: Vec<Cell<i8>>,
    cache_edges: bool,
}

impl<'g, G: IncidenceGraph> GraphOrdering<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self::with_options(graph, false, true)
    }

    pub fn with_options(graph: &'g G, order_by_degree: bool, cache_edges: bool) -> Self {
        let mut vertices: Vec<&'g G::Vertex> = graph.vertices().collect();
        if order_by_degree {
            // stable, like List.sort
            vertices.sort_by_key(|v| graph.degree_of(v));
        }
        let n = vertices.len();
        let order = vertices.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let (outgoing, incoming, edge_cache, adj) = if cache_edges {
            (
                (0..n).map(|_| OnceCell::new()).collect(),
                (0..n).map(|_| OnceCell::new()).collect(),
                (0..n * n).map(|_| Cell::new(None)).collect(),
                (0..n * n).map(|_| Cell::new(0)).collect(),
            )
        } else {
            (vec![], vec![], vec![], vec![])
        };
        GraphOrdering {
            graph,
            order,
            vertices,
            outgoing,
            incoming,
            edge_cache,
            adj,
            cache_edges,
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    fn neighbors(&self, v: usize, out: bool) -> Vec<usize> {
        let vertex = self.vertices[v];
        let g = self.graph;
        let mut res = Vec::new();
        let mut push = |e: &G::Edge| {
            let (s, t) = (g.edge_source(e).unwrap(), g.edge_target(e).unwrap());
            res.push(self.order[if s == vertex { t } else { s }]);
        };
        if out {
            g.out_edges_of(vertex).for_each(&mut push);
        } else {
            g.in_edges_of(vertex).for_each(&mut push);
        }
        res
    }

    /// Numbers of the vertices at the other end of the outgoing edges of vertex `v`.
    pub fn out_edges(&self, v: usize) -> Cow<'_, [usize]> {
        if self.cache_edges {
            Cow::Borrowed(self.outgoing[v].get_or_init(|| self.neighbors(v, true)))
        } else {
            Cow::Owned(self.neighbors(v, true))
        }
    }

    /// Numbers of the vertices at the other end of the incoming edges of vertex `v`.
    pub fn in_edges(&self, v: usize) -> Cow<'_, [usize]> {
        if self.cache_edges {
            Cow::Borrowed(self.incoming[v].get_or_init(|| self.neighbors(v, false)))
        } else {
            Cow::Owned(self.neighbors(v, false))
        }
    }

    pub fn has_edge(&self, v1: usize, v2: usize) -> bool {
        if self.cache_edges {
            let i = v1 * self.vertices.len() + v2;
            match self.adj[i].get() {
                0 => {
                    let e = self.graph.get_edge(self.vertices[v1], self.vertices[v2]);
                    self.adj[i].set(if e.is_some() { 1 } else { -1 });
                    self.edge_cache[i].set(e);
                    e.is_some()
                }
                c => c > 0,
            }
        } else {
            self.graph
                .get_edge(self.vertices[v1], self.vertices[v2])
                .is_some()
        }
    }

    /// be careful: there's no check against an invalid vertexNumber
    pub fn vertex(&self, v: usize) -> &'g G::Vertex {
        self.vertices[v]
    }

    pub fn edge(&self, v1: usize, v2: usize) -> Option<&'g G::Edge> {
        if self.cache_edges {
            self.has_edge(v1, v2);
            self.edge_cache[v1 * self.vertices.len() + v2].get()
        } else {
            self.graph.get_edge(self.vertices[v1], self.vertices[v2])
        }
    }

    pub fn vertex_number(&self, v: &G::Vertex) -> usize {
        self.order[v]
    }

    /// Numbers of the source and target of `e`.
    pub fn edge_numbers(&self, e: &G::Edge) -> [usize; 2] {
        let (s, t) = (self.graph.edge_source(e), self.graph.edge_target(e));
        [self.order[s.unwrap()], self.order[t.unwrap()]]
    }

    pub fn graph(&self) -> &'g G {
        self.graph
    }
}
