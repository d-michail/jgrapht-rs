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

/// Errors raised by fallible graph operations.
///
/// Match on the variant, never on the message text; the messages are not part of the API.
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
    ///
    /// This variant carries no payload so that every shortest-path algorithm can share one error
    /// type. [`BellmanFord::paths_from`](crate::alg::shortestpath::BellmanFord::paths_from) returns a
    /// richer `BellmanFordError` that includes the cycle.
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
    /// The library itself no longer returns it; it stays for code that wraps foreign errors.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// A required parameter was not supplied (a builder or generator that was not configured).
    #[error("missing parameter `{0}`")]
    MissingParameter(&'static str),
    /// Some vertex has no value in a map or function that must cover every vertex (a weight, a
    /// label, a demand, a layout position).
    #[error("no {0} for some vertex")]
    MissingVertexData(&'static str),
    /// The operation cannot run on this many vertices.
    #[error("graph has {got} vertices, at most {max} are supported")]
    TooManyVertices {
        /// The limit.
        max: usize,
        /// The number of vertices of the graph.
        got: usize,
    },
    /// An exact division by zero.
    #[error("division by zero")]
    DivisionByZero,
    /// An exact decimal division whose result has no finite decimal expansion.
    #[error("the decimal expansion does not terminate")]
    NonTerminatingExpansion,
    /// The operation was called in a state that does not allow it (a generator that can run
    /// only once, for example).
    #[error("invalid state: {0}")]
    InvalidState(&'static str),
    /// An internal invariant of the library was broken. This is a bug in `jgrapht`; please report
    /// it.
    #[error("internal error, please report a bug: {0}")]
    Internal(&'static str),
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
    /// A link prediction index is not defined for the queried pair.
    #[error("link prediction index not well defined: {0}")]
    IndexNotWellDefined(String),
    /// An element of a set-like data structure (union-find, list, neighbour cache) is not in it.
    #[error("element not found")]
    ElementNotFound,
    /// An element that must be unique is already present.
    #[error("element already present")]
    DuplicateElement,
    /// A tour, or a sub-tour used to start one, is not valid for the graph.
    #[error("not a valid tour of the graph")]
    InvalidTour,
    /// A flow problem violates the requirements of the algorithm.
    #[error("invalid flow problem: {0}")]
    InvalidFlowProblem(FlowProblem),
}

/// What is wrong with a minimum-cost flow problem; see [`GraphError::InvalidFlowProblem`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FlowProblem {
    /// The node supplies do not add up to zero.
    #[error("the node supplies must add up to zero")]
    SupplyNotBalanced,
    /// An arc has a negative capacity.
    #[error("negative arc capacity")]
    NegativeCapacity,
    /// An arc has a lower capacity above its upper capacity.
    #[error("lower capacity above upper capacity")]
    LowerAboveUpper,
    /// An arc with infinite capacity has a negative cost.
    #[error("infinite capacity with negative cost")]
    InfiniteCapacityNegativeCost,
    /// An arc has an infinite cost.
    #[error("infinite cost")]
    InfiniteCost,
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
    /// Exactly the given value.
    #[error("exactly {0}")]
    Exactly(u64),
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
    /// A collection of collections that overlap neither each other nor themselves.
    #[error("disjoint, without duplicates")]
    Disjoint,
    /// Non-negative probabilities that add up to one.
    #[error("a probability distribution")]
    ProbabilityDistribution,
    /// An index inside the bounds of the structure it addresses.
    #[error("within bounds")]
    InBounds,
    /// Between the two values, both included.
    #[error("between {0} and {1}")]
    InRange(u64, u64),
    /// Between minus the value and the value, both included.
    #[error("between -{0} and {0}")]
    AbsAtMost(u64),
    /// At most the number of vertices of the graph.
    #[error("at most the number of vertices")]
    AtMostVertexCount,
    /// Consistent with the other parameters (an ordering such as minimum <= maximum, or a count
    /// that the other sizes allow).
    #[error("consistent with the other parameters")]
    Consistent,
    /// Small enough for the area it must fit into.
    #[error("small enough to fit the area")]
    FitsArea,
    /// A partition of the vertices that matches the graph.
    #[error("a valid partition of the graph")]
    ValidPartition,
    /// A solution that is valid for the problem.
    #[error("a valid solution")]
    ValidSolution,
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
    /// Having at least one edge.
    #[error("a graph with at least one edge")]
    HasEdges,
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
    /// Satisfying Ore's condition on degree sums.
    #[error("satisfying Ore's property")]
    OreProperty,
    /// A forest with a single root in each tree.
    #[error("a forest with one root per tree")]
    OneRootPerTree,
    /// The same direction as the other graph.
    #[error("of the same direction as the other graph")]
    SameDirection,
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
