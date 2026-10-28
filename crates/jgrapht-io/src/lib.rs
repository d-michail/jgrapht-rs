// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

#![deny(missing_docs)]
//! Graph import and export.
//!
//! Every format lives in its own module. Importers implement [`GraphImporter`] (read into a graph that
//! you create) and exporters implement [`GraphExporter`]; both report malformed input and failed
//! writes as [`IoError`] and never panic on bad input.
//!
//! | Format | Module | Import | Export |
//! |---|---|---|---|
//! | DIMACS (shortest path, max clique, colouring) | [`dimacs`] | yes | yes |
//! | CSV (edge list, adjacency list, matrix) | [`csv`] | yes | yes |
//! | graph6, sparse6, digraph6 | [`graph6`] | yes | yes |
//! | JSON | [`json`] | yes | yes |
//! | GML | [`gml`] | yes | yes |
//! | DOT (Graphviz) | [`dot`] | yes | yes |
//! | GraphML | [`graphml`] | yes | yes |
//! | GEXF | [`gexf`] | yes | yes |
//! | Lemon | [`lemon`] | yes | yes |
//! | Matrix (adjacency and Laplacians) | [`matrix`] | no | yes |
//! | TSPLIB | [`tsplib`] | yes | no |
//!
//! Importers create vertices through the graph's vertex factory or a factory you install, and add edges
//! to the graph you pass in; an error can leave the vertices and edges read so far in the graph.
//! Exporters take identifiers from a vertex-id provider (the default numbers the vertices) and
//! optional attribute providers.
//!
//! ```
//! use jgrapht::{DiGraph, Graph};
//! use jgrapht_io::dimacs::{DimacsExporter, DimacsFormat, DimacsImporter};
//! use jgrapht_io::{GraphExporter, GraphImporter, Result};
//!
//! let mut graph = DiGraph::<usize>::multi()
//!     .weighted()
//!     .with_vertex_factory(|id| id as usize);
//! DimacsImporter::new().import_str(&mut graph, "p sp 3 2\na 1 2 1.5\na 2 3 2.5\n")?;
//! assert_eq!((graph.vertex_count(), graph.edge_count()), (3, 2));
//!
//! let mut exporter = DimacsExporter::with_vertex_id_provider(|v: &usize| (v + 1).to_string());
//! exporter.set_format(DimacsFormat::ShortestPath);
//! let text = exporter.export_to_string(&graph)?;
//! assert!(text.contains("p sp 3 2"), "{text}");
//! # Ok::<(), jgrapht_io::IoError>(())
//! ```

#![forbid(unsafe_code)]

mod attribute;
pub mod csv;
pub mod dimacs;
pub mod dot;
mod error;
mod event;
mod exporter;
pub mod gexf;
pub mod gml;
pub mod graph6;
pub mod graphml;
pub mod json;
pub mod lemon;
pub mod matrix;
mod traits;
pub mod tsplib;

pub use attribute::{Attribute, AttributeMap, AttributeType, DefaultAttribute};
pub use error::{CallbackError, IoError, Result};
pub use event::ImportEvent;
pub use exporter::{BaseExporter, IntegerIdProvider};
pub use traits::{EventDrivenImporter, GraphExporter, GraphImporter};
