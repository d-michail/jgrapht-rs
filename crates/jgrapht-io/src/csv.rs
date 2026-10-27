//! Delimiter-separated edge lists, adjacency lists, and matrices.

use crate::exporter::format_f64;
use crate::{
    BaseExporter, EventDrivenImporter, GraphExporter, GraphImporter, ImportEvent,
    IntegerIdProvider, IoError, Result,
};
use jgrapht::{Graph, GraphMut};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io::{Read, Write};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CsvFormat {
    EdgeList,
    #[default]
    AdjacencyList,
    Matrix,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CsvParameter {
    EdgeWeights,
    MatrixNodeId,
    MatrixZeroWhenNoEdge,
}

pub type CsvEdge = (String, String, Option<f64>);

fn valid_delimiter(delimiter: char) -> bool {
    !matches!(delimiter, '\r' | '\n' | '"')
}

pub fn escape_field(field: &str, delimiter: char) -> String {
    if field.contains(delimiter) || field.contains('"') || field.contains(['\r', '\n']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
}

/// Unescapes a delimiter-separated field using JGraphT's DSV rules.
pub fn unescape_field(field: &str, delimiter: char) -> String {
    if field.len() < 2 || !field.starts_with('"') || !field.ends_with('"') {
        return field.to_owned();
    }
    let inner = &field[1..field.len() - 1];
    if inner.contains(delimiter) || inner.contains('"') || inner.contains(['\r', '\n']) {
        inner.replace("\"\"", "\"")
    } else {
        field.to_owned()
    }
}

fn parse_records(input: &str, delimiter: char) -> Result<Vec<Vec<String>>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut chars = input.chars().peekable();
    let mut quoted = false;
    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(ch);
            }
            continue;
        }
        match ch {
            '"' if field.is_empty() => quoted = true,
            ch if ch == delimiter => {
                record.push(std::mem::take(&mut field));
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            '\n' => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            _ => field.push(ch),
        }
    }
    if quoted {
        return Err(IoError::Import(
            "CSV input has an unterminated quoted field".into(),
        ));
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    Ok(records)
}

#[derive(Debug, Clone)]
pub struct CsvEventDrivenImporter {
    format: CsvFormat,
    delimiter: char,
    parameters: HashSet<CsvParameter>,
}

impl Default for CsvEventDrivenImporter {
    fn default() -> Self {
        Self {
            format: CsvFormat::default(),
            delimiter: ',',
            parameters: HashSet::new(),
        }
    }
}

impl CsvEventDrivenImporter {
    pub fn new(format: CsvFormat) -> Self {
        Self {
            format,
            ..Self::default()
        }
    }

    pub const fn format(&self) -> CsvFormat {
        self.format
    }

    pub fn set_format(&mut self, format: CsvFormat) {
        self.format = format;
    }

    pub const fn delimiter(&self) -> char {
        self.delimiter
    }

    pub fn set_delimiter(&mut self, delimiter: char) -> Result<()> {
        if !valid_delimiter(delimiter) {
            return Err(IoError::Import("invalid CSV delimiter".into()));
        }
        self.delimiter = delimiter;
        Ok(())
    }

    pub fn set_parameter(&mut self, parameter: CsvParameter, enabled: bool) {
        if enabled {
            self.parameters.insert(parameter);
        } else {
            self.parameters.remove(&parameter);
        }
    }

    pub fn is_parameter(&self, parameter: CsvParameter) -> bool {
        self.parameters.contains(&parameter)
    }

    fn emit_vertex<F>(seen: &mut HashSet<String>, id: &str, emit: &mut F) -> Result<()>
    where
        F: FnMut(ImportEvent<String, CsvEdge>) -> Result<()>,
    {
        if seen.insert(id.to_owned()) {
            emit(ImportEvent::Vertex(id.to_owned()))?;
        }
        Ok(())
    }
}

