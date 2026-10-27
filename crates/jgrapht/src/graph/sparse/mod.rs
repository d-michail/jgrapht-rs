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
