//! Common traits implemented by graph importers and exporters.

#![deny(missing_docs)]

use crate::{ImportEvent, Result};
use jgrapht::{Graph, GraphMut};
use std::io::{Cursor, Read, Write};

/// Imports a complete document directly into a mutable graph.
pub trait GraphImporter<G: GraphMut> {
    /// Reads a graph document from `input` and adds its vertices and edges to `graph`.
    ///
    /// # Errors
    ///
    /// Returns an error when the document is malformed, the input cannot be read, or a requested
    /// graph mutation is unsupported.
    fn import_graph<R: Read>(&mut self, graph: &mut G, input: R) -> Result<()>;

    /// Imports a graph from a UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`GraphImporter::import_graph`].
    fn import_str(&mut self, graph: &mut G, input: &str) -> Result<()> {
        self.import_graph(graph, Cursor::new(input.as_bytes()))
    }
}

/// Exports a read-only graph to a format-specific document.
pub trait GraphExporter<G: Graph> {
    /// Writes `graph` to `output`.
    ///
    /// # Errors
    ///
    /// Returns an error when the graph cannot be represented by the format or the output cannot
    /// be written.
    fn export_graph<W: Write>(&mut self, graph: &G, output: W) -> Result<()>;

    /// Exports `graph` to an owned UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns the same format and I/O errors as [`GraphExporter::export_graph`], or an export
    /// error if an exporter produces bytes that are not valid UTF-8.
    fn export_to_string(&mut self, graph: &G) -> Result<String> {
        let mut output = Vec::new();
        self.export_graph(graph, &mut output)?;
        String::from_utf8(output).map_err(|error| {
            crate::IoError::Export(format!("exporter wrote invalid UTF-8: {error}"))
        })
    }
}

/// Parses a document into a stream of format-neutral [`ImportEvent`] values.
///
/// Event-driven importers let callers inspect or transform a document without constructing a
/// graph, and form the parsing layer used by the graph importers.
pub trait EventDrivenImporter {
    /// Vertex identifier or value emitted by this importer.
    type Vertex;
    /// Edge identifier or endpoint record emitted by this importer.
    type Edge;

    /// Reads `input` and calls `emit` for each import event in document order.
    ///
    /// # Errors
    ///
    /// Returns an error when parsing or reading fails, or propagates an error returned by `emit`.
    fn import_events<R, F>(&mut self, input: R, emit: F) -> Result<()>
    where
        R: Read,
        F: FnMut(ImportEvent<Self::Vertex, Self::Edge>) -> Result<()>;

    /// Parses a UTF-8 string and calls `emit` for each event.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`EventDrivenImporter::import_events`].
    fn import_str<F>(&mut self, input: &str, emit: F) -> Result<()>
    where
        F: FnMut(ImportEvent<Self::Vertex, Self::Edge>) -> Result<()>,
    {
        self.import_events(Cursor::new(input.as_bytes()), emit)
    }
}
