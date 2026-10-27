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

//! Failures at the I/O boundary (API roadmap R7): a graph that breaks the `Graph` integrity
//! contract makes every exporter return `IoError::Export` instead of panicking, a user callback
//! that fails surfaces as `IoError::Callback` with its error as the source, and importers document
//! that an error may leave earlier elements in the graph.
use jgrapht::{DefaultEdge, DiGraph, Graph, GraphType, IncidenceGraph, IncomingGraph, OutgoingGraph};
use jgrapht_io::csv::{CsvExporter, CsvFormat, CsvImporter};
use jgrapht_io::dimacs::{DimacsExporter, DimacsImporter};
use jgrapht_io::dot::{DotExporter, DotImporter};
use jgrapht_io::gexf::{GexfExporter, GexfImporter};
use jgrapht_io::gml::{GmlExporter, GmlImporter};
use jgrapht_io::graph6::Graph6Importer;
use jgrapht_io::graphml::{GraphMlExporter, GraphMlImporter};
use jgrapht_io::json::{JsonExporter, JsonImporter};
use jgrapht_io::lemon::LemonExporter;
use jgrapht_io::matrix::{MatrixExporter, MatrixFormat};
use jgrapht_io::{GraphExporter, GraphImporter, IoError};
use std::error::Error;

/// Which part of the integrity contract the graph violates.
#[derive(Clone, Copy)]
enum Defect {
    /// Edge 0 has no source vertex.
    NoSource,
    /// Edge 0 targets vertex 99, which is not a vertex.
    ForeignTarget,
    /// Edge 0 has no weight.
    NoWeight,
}

/// Vertices `0..3` and one edge `0 -> 1` that breaks the contract in one way.
struct BrokenGraph(Defect);

impl Graph for BrokenGraph {
    type Vertex = usize;
    type Edge = usize;
    fn graph_type(&self) -> GraphType {
        GraphType::pseudograph().directed(true).weighted(true)
    }
    fn vertices(&self) -> impl Iterator<Item = &usize> {
        [0usize, 1, 2].iter()
    }
    fn edges(&self) -> impl Iterator<Item = &usize> {
        [0usize].iter()
    }
    fn vertex_count(&self) -> usize {
        3
    }
    fn edge_count(&self) -> usize {
        1
    }
    fn contains_vertex(&self, v: &usize) -> bool {
        *v < 3
    }
    fn contains_edge(&self, e: &usize) -> bool {
        *e == 0
    }
    fn edge_source(&self, e: &usize) -> Option<&usize> {
        match (e, self.0) {
            (0, Defect::NoSource) => None,
            (0, _) => Some(&0),
            _ => None,
        }
    }
    fn edge_target(&self, e: &usize) -> Option<&usize> {
        match (e, self.0) {
            (0, Defect::ForeignTarget) => Some(&99),
            (0, _) => Some(&1),
            _ => None,
        }
    }
    fn edge_weight(&self, e: &usize) -> Option<f64> {
        match (e, self.0) {
            (0, Defect::NoWeight) => None,
            (0, _) => Some(2.0),
            _ => None,
        }
    }
    fn edges_between<'a>(&'a self, _: &'a usize, _: &'a usize) -> impl Iterator<Item = &'a usize> {
        [0usize].iter()
    }
}

impl OutgoingGraph for BrokenGraph {
    fn out_edges_of(&self, _: &usize) -> impl Iterator<Item = &usize> {
        [0usize].iter()
    }
}
impl IncomingGraph for BrokenGraph {
    fn in_edges_of(&self, _: &usize) -> impl Iterator<Item = &usize> {
        [0usize].iter()
    }
}
impl IncidenceGraph for BrokenGraph {
    fn edges_of(&self, _: &usize) -> impl Iterator<Item = &usize> {
        [0usize].iter()
    }
}

fn export<X: GraphExporter<BrokenGraph>>(mut exporter: X, defect: Defect) -> Result<(), IoError> {
    let mut sink = Vec::new();
    exporter.export_graph(&BrokenGraph(defect), &mut sink)
}

fn is_export_error(result: Result<(), IoError>) -> bool {
    if !matches!(result, Err(IoError::Export(_))) {
        eprintln!("not an export error: {result:?}");
        return false;
    }
    true
}

#[test]
fn exporters_report_an_edge_without_endpoints_as_an_error() {
    for defect in [Defect::NoSource, Defect::ForeignTarget] {
        assert!(is_export_error(export(JsonExporter::<usize, usize>::new(), defect)));
        assert!(is_export_error(export(
            CsvExporter::<usize, usize>::new(CsvFormat::AdjacencyList),
            defect
        )));
        assert!(is_export_error(export(
            CsvExporter::<usize, usize>::new(CsvFormat::EdgeList),
            defect
        )));
        assert!(is_export_error(export(
            MatrixExporter::<usize, usize>::new(MatrixFormat::SparseAdjacency),
            defect
        )));
        assert!(is_export_error(export(DimacsExporter::<usize, usize>::new(), defect)));
        assert!(is_export_error(export(DotExporter::<usize, usize>::new(), defect)));
        assert!(is_export_error(export(GmlExporter::<usize, usize>::new(), defect)));
        assert!(is_export_error(export(GexfExporter::<usize, usize>::new(), defect)));
        assert!(is_export_error(export(GraphMlExporter::<usize, usize>::new(), defect)));
        assert!(is_export_error(export(LemonExporter::<usize, usize>::new(), defect)));
    }
}

