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

use super::ordering::GraphOrdering;
use crate::graph::IncidenceGraph;
use indexmap::IndexMap;
use std::collections::BTreeMap;
use std::fmt;

/// Marks an unmatched vertex in the `core` arrays handed to [`IsomorphicGraphMapping::from_cores`].
pub const NULL_NODE: usize = usize::MAX;

/// This class represents a GraphMapping between two (subgraph)isomorphic graphs. In the subgraph
/// isomorphic case, the second one is assumed to be a subgraph of the first one.
///
/// # Rust notes
///
/// `IsomorphicGraphMapping`: a vertex mapping between two graphs of the same type, in both
/// directions, with the induced edge correspondence.
pub struct IsomorphicGraphMapping<'g, G: IncidenceGraph> {
    forward: IndexMap<G::Vertex, G::Vertex>,
    backward: IndexMap<G::Vertex, G::Vertex>,
    graph1: &'g G,
    graph2: &'g G,
}

impl<'g, G: IncidenceGraph> Clone for IsomorphicGraphMapping<'g, G> {
    fn clone(&self) -> Self {
        IsomorphicGraphMapping {
            forward: self.forward.clone(),
            backward: self.backward.clone(),
            graph1: self.graph1,
            graph2: self.graph2,
        }
    }
}

impl<'g, G: IncidenceGraph> IsomorphicGraphMapping<'g, G> {
    /// Builds the mapping from the vertex numbers matched in `core1` / `core2`
    /// ([`NULL_NODE`] marks unmatched vertices).
    pub fn from_cores(
        g1: &GraphOrdering<'g, G>,
        g2: &GraphOrdering<'g, G>,
        core1: &[usize],
        core2: &[usize],
    ) -> Self {
        let (graph1, graph2) = (g1.graph(), g2.graph());
        let mut forward = IndexMap::with_capacity(graph1.vertex_count());
        let mut backward = IndexMap::with_capacity(graph1.vertex_count());
        for v in graph1.vertices() {
            let u = core1[g1.vertex_number(v)];
            if u != NULL_NODE {
                forward.insert(v.clone(), g2.vertex(u).clone());
            }
        }
        for v in graph2.vertices() {
            let u = core2[g2.vertex_number(v)];
            if u != NULL_NODE {
                backward.insert(v.clone(), g1.vertex(u).clone());
            }
        }
        IsomorphicGraphMapping {
            forward,
            backward,
            graph1,
            graph2,
        }
    }

    /// Performs `from maps`.
    pub fn from_maps(
        forward: IndexMap<G::Vertex, G::Vertex>,
        backward: IndexMap<G::Vertex, G::Vertex>,
        graph1: &'g G,
        graph2: &'g G,
    ) -> Self {
        IsomorphicGraphMapping {
            forward,
            backward,
            graph1,
            graph2,
        }
    }

    /// Gets the mapped value where the key is `vertex`
    pub fn vertex_correspondence(&self, v: &G::Vertex, forward: bool) -> Option<&G::Vertex> {
        if forward {
            self.forward.get(v)
        } else {
            self.backward.get(v)
        }
    }

    /// Gets the mapped value where the key is `edge`
    pub fn edge_correspondence(&self, e: &G::Edge, forward: bool) -> Option<&G::Edge> {
        let (from, to) = if forward {
            (self.graph1, self.graph2)
        } else {
            (self.graph2, self.graph1)
        };
        let u = self.vertex_correspondence(from.edge_source(e)?, forward)?;
        let v = self.vertex_correspondence(from.edge_target(e)?, forward)?;
        to.get_edge(u, v)
    }

    /// Get an unmodifiable version of the forward mapping function.
    pub fn forward_mapping(&self) -> &IndexMap<G::Vertex, G::Vertex> {
        &self.forward
    }

    /// Get an unmodifiable version of the backward mapping function.
    pub fn backward_mapping(&self) -> &IndexMap<G::Vertex, G::Vertex> {
        &self.backward
    }

    /// Vertices of the first graph that are mapped.
    pub fn mapping_domain(&self) -> impl Iterator<Item = &G::Vertex> {
        self.forward.keys()
    }

