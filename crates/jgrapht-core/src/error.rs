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
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

pub type Result<T, E = GraphError> = std::result::Result<T, E>;
