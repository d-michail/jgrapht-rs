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

//! Lemon graph format (LGF) exporter.
//!
//! LGF is the custom graph format used by the
//! [LEMON graph library](https://lemon.cs.elte.hu).

#![deny(missing_docs)]

use crate::exporter::format_f64;
use crate::{BaseExporter, GraphExporter, IntegerIdProvider, Result};
use jgrapht::Graph;
use std::borrow::Borrow;
use std::hash::Hash;
use std::io::Write;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Options which modify LEMON export behavior.
pub enum LemonParameter {
    /// Emit graph edge weights in the arc table.
    ExportEdgeWeights,
    /// Quote identifiers and escape them using Java string-literal rules.
    EscapeStringsAsJava,
}

/// Exports a graph in LEMON graph format (LGF).
///
/// Vertices receive sequential integer labels by default. The output contains
/// a node table followed by an arc table and can optionally include edge
/// weights or Java-style escaped identifiers.
pub struct LemonExporter<V, E> {
    base: BaseExporter<V, E>,
    export_weights: bool,
    escape_strings: bool,
}

impl<V, E> LemonExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    /// Creates an exporter which assigns sequential integer vertex identifiers.
    pub fn new() -> Self {
        let mut ids = IntegerIdProvider::new();
        Self::with_vertex_id_provider(move |vertex| ids.id(vertex))
    }

    /// Creates an exporter using `provider` to generate vertex identifiers.
    pub fn with_vertex_id_provider(provider: impl FnMut(&V) -> String + 'static) -> Self {
        Self {
            base: BaseExporter::new(provider),
            export_weights: false,
            escape_strings: false,
        }
    }

    /// Enables or disables an export option.
    pub fn set_parameter(&mut self, parameter: LemonParameter, enabled: bool) {
        match parameter {
            LemonParameter::ExportEdgeWeights => self.export_weights = enabled,
            LemonParameter::EscapeStringsAsJava => self.escape_strings = enabled,
        }
    }

    /// Returns whether an export option is enabled.
    pub fn is_parameter(&self, parameter: LemonParameter) -> bool {
        match parameter {
            LemonParameter::ExportEdgeWeights => self.export_weights,
            LemonParameter::EscapeStringsAsJava => self.escape_strings,
        }
    }

    fn prepare(&self, value: &str) -> String {
        if !self.escape_strings {
            return value.to_owned();
        }
        let escaped = value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        format!("\"{escaped}\"")
    }
}

impl<V, E> Default for LemonExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphExporter<G> for LemonExporter<V, E>
where
    G: Graph<Vertex = V, Edge = E>,
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash,
{
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        writeln!(output, "#Creator: JGraphT Lemon (LGF) Exporter")?;
        writeln!(output, "#Version: 1\n")?;
        writeln!(output, "@nodes")?;
        writeln!(output, "label")?;
        for vertex in graph.vertices() {
            let vertex = vertex.borrow();
            let id = self.base.vertex_id(vertex)?;
            writeln!(output, "{}", self.prepare(&id))?;
        }
        writeln!(output)?;
        writeln!(output, "@arcs")?;
        writeln!(
            output,
            "\t\t{}",
            if self.export_weights { "weight" } else { "-" }
        )?;
        for edge in graph.edges() {
            let edge = edge.borrow();
            let (source, target) = crate::exporter::edge_endpoints(graph, edge)?;
            let source = self.base.vertex_id(source)?;
            let target = self.base.vertex_id(target)?;
            write!(
                output,
                "{}\t{}",
                self.prepare(&source),
                self.prepare(&target)
            )?;
            if self.export_weights {
                write!(
                    output,
                    "\t{}",
                    format_f64(crate::exporter::edge_weight(graph, edge)?)
                )?;
            }
            writeln!(output)?;
        }
        writeln!(output)?;
        Ok(())
    }
}
