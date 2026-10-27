use thiserror::Error;

/// Errors raised by fallible graph operations (JGraphT throws unchecked exceptions instead).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GraphError {
    #[error("vertex not found in graph")]
    VertexNotFound,
    #[error("edge not found in graph")]
    EdgeNotFound,
    #[error("self-loops are not allowed in this graph")]
    LoopsNotAllowed,
    #[error("adding the edge would create a cycle")]
    CycleProhibited,
    #[error("graph contains a negative-weight cycle")]
    NegativeCycle,
    #[error("graph is not weighted")]
    NotWeighted,
    #[error("graph is unmodifiable")]
    Unmodifiable,
    #[error("graph has no edge factory")]
    NoEdgeFactory,
    #[error("graph has no vertex factory")]
    NoVertexFactory,
    #[error("edge already associated with different endpoints")]
    IntrusiveEdge,
    #[error("unsupported operation: {0}")]
    Unsupported(String),
    #[error("invalid graph walk: {0}")]
    InvalidGraphWalk(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("arithmetic overflow: {0}")]
    Overflow(String),
    /// A link prediction index is not defined for the queried pair (`LinkPredictionIndexNotWellDefinedException`).
    #[error("link prediction index not well defined: {0}")]
    IndexNotWellDefined(String),
}

pub type Result<T, E = GraphError> = std::result::Result<T, E>;