impl EventDrivenImporter for CsvEventDrivenImporter {
    type Vertex = String;
    type Edge = CsvEdge;

    fn import_events<R, F>(&mut self, mut input: R, mut emit: F) -> Result<()>
    where
        R: Read,
        F: FnMut(ImportEvent<Self::Vertex, Self::Edge>) -> Result<()>,
    {
        let mut text = String::new();
        input.read_to_string(&mut text)?;
        let records = parse_records(&text, self.delimiter)?;
        emit(ImportEvent::Start)?;
        let weighted = self.is_parameter(CsvParameter::EdgeWeights);
        let mut seen = HashSet::new();
        match self.format {
            CsvFormat::EdgeList | CsvFormat::AdjacencyList => {
                for (row_index, row) in records.iter().enumerate() {
                    if row.is_empty() || (row.len() == 1 && row[0].is_empty()) {
                        return Err(IoError::Import(format!(
                            "CSV row {} is empty",
                            row_index + 1
                        )));
                    }
                    let source = &row[0];
                    if source.is_empty() {
                        return Err(IoError::Import(format!(
                            "CSV row {} has an empty source",
                            row_index + 1
                        )));
                    }
                    Self::emit_vertex(&mut seen, source, &mut emit)?;
                    let step = if weighted { 2 } else { 1 };
                    if (row.len() - 1) % step != 0 {
                        return Err(IoError::Import(format!(
                            "CSV row {} has an incomplete weighted edge",
                            row_index + 1
                        )));
                    }
                    let targets = if self.format == CsvFormat::EdgeList {
                        if row.len() != 1 + step {
                            return Err(IoError::Import(format!(
                                "CSV edge-list row {} must contain exactly one edge",
                                row_index + 1
                            )));
                        }
                        &row[1..]
                    } else {
                        &row[1..]
                    };
                    for fields in targets.chunks(step) {
                        let target = &fields[0];
                        if target.is_empty() {
                            return Err(IoError::Import(format!(
                                "CSV row {} has an empty target",
                                row_index + 1
                            )));
                        }
                        Self::emit_vertex(&mut seen, target, &mut emit)?;
                        let weight = if weighted {
                            Some(fields[1].parse::<f64>().map_err(|_| {
                                IoError::Import(format!(
                                    "CSV row {} has an invalid edge weight",
                                    row_index + 1
                                ))
                            })?)
                        } else {
                            None
                        };
                        emit(ImportEvent::Edge((source.clone(), target.clone(), weight)))?;
                    }
                }
            }
            CsvFormat::Matrix => {
                if records.is_empty() {
                    return Err(IoError::Import("CSV matrix is empty".into()));
                }
                let node_ids = self.is_parameter(CsvParameter::MatrixNodeId);
                let zero_missing = self.is_parameter(CsvParameter::MatrixZeroWhenNoEdge);
                let (columns, rows): (Vec<String>, &[Vec<String>]) = if node_ids {
                    let header = &records[0];
                    if header.len() < 2 || !header[0].is_empty() {
                        return Err(IoError::Import(
                            "CSV matrix node-id header must start with an empty field".into(),
                        ));
                    }
                    let columns = header[1..].to_vec();
                    if columns.iter().any(|id| id.trim().is_empty()) {
                        return Err(IoError::Import(
                            "CSV matrix header contains an empty node id".into(),
                        ));
                    }
                    (columns, &records[1..])
                } else {
                    let count = records[0].len();
                    ((1..=count).map(|id| id.to_string()).collect(), &records)
                };
                for id in &columns {
                    Self::emit_vertex(&mut seen, id, &mut emit)?;
                }
                if rows.len() != columns.len() {
                    return Err(IoError::Import(format!(
                        "CSV matrix has {} data rows but {} columns",
                        rows.len(),
                        columns.len()
                    )));
                }
                let column_set: HashSet<_> = columns.iter().cloned().collect();
                let mut row_ids = HashSet::new();
                for (row_index, row) in rows.iter().enumerate() {
                    let (source, entries) = if node_ids {
                        let Some((source, entries)) = row.split_first() else {
                            return Err(IoError::Import("CSV matrix row is empty".into()));
                        };
                        if !column_set.contains(source) || !row_ids.insert(source.clone()) {
                            return Err(IoError::Import(format!(
                                "CSV matrix row {} has an unknown or duplicate node id",
                                row_index + 1
                            )));
                        }
                        (source, entries)
                    } else {
                        (&columns[row_index], row.as_slice())
                    };
                    if entries.len() != columns.len() {
                        return Err(IoError::Import(format!(
                            "CSV matrix row {} has {} entries; expected {}",
                            row_index + 1,
                            entries.len(),
                            columns.len()
                        )));
                    }
                    for (target, entry) in columns.iter().zip(entries) {
                        if entry.is_empty() {
                            continue;
                        }
                        if let Ok(integer) = entry.parse::<i64>() {
                            if integer == 0 && zero_missing {
                                continue;
                            }
                            let weight = weighted.then_some(integer as f64);
                            if integer != 0 || weighted {
                                emit(ImportEvent::Edge((source.clone(), target.clone(), weight)))?;
                            }
                            continue;
                        }
                        let number = entry.parse::<f64>().map_err(|_| {
                            IoError::Import(format!(
                                "CSV matrix row {} contains invalid entry {entry:?}",
                                row_index + 1
                            ))
                        })?;
                        if !weighted {
                            return Err(IoError::Import(
                                "CSV matrix contains a floating-point value without edge weights"
                                    .into(),
                            ));
                        }
                        emit(ImportEvent::Edge((
                            source.clone(),
                            target.clone(),
                            Some(number),
                        )))?;
                    }
                }
            }
        }
        emit(ImportEvent::End)
    }
}

