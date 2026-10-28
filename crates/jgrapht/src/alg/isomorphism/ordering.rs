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
use crate::graph::{Graph, GraphExt};
use std::borrow::Cow;
use std::cell::OnceCell;
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
    order: HashMap<G::Vertex, usize>,
    vertices: Vec<G::Vertex>,
    outgoing: Vec<OnceCell<Vec<usize>>>,
    incoming: Vec<OnceCell<Vec<usize>>>,
    edge_cache: Vec<OnceCell<Option<G::Edge>>>,
    cache_edges: bool,
}

impl<'g, G: IncidenceGraph> GraphOrdering<'g, G> {
    /// An ordering of the vertices of `graph` that is not sorted by degree and caches the edges.
    pub fn new(graph: &'g G) -> Self {
        Self::with_options(graph, false, true)
    }

    /// Creates an instance with the given options.
    pub fn with_options(graph: &'g G, order_by_degree: bool, cache_edges: bool) -> Self {
        let mut vertices: Vec<G::Vertex> = graph.vertices_cloned().collect();
        if order_by_degree {
            // stable, like List.sort
            vertices.sort_by_key(|v| graph.degree_of(v));
        }
        let n = vertices.len();
        let order = vertices
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();
        let (outgoing, incoming, edge_cache) = if cache_edges {
            (
                (0..n).map(|_| OnceCell::new()).collect(),
                (0..n).map(|_| OnceCell::new()).collect(),
                (0..n * n).map(|_| OnceCell::new()).collect(),
            )
        } else {
            (vec![], vec![], vec![])
        };
        GraphOrdering {
            graph,
            order,
            vertices,
            outgoing,
            incoming,
            edge_cache,
            cache_edges,
        }
    }

    /// Number of vertices of the graph.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    fn neighbors(&self, v: usize, out: bool) -> Vec<usize> {
        let vertex = &self.vertices[v];
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

    /// Whether the vertices with numbers `v1` and `v2` are joined by an edge.
    pub fn has_edge(&self, v1: usize, v2: usize) -> bool {
        self.edge(v1, v2).is_some()
    }

    /// be careful: there's no check against an invalid vertexNumber
    pub fn vertex(&self, v: usize) -> &G::Vertex {
        &self.vertices[v]
    }

    /// The edge between the vertices with numbers `v1` and `v2`, if there is one.
    pub fn edge(&self, v1: usize, v2: usize) -> Option<G::Edge> {
        let lookup = || {
            self.graph
                .edge_between(&self.vertices[v1], &self.vertices[v2])
                .cloned()
        };
        if self.cache_edges {
            self.edge_cache[v1 * self.vertices.len() + v2]
                .get_or_init(lookup)
                .clone()
        } else {
            lookup()
        }
    }

    /// The number of `v` in this ordering.
    pub fn vertex_number(&self, v: &G::Vertex) -> usize {
        self.order[v]
    }

    /// Numbers of the source and target of `e`.
    ///
    /// # Panics
    ///
    /// Panics only if the graph breaks the integrity contract of [`Graph`] (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
    pub fn edge_numbers(&self, e: &G::Edge) -> [usize; 2] {
        let (s, t) = (self.graph.edge_source(e), self.graph.edge_target(e));
        [self.order[s.unwrap()], self.order[t.unwrap()]]
    }

    /// The graph being ordered.
    pub fn graph(&self) -> &'g G {
        self.graph
    }
}
