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
    /// An argument violates an operation's preconditions and no more specific variant applies.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// A numeric or collection parameter violates its documented requirement, for example a
    /// non-positive tolerance, radius or count.
    #[error("parameter `{name}` must be {requirement}")]
    InvalidParameter {
        /// The parameter, spelled as in the Rust API (`"k"`, `"tolerance"`, `"max_iterations"`).
        name: &'static str,
        /// What the parameter must satisfy.
        requirement: ParameterRequirement,
    },
    /// An edge weight the operation cannot use.
    #[error("invalid edge weight: {0}")]
    InvalidWeight(WeightProblem),
    /// The graph does not have a property the operation requires and that is not encoded in its
    /// type, such as being simple, complete, Eulerian or planar.
    #[error("graph must be {0}")]
    InvalidGraph(GraphProperty),
    /// The source and the sink of a flow or cut computation are the same vertex.
    #[error("source and sink must differ")]
    SourceEqualsSink,
    /// The problem has no feasible solution, and absence is not a normal outcome of the operation.
    #[error("problem is infeasible")]
    Infeasible,
    /// The problem has no finite optimum.
    #[error("problem is unbounded")]
    Unbounded,
    /// An exact intermediate value exceeded the supported numeric range.
    #[error("arithmetic overflow: {0}")]
    Overflow(String),
    /// A link prediction index is not defined for the queried pair (`LinkPredictionIndexNotWellDefinedException`).
    #[error("link prediction index not well defined: {0}")]
    IndexNotWellDefined(String),
}

/// A requirement on a numeric or collection parameter; see [`GraphError::InvalidParameter`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ParameterRequirement {
    /// Strictly greater than zero.
    #[error("positive")]
    Positive,
    /// Zero or greater.
    #[error("non-negative")]
    NonNegative,
    /// At least the given value.
    #[error("at least {0}")]
    AtLeast(u64),
    /// At most the given value.
    #[error("at most {0}")]
    AtMost(u64),
    /// A non-empty collection.
    #[error("non-empty")]
    NonEmpty,
    /// A collection with an even number of elements.
    #[error("of even size")]
    Even,
    /// A finite number (neither infinite nor NaN).
    #[error("finite")]
    Finite,
    /// A probability in `[0, 1]`.
    #[error("a probability in [0, 1]")]
    Probability,
}

/// Why an edge weight is unusable; see [`GraphError::InvalidWeight`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum WeightProblem {
    /// A negative weight where only non-negative weights are allowed.
    #[error("negative")]
    Negative,
    /// A NaN weight.
    #[error("NaN")]
    NaN,
    /// A negative or NaN weight, from a check that does not distinguish the two.
    #[error("negative or NaN")]
    NegativeOrNaN,
    /// An infinite or NaN weight.
    #[error("not finite")]
    NotFinite,
}

/// A structural property a graph can be required to have; see [`GraphError::InvalidGraph`].
///
/// Properties that the type system already expresses ([`crate::DirectedGraph`],
/// [`crate::UndirectedGraph`], the incidence capabilities) appear here only for operations that
/// accept any graph and check the [`crate::GraphType`] at run time.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum GraphProperty {
    /// All edges directed.
    #[error("directed")]
    Directed,
    /// All edges undirected.
    #[error("undirected")]
    Undirected,
    /// Not mixed: either all edges directed or all undirected.
    #[error("directed or undirected, not mixed")]
    NotMixed,
    /// Without self-loops and parallel edges.
    #[error("simple")]
    Simple,
    /// Simple and directed.
    #[error("simple and directed")]
    SimpleDirected,
    /// Without parallel edges.
    #[error("free of parallel edges")]
    NoParallelEdges,
    /// Without self-loops.
    #[error("free of self-loops")]
    NoSelfLoops,
    /// Allowing self-loops, for a graph that receives them.
    #[error("a graph that allows self-loops")]
    AllowsSelfLoops,
    /// Allowing parallel edges, for a graph that receives them.
    #[error("a graph that allows parallel edges")]
    AllowsParallelEdges,
    /// Having at least one vertex.
    #[error("non-empty")]
    NonEmpty,
    /// Having at least two vertices.
    #[error("a graph with at least two vertices")]
    AtLeastTwoVertices,
    /// Having no vertices and no edges, for a graph that receives a result.
    #[error("empty")]
    Empty,
    /// Every pair of distinct vertices adjacent.
    #[error("complete")]
    Complete,
    /// Complete bipartite with equally sized sides.
    #[error("complete bipartite with equally sized sides")]
    CompleteBipartite,
    /// Two-colorable.
    #[error("bipartite")]
    Bipartite,
    /// A single (weakly) connected component.
    #[error("connected")]
    Connected,
    /// Carrying edge weights.
    #[error("weighted")]
    Weighted,
    /// Without directed cycles.
    #[error("acyclic")]
    Acyclic,
    /// Having an Eulerian cycle.
    #[error("Eulerian")]
    Eulerian,
    /// Embeddable in the plane.
    #[error("planar")]
    Planar,
    /// Not embeddable in the plane.
    #[error("non-planar")]
    NonPlanar,
    /// A tree.
    #[error("a tree")]
    Tree,
}

impl GraphError {
    /// [`GraphError::InvalidParameter`] for a parameter that must be positive.
    pub fn must_be_positive(name: &'static str) -> Self {
        GraphError::InvalidParameter {
            name,
            requirement: ParameterRequirement::Positive,
        }
    }

    /// [`GraphError::InvalidParameter`] for a parameter that must be non-negative.
    pub fn must_be_non_negative(name: &'static str) -> Self {
        GraphError::InvalidParameter {
            name,
            requirement: ParameterRequirement::NonNegative,
        }
    }
}

/// Result type used by fallible `jgrapht` operations.
pub type Result<T, E = GraphError> = std::result::Result<T, E>;