type VertexFactory<V> = Box<dyn FnMut(&str) -> V>;

pub struct CsvImporter<V> {
    parser: CsvEventDrivenImporter,
    vertex_factory: Option<VertexFactory<V>>,
}

impl<V> Default for CsvImporter<V> {
    fn default() -> Self {
        Self::new(CsvFormat::default())
    }
}

impl<V> CsvImporter<V> {
    pub fn new(format: CsvFormat) -> Self {
        Self {
            parser: CsvEventDrivenImporter::new(format),
            vertex_factory: None,
        }
    }

    pub fn with_delimiter(format: CsvFormat, delimiter: char) -> Result<Self> {
        let mut importer = Self::new(format);
        importer.parser.set_delimiter(delimiter)?;
        Ok(importer)
    }

    pub fn set_parameter(&mut self, parameter: CsvParameter, enabled: bool) {
        self.parser.set_parameter(parameter, enabled);
    }

    pub const fn format(&self) -> CsvFormat {
        self.parser.format()
    }

    pub fn set_format(&mut self, format: CsvFormat) {
        self.parser.set_format(format);
    }

    pub const fn delimiter(&self) -> char {
        self.parser.delimiter()
    }

    pub fn set_delimiter(&mut self, delimiter: char) -> Result<()> {
        self.parser.set_delimiter(delimiter)
    }

    pub fn is_parameter(&self, parameter: CsvParameter) -> bool {
        self.parser.is_parameter(parameter)
    }

    pub fn set_vertex_factory(&mut self, factory: impl FnMut(&str) -> V + 'static) {
        self.vertex_factory = Some(Box::new(factory));
    }
}

