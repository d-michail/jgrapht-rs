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