    /// Vertices of the second graph that are mapped.
    pub fn mapping_range(&self) -> impl Iterator<Item = &G::Vertex> {
        self.backward.keys()
    }

    /// Checks if a vertex $v$ from the first graph has a corresponding vertex in the second graph
    pub fn has_vertex_correspondence(&self, v: &G::Vertex) -> bool {
        self.forward.contains_key(v)
    }

    /// Checks if a edge e from the first graph has a corresponding edge in the second graph
    pub fn has_edge_correspondence(&self, e: &G::Edge) -> bool {
        self.edge_correspondence(e, true).is_some()
    }

    /// Returns the graph1.
    pub fn graph1(&self) -> &'g G {
        self.graph1
    }

    /// Returns the graph2.
    pub fn graph2(&self) -> &'g G {
        self.graph2
    }

    /// Whether this is a bijection that maps edges onto edges in both directions.
    pub fn is_valid_isomorphism(&self) -> bool {
        let (g1, g2) = (self.graph1, self.graph2);
        if !g1
            .vertices()
            .all(|v| self.forward.get(v).is_some_and(|u| g2.contains_vertex(u)))
        {
            return false;
        }
        if !g2
            .vertices()
            .all(|v| self.backward.get(v).is_some_and(|u| g1.contains_vertex(u)))
        {
            return false;
        }
        g1.edges()
            .all(|e| self.edge_correspondence(e, true).is_some())
            && g2
                .edges()
                .all(|e| self.edge_correspondence(e, false).is_some())
    }

    /// Whether `other` agrees with this mapping on every vertex of the second graph.
    pub fn is_equal_mapping(&self, other: &Self) -> bool {
        self.graph2
            .vertices()
            .all(|v| self.backward.get(v) == other.backward.get(v))
    }

    /// The mapping `self` followed by `other`, from `self`'s first graph to `other`'s second.
    pub fn compose(&self, other: &Self) -> Self {
        let mut forward = IndexMap::with_capacity(self.forward.len());
        let mut backward = IndexMap::with_capacity(self.forward.len());
        for v in self.graph1.vertices() {
            let u = self
                .forward
                .get(v)
                .and_then(|m| other.vertex_correspondence(m, true));
            if let Some(u) = u {
                forward.insert(v.clone(), u.clone());
                backward.insert(u.clone(), v.clone());
            }
        }
        IsomorphicGraphMapping {
            forward,
            backward,
            graph1: self.graph1,
            graph2: other.graph2,
        }
    }

    /// Computes an identity automorphism (i.e. a self-mapping of a graph in which each vertex also
    /// maps to itself).
    pub fn identity(graph: &'g G) -> Self {
        let map: IndexMap<_, _> = graph.vertices().map(|v| (v.clone(), v.clone())).collect();
        IsomorphicGraphMapping {
            forward: map.clone(),
            backward: map,
            graph1: graph,
            graph2: graph,
        }
    }
}

impl<'g, G: IncidenceGraph> PartialEq for IsomorphicGraphMapping<'g, G> {
    fn eq(&self, other: &Self) -> bool {
        self.forward == other.forward
            && self.backward == other.backward
            && std::ptr::eq(self.graph1, other.graph1)
            && std::ptr::eq(self.graph2, other.graph2)
    }
}

impl<'g, G: IncidenceGraph> fmt::Debug for IsomorphicGraphMapping<'g, G>
where
    G::Vertex: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IsomorphicGraphMapping")
            .field("forward", &self.forward)
            .finish()
    }
}

/// `[a=x b=y c=~~]`, the vertices of the first graph sorted by their text.
impl<'g, G: IncidenceGraph> fmt::Display for IsomorphicGraphMapping<'g, G>
where
    G::Vertex: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sorted: BTreeMap<String, &G::Vertex> =
            self.graph1.vertices().map(|v| (v.to_string(), v)).collect();
        write!(f, "[")?;
        for (i, (name, v)) in sorted.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            match self.forward.get(*v) {
                Some(u) => write!(f, "{name}={u}")?,
                None => write!(f, "{name}=~~")?,
            }
        }
        write!(f, "]")
    }
}
