//! Format-neutral events emitted while importing graph documents.

#![deny(missing_docs)]

use crate::{Attribute, AttributeMap};

/// A value emitted while parsing a graph document.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum ImportEvent<V, E> {
    /// Parsing of a graph document has started.
    Start,
    /// Parsing of a graph document has completed.
    End,
    /// Declared or discovered number of vertices.
    VertexCount(usize),
    /// Declared or discovered number of edges.
    EdgeCount(usize),
    /// A vertex without attached attributes.
    Vertex(V),
    /// A vertex together with all attributes known at creation time.
    VertexWithAttributes(V, AttributeMap),
    /// An edge without attached attributes.
    Edge(E),
    /// An edge together with all attributes known at creation time.
    EdgeWithAttributes(E, AttributeMap),
    /// A graph-level attribute.
    GraphAttribute(String, Attribute),
    /// An attribute associated with a previously emitted vertex.
    VertexAttribute(V, String, Attribute),
    /// An attribute associated with a previously emitted edge.
    EdgeAttribute(E, String, Attribute),
}
