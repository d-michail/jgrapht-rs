//! Immutable graphs on integer vertices and edges backed by compressed sparse rows
//! (`org.jgrapht.opt.graph.sparse`).
//!
//! Vertices are `0..n` and edges `0..m` in the order the edges were given. The graphs implement
//! [`Graph`](crate::graph::Graph) only; there is no `GraphMut` implementation. Because the trait
//! hands out `&usize`, each graph keeps one identity array of `max(n, m)` ids to borrow from.

mod csr;
mod directed;
mod undirected;

pub use csr::CsrBooleanMatrix;
pub use directed::{IncomingEdgesSupport, SparseIntDirectedGraph, SparseIntDirectedWeightedGraph};
pub use undirected::{SparseIntUndirectedGraph, SparseIntUndirectedWeightedGraph};

use crate::{GraphError, Result};

fn identity_ids(n: usize, m: usize) -> Vec<usize> {
    (0..n.max(m)).collect()
}

/// Splits `(source, target)` pairs into the two endpoint arrays, checking the endpoint range and
/// the announced edge count.
fn collect_endpoints(
    num_vertices: usize,
    num_edges: usize,
    edges: impl IntoIterator<Item = (usize, usize)>,
) -> Result<(Vec<usize>, Vec<usize>)> {
    let mut source = Vec::with_capacity(num_edges);
    let mut target = Vec::with_capacity(num_edges);
    for (s, t) in edges {
        if s >= num_vertices || t >= num_vertices {
            return Err(GraphError::InvalidArgument(format!(
                "no such vertex in graph: {}",
                if s >= num_vertices { s } else { t }
            )));
        }
        source.push(s);
        target.push(t);
    }
    if source.len() != num_edges {
        return Err(GraphError::InvalidArgument(format!(
            "expected {num_edges} edges, got {}",
            source.len()
        )));
    }
    Ok((source, target))
}
