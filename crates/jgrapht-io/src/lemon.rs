//! Lemon graph format (LGF) exporter.

use crate::exporter::format_f64;
use crate::{BaseExporter, GraphExporter, IntegerIdProvider, IoError, Result};
use jgrapht::Graph;
use std::hash::Hash;
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LemonParameter {
    ExportEdgeWeights,
    EscapeStringsAsJava,
}

pub struct LemonExporter<V, E> {
    base: BaseExporter<V, E>,
    export_weights: bool,
    escape_strings: bool,
}

impl<V, E> LemonExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    pub fn new() -> Self {
        let mut ids = IntegerIdProvider::new();
        Self::with_vertex_id_provider(move |vertex| ids.id(vertex))
    }

    pub fn with_vertex_id_provider(provider: impl FnMut(&V) -> String + 'static) -> Self {
        Self {
            base: BaseExporter::new(provider),
            export_weights: false,
            escape_strings: false,
        }
    }

    pub fn set_parameter(&mut self, parameter: LemonParameter, enabled: bool) {
        match parameter {
            LemonParameter::ExportEdgeWeights => self.export_weights = enabled,
            LemonParameter::EscapeStringsAsJava => self.escape_strings = enabled,
        }
    }

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
            let source = graph
                .edge_source(edge)
                .ok_or_else(|| IoError::Export("edge has no source".into()))?;
            let target = graph
                .edge_target(edge)
                .ok_or_else(|| IoError::Export("edge has no target".into()))?;
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
                    format_f64(graph.edge_weight(edge).unwrap_or(1.0))
                )?;
            }
            writeln!(output)?;
        }
        writeln!(output)?;
        Ok(())
    }
}
