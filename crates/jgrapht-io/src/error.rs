//! Error and result types shared by graph importers and exporters.

#![deny(missing_docs)]

use jgrapht::GraphError;
use thiserror::Error;

/// Errors raised while importing or exporting graphs.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum IoError {
    /// Reading from or writing to the supplied stream failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A graph mutation or query required by the format failed.
    #[error("graph operation failed: {0}")]
    Graph(#[from] GraphError),
    /// The input document is malformed or unsupported.
    #[error("import failed: {0}")]
    Import(String),
    /// The graph cannot be represented by the requested output format.
    #[error("export failed: {0}")]
    Export(String),
}

/// Result type used by fallible `jgrapht-io` operations.
pub type Result<T, E = IoError> = std::result::Result<T, E>;
