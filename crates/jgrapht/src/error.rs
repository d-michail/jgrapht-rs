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

//! Error and result types shared by the core crate.

#![deny(missing_docs)]

use thiserror::Error;

/// Errors raised by fallible graph operations (JGraphT throws unchecked exceptions instead).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GraphError {
    /// A requested vertex is not present in the graph.
    #[error("vertex not found in graph")]
    VertexNotFound,
    /// A requested edge is not present in the graph.
    #[error("edge not found in graph")]
    EdgeNotFound,
    /// An operation attempted to add a self-loop to a graph that forbids loops.
    #[error("self-loops are not allowed in this graph")]
    LoopsNotAllowed,
    /// An edge insertion would violate the graph's acyclic invariant.
    #[error("adding the edge would create a cycle")]
    CycleProhibited,
    /// A negative-weight cycle was found while computing shortest paths.
    #[error("graph contains a negative-weight cycle")]
    NegativeCycle,
    /// A weighted operation was requested from an unweighted graph.
    #[error("graph is not weighted")]
    NotWeighted,
    /// A mutation was requested from a read-only graph or view.
    #[error("graph is unmodifiable")]
    Unmodifiable,
    /// The graph cannot create edges because no edge factory was configured.
    #[error("graph has no edge factory")]
    NoEdgeFactory,
    /// The graph cannot create vertices because no vertex factory was configured.
    #[error("graph has no vertex factory")]
    NoVertexFactory,
    /// An intrusive edge value is already associated with different endpoints.
    #[error("edge already associated with different endpoints")]
    IntrusiveEdge,
    /// The graph representation does not support the requested operation.
    #[error("unsupported operation: {0}")]
    Unsupported(String),
    /// A supplied walk is inconsistent with the graph or its endpoints.
    #[error("invalid graph walk: {0}")]
    InvalidGraphWalk(String),
    /// An argument violates an operation's preconditions.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// An exact intermediate value exceeded the supported numeric range.
    #[error("arithmetic overflow: {0}")]
    Overflow(String),
    /// A link prediction index is not defined for the queried pair (`LinkPredictionIndexNotWellDefinedException`).
    #[error("link prediction index not well defined: {0}")]
    IndexNotWellDefined(String),
}

/// Result type used by fallible `jgrapht` operations.
pub type Result<T, E = GraphError> = std::result::Result<T, E>;
