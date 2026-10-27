use super::{CsrBooleanMatrix, collect_endpoints, identity_ids};
use crate::graph::Graph;
use crate::{GraphError, GraphType, Result};

/// `IncidenceMatrixSparseUndirectedSpecifics`: one incidence CSR matrix (vertex by edge id) in
/// which every edge is listed under both endpoints, so a self-loop appears twice in its row.
#[derive(Debug)]
struct UndirectedCore {
    ids: Vec<usize>,
    source: Vec<usize>,
    target: Vec<usize>,
    incidence: CsrBooleanMatrix,
}

impl UndirectedCore {
    fn new(
        num_vertices: usize,
        num_edges: usize,
        edges: impl IntoIterator<Item = (usize, usize)>,
    ) -> Result<Self> {
        let (source, target) = collect_endpoints(num_vertices, num_edges, edges)?;
        let mut non_zeros = Vec::with_capacity(2 * num_edges);
        for (e, (&s, &t)) in source.iter().zip(&target).enumerate() {
            non_zeros.push((s, e));
            non_zeros.push((t, e));
        }
        Ok(Self {
            ids: identity_ids(num_vertices, num_edges),
            incidence: CsrBooleanMatrix::new(num_vertices, num_edges, &non_zeros)?,
            source,
            target,
        })
    }

    fn has_vertex(&self, v: usize) -> bool {
        v < self.incidence.rows()
    }

    /// Incidence row of `v`, with a self-loop listed twice.
    fn row(&self, v: usize) -> &[usize] {
        if self.has_vertex(v) {
            self.incidence.non_zeros_of(v)
        } else {
            &[]
        }
    }
}

macro_rules! undirected_graph_impl {
    ($graph:ty, $weighted:expr, $weight:expr) => {
        impl Graph for $graph {
            type Vertex = usize;
            type Edge = usize;

            fn graph_type(&self) -> GraphType {
                GraphType::pseudograph()
                    .weighted($weighted)
                    .as_unmodifiable()
            }

            fn vertices(&self) -> impl Iterator<Item = &usize> {
                self.core.ids[..self.core.incidence.rows()].iter()
            }

            fn edges(&self) -> impl Iterator<Item = &usize> {
                self.core.ids[..self.core.source.len()].iter()
            }

            fn vertex_count(&self) -> usize {
                self.core.incidence.rows()
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

            /// Incident edges in increasing id order; a self-loop appears once (its two
            /// incidence entries are adjacent).
            fn edges_of(&self, v: &usize) -> Result<impl Iterator<Item = &usize>> {
                let row = self.core.row(*v);
                Ok(row
                    .iter()
                    .enumerate()
                    .filter(move |&(i, &e)| i == 0 || row[i - 1] != e)
                    .map(|(_, e)| e))
            }

            fn edges_between<'a>(
                &'a self,
                u: &'a usize,
                v: &'a usize,
            ) -> impl Iterator<Item = &'a usize> {
                let core = &self.core;
                let row = if core.has_vertex(*v) {
                    core.row(*u)
                } else {
                    &[]
                };
                row.iter().enumerate().filter_map(move |(i, e)| {
                    let (s, t) = (core.source[*e], core.target[*e]);
                    let matches = (s == *u && t == *v) || (s == *v && t == *u);
                    (matches && (i == 0 || row[i - 1] != *e)).then_some(e)
                })
            }

            fn degree_of(&self, v: &usize) -> Result<usize> {
                Ok(self.core.row(*v).len())
            }
        }

        impl crate::graph::UndirectedGraph for $graph {}
    };
}

/// `SparseIntUndirectedGraph`: undirected multigraph with loops on vertices `0..n`.
#[derive(Debug)]
pub struct SparseIntUndirectedGraph {
    core: UndirectedCore,
}

impl SparseIntUndirectedGraph {
    pub fn new(num_vertices: usize, edges: &[(usize, usize)]) -> Result<Self> {
        Self::from_iter(num_vertices, edges.len(), edges.iter().copied())
    }

    /// Builds the graph from an edge stream that must yield exactly `num_edges` pairs.
    pub fn from_iter(
        num_vertices: usize,
        num_edges: usize,
        edges: impl IntoIterator<Item = (usize, usize)>,
    ) -> Result<Self> {
        Ok(Self {
            core: UndirectedCore::new(num_vertices, num_edges, edges)?,
        })
    }
}

undirected_graph_impl!(SparseIntUndirectedGraph, false, |g: &Self, e: usize| {
    (e < g.core.source.len()).then_some(1.0)
});

/// `SparseIntUndirectedWeightedGraph`: as [`SparseIntUndirectedGraph`] with one weight per edge.
#[derive(Debug)]
pub struct SparseIntUndirectedWeightedGraph {
    core: UndirectedCore,
    weights: Vec<f64>,
}

impl SparseIntUndirectedWeightedGraph {
    pub fn new(num_vertices: usize, edges: &[(usize, usize, f64)]) -> Result<Self> {
        Self::from_iter(num_vertices, edges.len(), edges.iter().copied())
    }

    pub fn from_iter(
        num_vertices: usize,
        num_edges: usize,
        edges: impl IntoIterator<Item = (usize, usize, f64)>,
    ) -> Result<Self> {
        let mut weights = Vec::with_capacity(num_edges);
        let core = UndirectedCore::new(
            num_vertices,
            num_edges,
            edges.into_iter().map(|(s, t, w)| {
                weights.push(w);
                (s, t)
            }),
        )?;
        Ok(Self { core, weights })
    }

    /// `setEdgeWeight`: the one mutation these graphs allow.
    pub fn set_edge_weight(&mut self, e: usize, weight: f64) -> Result<()> {
        *self.weights.get_mut(e).ok_or(GraphError::EdgeNotFound)? = weight;
        Ok(())
    }
}

undirected_graph_impl!(
    SparseIntUndirectedWeightedGraph,
    true,
    |g: &Self, e: usize| { g.weights.get(e).copied() }
);
