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

use super::{CsrBooleanMatrix, collect_endpoints};
use crate::graph::Graph;
use crate::graph::capability::{IncidenceGraph, IncomingGraph, OutgoingGraph};
use crate::{GraphError, GraphType, Result};
use std::marker::PhantomData;
use std::sync::OnceLock;

mod sealed {
    pub trait Sealed {}
}

/// Which incoming-edge index a [`SparseIntDirectedGraph`] keeps; one of [`NoIncoming`],
/// [`LazyIncoming`] and [`FullIncoming`]. Sealed.
pub trait IncomingMode: sealed::Sealed + std::fmt::Debug + Send + Sync + 'static {
    /// Whether the graph can answer incoming-edge queries at all.
    const SUPPORTED: bool;
    /// Whether the constructor builds the incoming index (otherwise it is built on first use).
    const EAGER: bool;
}

/// An [`IncomingMode`] with an incoming index: [`LazyIncoming`] or [`FullIncoming`]. Graphs in
/// these modes implement [`IncomingGraph`] and [`IncidenceGraph`].
pub trait HasIncoming: IncomingMode {}

/// No incoming-edge index: the graph is [`OutgoingGraph`] only, and passing it to an algorithm that
/// needs incoming edges is a compile error.
#[derive(Debug, Clone, Copy)]
pub enum NoIncoming {}
/// The incoming index is built on first use.
#[derive(Debug, Clone, Copy)]
pub enum LazyIncoming {}
/// The incoming index is built by the constructor.
#[derive(Debug, Clone, Copy)]
pub enum FullIncoming {}

macro_rules! incoming_mode {
    ($mode:ty, $supported:expr, $eager:expr) => {
        impl sealed::Sealed for $mode {}
        impl IncomingMode for $mode {
            const SUPPORTED: bool = $supported;
            const EAGER: bool = $eager;
        }
    };
}
incoming_mode!(NoIncoming, false, false);
incoming_mode!(LazyIncoming, true, false);
incoming_mode!(FullIncoming, true, true);
impl HasIncoming for LazyIncoming {}
impl HasIncoming for FullIncoming {}

/// `NoIncomingNoReindexSparseDirectedSpecifics` and `IncomingNoReindexSparseDirectedSpecifics`:
/// an outgoing CSR matrix (vertex by edge id) and, when supported, an incoming one.
#[derive(Debug)]
struct DirectedCore {
    num_vertices: usize,
    source: Vec<usize>,
    target: Vec<usize>,
    out_matrix: CsrBooleanMatrix,
    in_matrix: OnceLock<CsrBooleanMatrix>,
}

impl DirectedCore {
    fn new<I: IncomingMode>(
        num_vertices: usize,
        num_edges: usize,
        edges: impl IntoIterator<Item = (usize, usize)>,
    ) -> Result<Self> {
        let (source, target) = collect_endpoints(num_vertices, num_edges, edges)?;
        let outgoing: Vec<_> = source.iter().copied().zip(0..).collect();
        let out_matrix = CsrBooleanMatrix::new(num_vertices, num_edges, &outgoing)?;
        let core = Self {
            num_vertices,
            source,
            target,
            out_matrix,
            in_matrix: OnceLock::new(),
        };
        if I::EAGER {
            core.in_matrix();
        }
        Ok(core)
    }

    /// `indexIncomingEdges`, run at most once.
    fn in_matrix(&self) -> &CsrBooleanMatrix {
        self.in_matrix.get_or_init(|| {
            let incoming: Vec<_> = self.target.iter().copied().zip(0..).collect();
            CsrBooleanMatrix::new(self.num_vertices, self.source.len(), &incoming)
                .expect("endpoints were validated")
        })
    }

    fn has_vertex(&self, v: usize) -> bool {
        v < self.num_vertices
    }

    fn row<'a>(&'a self, matrix: &'a CsrBooleanMatrix, v: usize) -> &'a [usize] {
        if self.has_vertex(v) {
            matrix.non_zeros_of(v)
        } else {
            &[]
        }
    }

