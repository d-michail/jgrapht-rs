use super::{CsrBooleanMatrix, collect_endpoints, identity_ids};
use crate::graph::Graph;
use crate::{GraphError, GraphType, Result};
use std::sync::OnceLock;

const NO_INCOMING: &str = "this graph does not support incoming edges";

/// Which incoming-edge index a [`SparseIntDirectedGraph`] builds.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncomingEdgesSupport {
    /// `degree_of`, `edges_of`, `in_degree_of` and `in_edges_of` return `Unsupported`.
    NoIncomingEdges,
    /// The incoming index is built on first use.
    LazyIncomingEdges,
    /// The incoming index is built by the constructor.
    FullIncomingEdges,
}

/// `NoIncomingNoReindexSparseDirectedSpecifics` and `IncomingNoReindexSparseDirectedSpecifics`:
/// an outgoing CSR matrix (vertex by edge id) and, when supported, an incoming one.
#[derive(Debug)]
struct DirectedCore {
    ids: Vec<usize>,
    num_vertices: usize,
    source: Vec<usize>,
    target: Vec<usize>,
    out_matrix: CsrBooleanMatrix,
    in_matrix: OnceLock<CsrBooleanMatrix>,
    incoming: bool,
}

impl DirectedCore {
    fn new(
        num_vertices: usize,
        num_edges: usize,
        edges: impl IntoIterator<Item = (usize, usize)>,
        support: IncomingEdgesSupport,
    ) -> Result<Self> {
        let (source, target) = collect_endpoints(num_vertices, num_edges, edges)?;
        let outgoing: Vec<_> = source.iter().copied().zip(0..).collect();
        let out_matrix = CsrBooleanMatrix::new(num_vertices, num_edges, &outgoing)?;
        let core = Self {
            ids: identity_ids(num_vertices, num_edges),
            num_vertices,
            source,
            target,
            out_matrix,
            in_matrix: OnceLock::new(),
            incoming: support != IncomingEdgesSupport::NoIncomingEdges,
        };
        if support == IncomingEdgesSupport::FullIncomingEdges {
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

    fn require_incoming(&self) -> Result<()> {
        if self.incoming {
            Ok(())
        } else {
            Err(GraphError::Unsupported(NO_INCOMING.into()))
        }
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

    fn in_edges(&self, v: usize) -> Result<&[usize]> {
        self.require_incoming()?;
        Ok(if self.has_vertex(v) {
            self.in_matrix().non_zeros_of(v)
        } else {
            &[]
        })
    }

    fn graph_type(&self, weighted: bool) -> GraphType {
        GraphType::pseudograph()
            .directed(true)
            .weighted(weighted)
            .as_unmodifiable()
    }
}

macro_rules! directed_graph_impl {
    ($graph:ty, $weighted:expr, $weight:expr) => {
        impl Graph for $graph {
            type Vertex = usize;
            type Edge = usize;

            fn graph_type(&self) -> GraphType {
                self.core.graph_type($weighted)
            }

            fn vertices(&self) -> impl Iterator<Item = &usize> {
                self.core.ids[..self.core.num_vertices].iter()
            }

            fn edges(&self) -> impl Iterator<Item = &usize> {
                self.core.ids[..self.core.source.len()].iter()
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

            /// Outgoing edges followed by incoming ones; a self-loop appears once.
            ///
            /// Fails for [`IncomingEdgesSupport::NoIncomingEdges`].
            fn edges_of(&self, v: &usize) -> Result<impl Iterator<Item = &usize>> {
                let core = &self.core;
                let incoming = core.in_edges(*v)?;
                Ok(core.out_edges(*v).iter().chain(
                    incoming
                        .iter()
                        .filter(move |&&e| core.source[e] != core.target[e]),
                ))
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

            fn in_edges_of(&self, v: &usize) -> Result<impl Iterator<Item = &usize>> {
                Ok(self.core.in_edges(*v)?.iter())
            }

            fn out_edges_of(&self, v: &usize) -> Result<impl Iterator<Item = &usize>> {
                Ok(self.core.out_edges(*v).iter())
            }

            fn in_degree_of(&self, v: &usize) -> Result<usize> {
                Ok(self.core.in_edges(*v)?.len())
            }

            fn out_degree_of(&self, v: &usize) -> Result<usize> {
                Ok(self.core.out_edges(*v).len())
            }

            /// Fails for [`IncomingEdgesSupport::NoIncomingEdges`].
            fn degree_of(&self, v: &usize) -> Result<usize> {
                Ok(self.core.in_edges(*v)?.len() + self.core.out_edges(*v).len())
            }
        }

        impl crate::graph::DirectedGraph for $graph {}
    };
}

/// `SparseIntDirectedGraph`: directed multigraph with loops on vertices `0..n`; edge `i` is the
/// `i`-th pair given to the constructor.
#[derive(Debug)]
pub struct SparseIntDirectedGraph {
    core: DirectedCore,
}

impl SparseIntDirectedGraph {
    /// Full incoming-edge support.
    pub fn new(num_vertices: usize, edges: &[(usize, usize)]) -> Result<Self> {
        Self::with_incoming_support(num_vertices, edges, IncomingEdgesSupport::FullIncomingEdges)
    }

    pub fn with_incoming_support(
        num_vertices: usize,
        edges: &[(usize, usize)],
        support: IncomingEdgesSupport,
    ) -> Result<Self> {
        Self::from_iter(num_vertices, edges.len(), edges.iter().copied(), support)
    }

    /// Builds the graph from an edge stream that must yield exactly `num_edges` pairs.
    pub fn from_iter(
        num_vertices: usize,
        num_edges: usize,
        edges: impl IntoIterator<Item = (usize, usize)>,
        support: IncomingEdgesSupport,
    ) -> Result<Self> {
        Ok(Self {
            core: DirectedCore::new(num_vertices, num_edges, edges, support)?,
        })
    }
}

directed_graph_impl!(SparseIntDirectedGraph, false, |g: &Self, e: usize| {
    (e < g.core.source.len()).then_some(1.0)
});

/// `SparseIntDirectedWeightedGraph`: as [`SparseIntDirectedGraph`] with one weight per edge.
#[derive(Debug)]
pub struct SparseIntDirectedWeightedGraph {
    core: DirectedCore,
    weights: Vec<f64>,
}

impl SparseIntDirectedWeightedGraph {
    pub fn new(num_vertices: usize, edges: &[(usize, usize, f64)]) -> Result<Self> {
        Self::with_incoming_support(num_vertices, edges, IncomingEdgesSupport::FullIncomingEdges)
    }

    pub fn with_incoming_support(
        num_vertices: usize,
        edges: &[(usize, usize, f64)],
        support: IncomingEdgesSupport,
    ) -> Result<Self> {
        Self::from_iter(num_vertices, edges.len(), edges.iter().copied(), support)
    }

    pub fn from_iter(
        num_vertices: usize,
        num_edges: usize,
        edges: impl IntoIterator<Item = (usize, usize, f64)>,
        support: IncomingEdgesSupport,
    ) -> Result<Self> {
        let mut weights = Vec::with_capacity(num_edges);
        let core = DirectedCore::new(
            num_vertices,
            num_edges,
            edges.into_iter().map(|(s, t, w)| {
                weights.push(w);
                (s, t)
            }),
            support,
        )?;
        Ok(Self { core, weights })
    }

    /// `setEdgeWeight`: the one mutation these graphs allow.
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