#[test]
fn exporters_report_an_edge_without_a_weight_as_an_error() {
    let mut csv = CsvExporter::<usize, usize>::new(CsvFormat::EdgeList);
    csv.set_parameter(jgrapht_io::csv::CsvParameter::EdgeWeights, true);
    assert!(is_export_error(export(csv, Defect::NoWeight)));
    let mut gml = GmlExporter::<usize, usize>::new();
    gml.set_export_edge_weights(true);
    assert!(is_export_error(export(gml, Defect::NoWeight)));
}

/// A parse failure of the user's own id type, with an error to preserve.
#[derive(Debug)]
struct NotANumber(String);

impl std::fmt::Display for NotANumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} is not a number", self.0)
    }
}

impl Error for NotANumber {}

fn parse(id: &str) -> Result<usize, jgrapht_io::CallbackError> {
    id.parse().map_err(|_| NotANumber(id.to_owned()).into())
}

fn assert_callback_failure(error: IoError) {
    let IoError::Callback(source) = &error else {
        panic!("expected IoError::Callback, got {error}");
    };
    assert_eq!(source.to_string(), "\"x\" is not a number");
    assert!(error.source().is_some());
    assert!(error.to_string().contains("is not a number"));
}

fn target() -> DiGraph<usize> {
    DiGraph::<usize>::multi()
}

#[test]
fn a_failing_vertex_factory_surfaces_as_a_callback_error_in_every_importer() {
    let mut importer = CsvImporter::<usize>::new(CsvFormat::EdgeList);
    importer.try_set_vertex_factory(parse);
    assert_callback_failure(importer.import_str(&mut target(), "1,x\n").unwrap_err());

    let mut importer = DotImporter::<usize, DefaultEdge>::new();
    importer.try_set_vertex_factory(parse);
    assert_callback_failure(importer.import_str(&mut target(), "digraph { 1 -> x }").unwrap_err());

    let mut importer = JsonImporter::<usize, DefaultEdge>::new();
    importer.try_set_vertex_factory(parse);
    assert_callback_failure(
        importer
            .import_str(&mut target(), r#"{"nodes":[{"id":"x"}],"edges":[]}"#)
            .unwrap_err(),
    );

    let mut importer = GexfImporter::<usize, DefaultEdge>::new();
    importer.try_set_vertex_factory(parse);
    let gexf = r#"<gexf><graph defaultedgetype="directed"><nodes><node id="x"/></nodes><edges/></graph></gexf>"#;
    assert_callback_failure(importer.import_str(&mut target(), gexf).unwrap_err());

    let mut importer = GraphMlImporter::<usize, DefaultEdge>::new();
    importer.try_set_vertex_factory(parse);
    let graphml = r#"<graphml><graph edgedefault="directed"><node id="x"/></graph></graphml>"#;
    assert_callback_failure(importer.import_str(&mut target(), graphml).unwrap_err());
}

#[test]
fn numeric_id_factories_report_their_failures_too() {
    let boom = |id: usize| -> Result<usize, jgrapht_io::CallbackError> {
        Err(format!("no vertex for {id}").into())
    };
    let mut importer = DimacsImporter::<usize>::new();
    importer.try_set_vertex_factory(boom);
    let error = importer.import_str(&mut target(), "p edge 2 1\ne 1 2\n").unwrap_err();
    assert!(matches!(error, IoError::Callback(_)), "{error}");

    let mut importer = Graph6Importer::<usize>::new();
    importer.try_set_vertex_factory(boom);
    let error = importer.import_str(&mut target(), "A_\n").unwrap_err();
    assert!(matches!(error, IoError::Callback(_)), "{error}");

    let mut importer = GmlImporter::<usize, DefaultEdge>::new();
    importer.try_set_vertex_factory(|id: i64| -> Result<usize, jgrapht_io::CallbackError> {
        Err(format!("no vertex for {id}").into())
    });
    let error = importer
        .import_str(&mut target(), "graph [ node [ id 1 ] ]")
        .unwrap_err();
    assert!(matches!(error, IoError::Callback(_)), "{error}");
}

#[test]
fn an_importer_error_leaves_the_elements_read_before_it_in_the_graph() {
    // DIMACS declares three vertices and reads the first edge before the second one fails
    let mut graph = DiGraph::<usize>::multi().with_vertex_factory(|id| id as usize);
    let error = DimacsImporter::<usize>::new()
        .import_str(&mut graph, "p edge 3 2\ne 1 2\ne 1 9\n")
        .unwrap_err();
    assert!(matches!(error, IoError::Import(_) | IoError::Graph(_)), "{error}");
    assert_eq!(graph.vertex_count(), 3);
    assert_eq!(graph.edge_count(), 1);
}
