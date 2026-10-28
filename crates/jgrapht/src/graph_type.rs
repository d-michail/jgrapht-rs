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

/// Capabilities of a graph, mirroring JGraphT's `GraphType`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphType {
    /// Edges have a direction.
    pub directed: bool,
    /// An edge may join a vertex to itself.
    pub self_loops: bool,
    /// Several edges may join the same pair of vertices.
    pub multiple_edges: bool,
    /// Edges carry weights; otherwise every weight is `1.0`.
    pub weighted: bool,
    /// Adding an edge may close a cycle; `false` for a directed acyclic graph.
    pub allows_cycles: bool,
    /// Vertices and edges can be added and removed.
    pub modifiable: bool,
    /// Mixed graphs contain both directed and undirected edges (`directed` is then `false`).
    pub mixed: bool,
}

impl GraphType {
    /// Simple graph: no loops, no parallel edges, unweighted. Direction is set by the graph.
    pub const fn simple() -> Self {
        Self {
            directed: false,
            self_loops: false,
            multiple_edges: false,
            weighted: false,
            allows_cycles: true,
            modifiable: true,
            mixed: false,
        }
    }
    /// Multigraph: parallel edges, no loops.
    pub const fn multigraph() -> Self {
        Self {
            multiple_edges: true,
            ..Self::simple()
        }
    }
    /// Pseudograph: parallel edges and loops.
    pub const fn pseudograph() -> Self {
        Self {
            multiple_edges: true,
            self_loops: true,
            ..Self::simple()
        }
    }
    /// This type with the direction set to `d`.
    pub const fn directed(mut self, d: bool) -> Self {
        self.directed = d;
        self
    }
    /// This type with the weighted flag set to `w`.
    pub const fn weighted(mut self, w: bool) -> Self {
        self.weighted = w;
        self
    }
    /// Whether all edges are undirected.
    pub const fn is_undirected(&self) -> bool {
        !self.directed && !self.mixed
    }
    /// Whether the graph has both directed and undirected edges.
    pub const fn is_mixed(&self) -> bool {
        self.mixed
    }
    /// Whether all edges are directed.
    pub const fn is_directed(&self) -> bool {
        self.directed
    }
    /// This type as a mixed graph (directed and undirected edges together).
    pub const fn as_mixed(mut self) -> Self {
        self.directed = false;
        self.mixed = true;
        self
    }
    /// Whether the graph has neither self-loops nor parallel edges.
    pub const fn is_simple(&self) -> bool {
        !self.multiple_edges && !self.self_loops
    }
    /// Whether the graph allows both self-loops and parallel edges.
    pub const fn is_pseudograph(&self) -> bool {
        self.multiple_edges && self.self_loops
    }
    /// Whether the graph allows parallel edges but no self-loops.
    pub const fn is_multigraph(&self) -> bool {
        self.multiple_edges && !self.self_loops
    }
    /// Whether the graph cannot contain cycles.
    pub const fn is_acyclic(&self) -> bool {
        !self.allows_cycles
    }
    /// This type with modification switched off.
    pub const fn as_unmodifiable(mut self) -> Self {
        self.modifiable = false;
        self
    }
    /// This type with the weights dropped.
    pub const fn as_unweighted(mut self) -> Self {
        self.weighted = false;
        self
    }
    /// This type with weights switched on.
    pub const fn as_weighted(mut self) -> Self {
        self.weighted = true;
        self
    }
}