    fn out_edges(&self, v: usize) -> &[usize] {
        self.row(&self.out_matrix, v)
    }

    /// Incoming edges of `v`, building the index if needed. Only called for modes that have one.
    fn in_edges(&self, v: usize) -> &[usize] {
        if self.has_vertex(v) {
            self.in_matrix().non_zeros_of(v)
        } else {
            &[]
        }
    }

    fn graph_type(&self, weighted: bool) -> GraphType {
        GraphType::pseudograph()
            .directed(true)
            .weighted(weighted)
            .as_unmodifiable()
    }
}

macro_rules! directed_graph_impl {
    ($graph:ident, $weighted:expr, $weight:expr) => {
        impl<I: IncomingMode> Graph for $graph<I> {
            type Vertex = usize;
            type Edge = usize;
            type VertexRef<'a> = usize;
            type EdgeRef<'a> = usize;

            fn graph_type(&self) -> GraphType {
                self.core.graph_type($weighted)
            }

            fn vertices(&self) -> impl Iterator<Item = usize> {
                0..self.core.num_vertices
            }

            fn edges(&self) -> impl Iterator<Item = usize> {
                0..self.core.source.len()
            }

            fn vertex_count(&self) -> usize {
                self.core.num_vertices
            }

            fn edge_count(&self) -> usize {
                self.core.source.len()
            }

            fn contains_vertex(&self, v: &usize) -> bool {
                self.core.has_vertex(*v)
            }

            fn contains_edge(&self, e: &usize) -> bool {
                *e < self.core.source.len()
            }

            fn edge_source(&self, e: &usize) -> Option<&usize> {
                self.core.source.get(*e)
            }

            fn edge_target(&self, e: &usize) -> Option<&usize> {
                self.core.target.get(*e)
            }

            fn edge_weight(&self, e: &usize) -> Option<f64> {
                #[allow(clippy::redundant_closure_call)]
                ($weight)(self, *e)
            }

            fn edges_between<'a>(
                &'a self,
                u: &'a usize,
                v: &'a usize,
            ) -> impl Iterator<Item = &'a usize> {
                let core = &self.core;
                let row = if core.has_vertex(*v) {
                    core.out_edges(*u)
                } else {
                    &[]
                };
                row.iter().filter(move |&&e| core.target[e] == *v)
            }

            fn is_edge_directed(&self, _e: &usize) -> bool {
                true
            }
        }

        impl<I: IncomingMode> crate::graph::DirectedGraph for $graph<I> {}

        impl<I: IncomingMode> OutgoingGraph for $graph<I> {
            fn out_edges_of(&self, v: &usize) -> impl Iterator<Item = &usize> {
                self.core.out_edges(*v).iter()
            }

            fn out_degree_of(&self, v: &usize) -> usize {
                self.core.out_edges(*v).len()
            }
        }

        impl<I: HasIncoming> IncomingGraph for $graph<I> {
            fn in_edges_of(&self, v: &usize) -> impl Iterator<Item = &usize> {
                self.core.in_edges(*v).iter()
            }

            fn in_degree_of(&self, v: &usize) -> usize {
                self.core.in_edges(*v).len()
            }
        }

        impl<I: HasIncoming> IncidenceGraph for $graph<I> {
            fn edges_of(&self, v: &usize) -> impl Iterator<Item = &usize> {
                let core = &self.core;
                core.out_edges(*v).iter().chain(
                    core.in_edges(*v)
                        .iter()
                        .filter(move |&&e| core.source[e] != core.target[e]),
                )
            }

            fn degree_of(&self, v: &usize) -> usize {
                self.core.in_edges(*v).len() + self.core.out_edges(*v).len()
            }
        }
    };
}

