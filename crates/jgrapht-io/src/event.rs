use crate::{Attribute, AttributeMap};

/// A value emitted while parsing a graph document.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum ImportEvent<V, E> {
    Start,
    End,
    VertexCount(usize),
    EdgeCount(usize),
    Vertex(V),
    VertexWithAttributes(V, AttributeMap),
    Edge(E),
    EdgeWithAttributes(E, AttributeMap),
    GraphAttribute(String, Attribute),
    VertexAttribute(V, String, Attribute),
    EdgeAttribute(E, String, Attribute),
}
