use jgrapht::GraphError;
use thiserror::Error;

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

pub type Result<T, E = IoError> = std::result::Result<T, E>;