/// `SparseIntDirectedGraph`: directed multigraph with loops on vertices `0..n`; edge `i` is the
/// `i`-th pair given to the constructor.
///
/// The type parameter selects the incoming-edge index; see [`IncomingMode`]. The default,
/// [`FullIncoming`], implements every incidence capability; `SparseIntDirectedGraph::<NoIncoming>`
/// is cheaper but only implements [`OutgoingGraph`]:
///
/// ```compile_fail,E0277
/// use jgrapht::graph::capability::IncomingGraph;
/// use jgrapht::graph::sparse::{NoIncoming, SparseIntDirectedGraph};
///
/// fn needs_incoming<G: IncomingGraph>(_: &G) {}
/// let g = SparseIntDirectedGraph::<NoIncoming>::from_edges(2, &[(0, 1)]).unwrap();
/// needs_incoming(&g);
/// ```
#[derive(Debug)]
pub struct SparseIntDirectedGraph<I: IncomingMode = FullIncoming> {
    core: DirectedCore,
    mode: PhantomData<I>,
}

impl SparseIntDirectedGraph {
    /// Full incoming-edge support. Use [`SparseIntDirectedGraph::from_edges`] with an explicit
    /// [`IncomingMode`] for the other modes.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(num_vertices: usize, edges: &[(usize, usize)]) -> Result<Self> {
        Self::from_edges(num_vertices, edges)
    }
}

impl<I: IncomingMode> SparseIntDirectedGraph<I> {
    /// Builds the graph from an edge slice.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn from_edges(num_vertices: usize, edges: &[(usize, usize)]) -> Result<Self> {
        Self::from_iter(num_vertices, edges.len(), edges.iter().copied())
    }

    /// Builds the graph from an edge stream that must yield exactly `num_edges` pairs.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn from_iter(
        num_vertices: usize,
        num_edges: usize,
        edges: impl IntoIterator<Item = (usize, usize)>,
    ) -> Result<Self> {
        Ok(Self {
            core: DirectedCore::new::<I>(num_vertices, num_edges, edges)?,
            mode: PhantomData,
        })
    }
}

directed_graph_impl!(SparseIntDirectedGraph, false, |g: &Self, e: usize| {
    (e < g.core.source.len()).then_some(1.0)
});

/// `SparseIntDirectedWeightedGraph`: as [`SparseIntDirectedGraph`] with one weight per edge.
#[derive(Debug)]
pub struct SparseIntDirectedWeightedGraph<I: IncomingMode = FullIncoming> {
    core: DirectedCore,
    weights: Vec<f64>,
    mode: PhantomData<I>,
}

impl SparseIntDirectedWeightedGraph {
    /// Full incoming-edge support.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(num_vertices: usize, edges: &[(usize, usize, f64)]) -> Result<Self> {
        Self::from_edges(num_vertices, edges)
    }
}

impl<I: IncomingMode> SparseIntDirectedWeightedGraph<I> {
    /// Builds the graph from an edge slice.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn from_edges(num_vertices: usize, edges: &[(usize, usize, f64)]) -> Result<Self> {
        Self::from_iter(num_vertices, edges.len(), edges.iter().copied())
    }

    /// Performs `from iter`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn from_iter(
        num_vertices: usize,
        num_edges: usize,
        edges: impl IntoIterator<Item = (usize, usize, f64)>,
    ) -> Result<Self> {
        let mut weights = Vec::with_capacity(num_edges);
        let core = DirectedCore::new::<I>(
            num_vertices,
            num_edges,
            edges.into_iter().map(|(s, t, w)| {
                weights.push(w);
                (s, t)
            }),
        )?;
        Ok(Self {
            core,
            weights,
            mode: PhantomData,
        })
    }

    /// `setEdgeWeight`: the one mutation these graphs allow.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if an edge argument is not in the graph; errors of the underlying graph operations are propagated.
    pub fn set_edge_weight(&mut self, e: usize, weight: f64) -> Result<()> {
        *self.weights.get_mut(e).ok_or(GraphError::EdgeNotFound)? = weight;
        Ok(())
    }
}

directed_graph_impl!(
    SparseIntDirectedWeightedGraph,
    true,
    |g: &Self, e: usize| { g.weights.get(e).copied() }
);

#[cfg(feature = "serde")]
mod serde_impl;
