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

//! Adversarial `Graph` implementations for the Rust API hardening work (`docs/RUST_API_ROADMAP.md`,
//! R0). They exist only in tests and are shared by the graph, algorithm and I/O suites.
//!
//! * [`Profile::OutgoingOnly`]: no incoming index; `edges_of`/`in_edges_of` fail with
//!   `Unsupported`, like `SparseIntDirectedGraph` with `NoIncomingEdges`.
//! * [`Profile::FullIncidence`]: a well-formed reference with every capability.
//! * [`Profile::Inconsistent`]: violates the `Graph` integrity contract on purpose: one edge has
//!   an endpoint that is not a vertex, one has no source, and one has no weight.
//!
//! [`weight_failure_cases`] lists the weights that weight-sensitive algorithms must reject.
#![allow(dead_code)]

use jgrapht::graph::Graph;
use jgrapht::graph::capability::{IncidenceGraph, IncomingGraph, OutgoingGraph};
use jgrapht::{GraphError, GraphType, Result};

/// Which `Graph` contract a [`FixtureGraph`] honors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    OutgoingOnly,
    FullIncidence,
    Inconsistent,
}

/// A small directed multigraph over `u32` vertices and `u32` edge ids.
///
/// Edge `i` of the list given to [`FixtureGraph::new`] gets id `i`.
#[derive(Debug, Clone)]
pub struct FixtureGraph {
    profile: Profile,
    vertices: Vec<u32>,
    /// `(source, target, weight)`; `None` models a broken endpoint or weight lookup.
    edges: Vec<(Option<u32>, Option<u32>, Option<f64>)>,
    edge_ids: Vec<u32>,
}

impl FixtureGraph {
    pub fn new(profile: Profile, vertices: &[u32], edges: &[(u32, u32, f64)]) -> Self {
        let edges: Vec<_> = edges
            .iter()
            .map(|&(s, t, w)| (Some(s), Some(t), Some(w)))
            .collect();
        let edge_ids = (0..edges.len() as u32).collect();
        Self {
            profile,
            vertices: vertices.to_vec(),
            edges,
            edge_ids,
        }
    }

    /// `0 -> 1 -> 2 -> 3` plus the shortcut `0 -> 2`; weights `1, 2, 3, 10`.
    pub fn diamond(profile: Profile) -> Self {
        Self::new(
            profile,
            &[0, 1, 2, 3],
            &[(0, 1, 1.0), (1, 2, 2.0), (2, 3, 3.0), (0, 2, 10.0)],
        )
    }

    /// A graph that breaks the integrity contract in three independent ways:
    /// edge 0 targets vertex 99 (not a vertex), edge 1 has no source, edge 2 has no weight.
    pub fn inconsistent() -> Self {
        let mut g = Self::new(
            Profile::Inconsistent,
            &[0, 1, 2],
            &[(0, 99, 1.0), (0, 1, 1.0), (1, 2, 1.0)],
        );
        g.edges[1].0 = None;
        g.edges[2].2 = None;
        g
    }

    /// A well-formed graph whose single edge carries `weight`, e.g. NaN or a negative value.
    pub fn with_weight(profile: Profile, weight: f64) -> Self {
        Self::new(profile, &[0, 1], &[(0, 1, weight)])
    }

    pub fn profile(&self) -> Profile {
        self.profile
    }

    fn edge(&self, e: u32) -> Option<&(Option<u32>, Option<u32>, Option<f64>)> {
        self.edges.get(e as usize)
    }

    fn has_incoming(&self) -> bool {
        self.profile != Profile::OutgoingOnly
    }

    fn require_incoming(&self) -> Result<()> {
        if self.has_incoming() {
            Ok(())
        } else {
            Err(GraphError::Unsupported(
                "this graph does not support incoming edges".into(),
            ))
        }
    }

    fn outgoing(&self, v: u32) -> impl Iterator<Item = &u32> {
        self.edge_ids
            .iter()
            .filter(move |&&e| self.edges[e as usize].0 == Some(v))
    }
}

impl Graph for FixtureGraph {
    type Vertex = u32;
    type Edge = u32;

    fn graph_type(&self) -> GraphType {
        GraphType::pseudograph().directed(true).weighted(true)
    }

    fn vertices(&self) -> impl Iterator<Item = &u32> {
        self.vertices.iter()
    }

    fn edges(&self) -> impl Iterator<Item = &u32> {
        self.edge_ids.iter()
    }

    fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn contains_vertex(&self, v: &u32) -> bool {
        self.vertices.contains(v)
    }

    fn contains_edge(&self, e: &u32) -> bool {
        self.edge(*e).is_some()
    }

    fn edge_source(&self, e: &u32) -> Option<&u32> {
        self.edge(*e)?.0.as_ref()
    }

    fn edge_target(&self, e: &u32) -> Option<&u32> {
        self.edge(*e)?.1.as_ref()
    }

    fn edge_weight(&self, e: &u32) -> Option<f64> {
        self.edge(*e)?.2
    }

    fn edges_of(&self, v: &u32) -> Result<impl Iterator<Item = &u32>> {
        self.require_incoming()?;
        let v = *v;
        Ok(self.edge_ids.iter().filter(move |&&e| {
            let (s, t, _) = self.edges[e as usize];
            s == Some(v) || t == Some(v)
        }))
    }