impl<V, G> GraphImporter<G> for CsvImporter<V>
where
    G: GraphMut<Vertex = V>,
    V: Clone,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, input: R) -> Result<()> {
        let factory = &mut self.vertex_factory;
        let mut vertices = HashMap::<String, V>::new();
        self.parser.import_events(input, |event| {
            match event {
                ImportEvent::Vertex(id) => {
                    let vertex = if let Some(factory) = factory.as_mut() {
                        let vertex = factory(&id);
                        if !graph.add_vertex(vertex.clone()) {
                            return Err(IoError::Import(format!(
                                "vertex factory produced a duplicate for CSV id {id:?}"
                            )));
                        }
                        vertex
                    } else {
                        graph.add_new_vertex()?
                    };
                    vertices.insert(id, vertex);
                }
                ImportEvent::Edge((source, target, weight)) => {
                    let from = vertices.get(&source).ok_or_else(|| {
                        IoError::Import(format!("CSV node {source:?} does not exist"))
                    })?;
                    let to = vertices.get(&target).ok_or_else(|| {
                        IoError::Import(format!("CSV node {target:?} does not exist"))
                    })?;
                    let edge = graph.add_edge(from, to)?.ok_or_else(|| {
                        IoError::Import(format!("graph rejected CSV edge {source:?} -> {target:?}"))
                    })?;
                    if graph.graph_type().weighted
                        && let Some(weight) = weight
                    {
                        graph.set_edge_weight(&edge, weight)?;
                    }
                }
                _ => {}
            }
            Ok(())
        })
    }
}

pub struct CsvExporter<V, E> {
    base: BaseExporter<V, E>,
    format: CsvFormat,
    delimiter: char,
    parameters: HashSet<CsvParameter>,
}

impl<V, E> CsvExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    pub fn new(format: CsvFormat) -> Self {
        let mut ids = IntegerIdProvider::new();
        Self::with_vertex_id_provider(move |vertex| ids.id(vertex), format, ',')
            .expect("comma is a valid delimiter")
    }

    pub fn with_vertex_id_provider(
        provider: impl FnMut(&V) -> String + 'static,
        format: CsvFormat,
        delimiter: char,
    ) -> Result<Self> {
        if !valid_delimiter(delimiter) {
            return Err(IoError::Export("invalid CSV delimiter".into()));
        }
        Ok(Self {
            base: BaseExporter::new(provider),
            format,
            delimiter,
            parameters: HashSet::new(),
        })
    }

    pub fn set_parameter(&mut self, parameter: CsvParameter, enabled: bool) {
        if enabled {
            self.parameters.insert(parameter);
        } else {
            self.parameters.remove(&parameter);
        }
    }

    pub const fn format(&self) -> CsvFormat {
        self.format
    }

    pub fn set_format(&mut self, format: CsvFormat) {
        self.format = format;
    }

    pub const fn delimiter(&self) -> char {
        self.delimiter
    }

    pub fn set_delimiter(&mut self, delimiter: char) -> Result<()> {
        if !valid_delimiter(delimiter) {
            return Err(IoError::Export("invalid CSV delimiter".into()));
        }
        self.delimiter = delimiter;
        Ok(())
    }

    pub fn is_parameter(&self, parameter: CsvParameter) -> bool {
        self.parameters.contains(&parameter)
    }
}

