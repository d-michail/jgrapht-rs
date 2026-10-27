use crate::{ImportEvent, Result};
use jgrapht::{Graph, GraphMut};
use std::io::{Cursor, Read, Write};

pub trait GraphImporter<G: GraphMut> {
    fn import_graph<R: Read>(&mut self, graph: &mut G, input: R) -> Result<()>;

    fn import_str(&mut self, graph: &mut G, input: &str) -> Result<()> {
        self.import_graph(graph, Cursor::new(input.as_bytes()))
    }
}

pub trait GraphExporter<G: Graph> {
    fn export_graph<W: Write>(&mut self, graph: &G, output: W) -> Result<()>;

    fn export_to_string(&mut self, graph: &G) -> Result<String> {
        let mut output = Vec::new();
        self.export_graph(graph, &mut output)?;
        String::from_utf8(output).map_err(|error| {
            crate::IoError::Export(format!("exporter wrote invalid UTF-8: {error}"))
        })
    }
}

pub trait EventDrivenImporter {
    type Vertex;
    type Edge;

    fn import_events<R, F>(&mut self, input: R, emit: F) -> Result<()>
    where
        R: Read,
        F: FnMut(ImportEvent<Self::Vertex, Self::Edge>) -> Result<()>;
}
