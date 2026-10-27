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