    fn edges_between<'a>(&'a self, u: &'a u32, v: &'a u32) -> impl Iterator<Item = &'a u32> {
        self.edge_ids.iter().filter(move |&&e| {
            let (s, t, _) = self.edges[e as usize];
            s == Some(*u) && t == Some(*v)
        })
    }

    fn in_edges_of(&self, v: &u32) -> Result<impl Iterator<Item = &u32>> {
        self.require_incoming()?;
        let v = *v;
        Ok(self
            .edge_ids
            .iter()
            .filter(move |&&e| self.edges[e as usize].1 == Some(v)))
    }

    fn out_edges_of(&self, v: &u32) -> Result<impl Iterator<Item = &u32>> {
        Ok(self.outgoing(*v))
    }
}

impl jgrapht::graph::DirectedGraph for FixtureGraph {}

/// Weights that weight-sensitive algorithms must reject with a typed error: `(label, weight)`.
pub fn weight_failure_cases() -> [(&'static str, f64); 4] {
    [
        ("nan", f64::NAN),
        ("negative", -1.0),
        ("negative infinity", f64::NEG_INFINITY),
        ("tiny negative", -f64::MIN_POSITIVE),
    ]
}

/// Newtypes giving [`FixtureGraph`] the static capabilities its profile honors, for the
/// capability-trait tests. `OutgoingFixture` is deliberately not `IncomingGraph`.
macro_rules! delegate_graph {
    ($ty:ident) => {
        impl Graph for $ty {
            type Vertex = u32;
            type Edge = u32;
            fn graph_type(&self) -> GraphType {
                self.0.graph_type()
            }
            fn vertices(&self) -> impl Iterator<Item = &u32> {
                self.0.vertices()
            }
            fn edges(&self) -> impl Iterator<Item = &u32> {
                self.0.edges()
            }
            fn vertex_count(&self) -> usize {
                self.0.vertex_count()
            }
            fn edge_count(&self) -> usize {
                self.0.edge_count()
            }
            fn contains_vertex(&self, v: &u32) -> bool {
                self.0.contains_vertex(v)
            }
            fn contains_edge(&self, e: &u32) -> bool {
                self.0.contains_edge(e)
            }
            fn edge_source(&self, e: &u32) -> Option<&u32> {
                self.0.edge_source(e)
            }
            fn edge_target(&self, e: &u32) -> Option<&u32> {
                self.0.edge_target(e)
            }
            fn edge_weight(&self, e: &u32) -> Option<f64> {
                self.0.edge_weight(e)
            }
            fn edges_of(&self, v: &u32) -> Result<impl Iterator<Item = &u32>> {
                self.0.edges_of(v)
            }
            fn edges_between<'a>(
                &'a self,
                u: &'a u32,
                v: &'a u32,
            ) -> impl Iterator<Item = &'a u32> {
                self.0.edges_between(u, v)
            }
            fn in_edges_of(&self, v: &u32) -> Result<impl Iterator<Item = &u32>> {
                self.0.in_edges_of(v)
            }
            fn out_edges_of(&self, v: &u32) -> Result<impl Iterator<Item = &u32>> {
                self.0.out_edges_of(v)
            }
        }
    };
}

/// Implements only [`OutgoingGraph`].
#[derive(Debug, Clone)]
pub struct OutgoingFixture(pub FixtureGraph);
/// Implements the full [`IncidenceGraph`] capability set.
#[derive(Debug, Clone)]
pub struct FullFixture(pub FixtureGraph);

delegate_graph!(OutgoingFixture);
delegate_graph!(FullFixture);

impl OutgoingFixture {
    pub fn new(vertices: &[u32], edges: &[(u32, u32, f64)]) -> Self {
        Self(FixtureGraph::new(Profile::OutgoingOnly, vertices, edges))
    }
}

impl FullFixture {
    pub fn new(vertices: &[u32], edges: &[(u32, u32, f64)]) -> Self {
        Self(FixtureGraph::new(Profile::FullIncidence, vertices, edges))
    }
}

impl OutgoingGraph for OutgoingFixture {
    fn out_edges_of(&self, v: &u32) -> impl Iterator<Item = &u32> {
        self.0.outgoing(*v)
    }
}

impl OutgoingGraph for FullFixture {
    fn out_edges_of(&self, v: &u32) -> impl Iterator<Item = &u32> {
        self.0.outgoing(*v)
    }
}

impl IncomingGraph for FullFixture {
    fn in_edges_of(&self, v: &u32) -> impl Iterator<Item = &u32> {
        let v = *v;
        self.0
            .edge_ids
            .iter()
            .filter(move |&&e| self.0.edges[e as usize].1 == Some(v))
    }
}

impl IncidenceGraph for FullFixture {
    fn edges_of(&self, v: &u32) -> impl Iterator<Item = &u32> {
        let v = *v;
        self.0.edge_ids.iter().filter(move |&&e| {
            let (s, t, _) = self.0.edges[e as usize];
            s == Some(v) || t == Some(v)
        })
    }
}