impl<V, E, G> GraphExporter<G> for CsvExporter<V, E>
where
    G: Graph<Vertex = V, Edge = E>,
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash,
{
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        let weighted = self.parameters.contains(&CsvParameter::EdgeWeights);
        match self.format {
            CsvFormat::EdgeList => {
                for edge in graph.edges() {
                    let source = self.base.vertex_id(
                        graph
                            .edge_source(edge)
                            .ok_or_else(|| IoError::Export("edge has no source".into()))?,
                    )?;
                    let target = self.base.vertex_id(
                        graph
                            .edge_target(edge)
                            .ok_or_else(|| IoError::Export("edge has no target".into()))?,
                    )?;
                    write!(
                        output,
                        "{}{}{}",
                        escape_field(&source, self.delimiter),
                        self.delimiter,
                        escape_field(&target, self.delimiter)
                    )?;
                    if weighted {
                        let weight = graph
                            .edge_weight(edge)
                            .ok_or_else(|| IoError::Export("edge has no weight".into()))?;
                        write!(output, "{}{}", self.delimiter, format_f64(weight))?;
                    }
                    writeln!(output)?;
                }
            }
            CsvFormat::AdjacencyList => {
                let vertices: Vec<_> = graph.vertices().cloned().collect();
                for vertex in &vertices {
                    let id = self.base.vertex_id(vertex)?;
                    write!(output, "{}", escape_field(&id, self.delimiter))?;
                    for edge in graph.out_edges_of(vertex).unwrap() {
                        let source = graph.edge_source(edge).expect("graph edge source");
                        let target = graph.edge_target(edge).expect("graph edge target");
                        let opposite = if source == vertex { target } else { source };
                        let id = self.base.vertex_id(opposite)?;
                        write!(
                            output,
                            "{}{}",
                            self.delimiter,
                            escape_field(&id, self.delimiter)
                        )?;
                        if weighted {
                            let weight = graph
                                .edge_weight(edge)
                                .ok_or_else(|| IoError::Export("edge has no weight".into()))?;
                            write!(output, "{}{}", self.delimiter, format_f64(weight))?;
                        }
                    }
                    writeln!(output)?;
                }
            }
            CsvFormat::Matrix => {
                let vertices: Vec<_> = graph.vertices().cloned().collect();
                let node_ids = self.parameters.contains(&CsvParameter::MatrixNodeId);
                let zero_missing = self
                    .parameters
                    .contains(&CsvParameter::MatrixZeroWhenNoEdge);
                if node_ids {
                    for vertex in &vertices {
                        let id = self.base.vertex_id(vertex)?;
                        write!(
                            output,
                            "{}{}",
                            self.delimiter,
                            escape_field(&id, self.delimiter)
                        )?;
                    }
                    writeln!(output)?;
                }
                for (row, vertex) in vertices.iter().enumerate() {
                    if node_ids {
                        let id = self.base.vertex_id(vertex)?;
                        write!(
                            output,
                            "{}{}",
                            escape_field(&id, self.delimiter),
                            self.delimiter
                        )?;
                    }
                    for (column, target) in vertices.iter().enumerate() {
                        if let Some(edge) = graph.get_edge(vertex, target) {
                            if weighted {
                                write!(
                                    output,
                                    "{}",
                                    format_f64(graph.edge_weight(edge).unwrap_or(1.0))
                                )?;
                            } else {
                                output.write_all(b"1")?;
                            }
                        } else if zero_missing {
                            output.write_all(b"0")?;
                        }
                        if column + 1 < vertices.len() {
                            write!(output, "{}", self.delimiter)?;
                        }
                    }
                    if row < vertices.len() {
                        writeln!(output)?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Exports the CSV dialect accepted by Microsoft Visio's graph importer.
pub struct VisioExporter<V, E> {
    base: BaseExporter<V, E>,
}

impl<V, E> VisioExporter<V, E>
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
        }
    }
}

impl<V, E> Default for VisioExporter<V, E>
where
    V: Clone + Eq + Hash + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V, E, G> GraphExporter<G> for VisioExporter<V, E>
where
    G: Graph<Vertex = V, Edge = E>,
    V: Clone + Eq + Hash + 'static,
    E: Clone + Eq + Hash,
{
    fn export_graph<W: Write>(&mut self, graph: &G, mut output: W) -> Result<()> {
        for vertex in graph.vertices() {
            let name = self.base.vertex_id(vertex)?;
            writeln!(output, "Shape,{name},,{name}")?;
        }
        for edge in graph.edges() {
            let source = graph
                .edge_source(edge)
                .ok_or_else(|| IoError::Export("edge has no source".into()))?;
            let target = graph
                .edge_target(edge)
                .ok_or_else(|| IoError::Export("edge has no target".into()))?;
            let source = self.base.vertex_id(source)?;
            let target = self.base.vertex_id(target)?;
            writeln!(output, "Link,{source}-->{target},,,{source},{target}")?;
        }
        Ok(())
    }
}
