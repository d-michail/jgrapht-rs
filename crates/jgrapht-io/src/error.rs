use jgrapht::GraphError;
use thiserror::Error;

/// Errors raised while importing or exporting graphs.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum IoError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("graph operation failed: {0}")]
    Graph(#[from] GraphError),
    #[error("import failed: {0}")]
    Import(String),
    #[error("export failed: {0}")]
    Export(String),
}

/// Result type used by fallible `jgrapht-io` operations.
pub type Result<T, E = IoError> = std::result::Result<T, E>;
