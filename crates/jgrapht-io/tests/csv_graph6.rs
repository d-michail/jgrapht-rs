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

use jgrapht::{DiGraph, Graph, GraphMut, GraphMutExt, UGraph};
use jgrapht_io::csv::{CsvExporter, CsvFormat, CsvImporter, CsvParameter, escape_field};
use jgrapht_io::graph6::{Graph6Exporter, Graph6Format, Graph6Importer};
use jgrapht_io::{GraphExporter, GraphImporter};

#[test]
fn csv_escaping_and_edge_list_round_trip() {
    assert_eq!(escape_field("a;b", ';'), "\"a;b\"");
    assert_eq!(escape_field("fred\n\"21\"", ';'), "\"fred\n\"\"21\"\"\"");

    let mut graph = DiGraph::<String>::multi().weighted();
    graph.add_vertices(["a".into(), "b;b".into(), "c".into()]);
    let edge = graph.add_edge(&"a".into(), &"b;b".into()).unwrap().unwrap();
    graph.set_edge_weight(&edge, 2.0).unwrap();
    let edge = graph.add_edge(&"b;b".into(), &"c".into()).unwrap().unwrap();
    graph.set_edge_weight(&edge, 3.5).unwrap();

    let mut exporter = CsvExporter::with_vertex_id_provider(
        |vertex: &String| vertex.clone(),
        CsvFormat::EdgeList,
        ';',
    )
    .unwrap();
    exporter.set_parameter(CsvParameter::EdgeWeights, true);
    let text = exporter.export_to_string(&graph).unwrap();
    assert_eq!(text, "a;\"b;b\";2.0\n\"b;b\";c;3.5\n");

    let mut copy = DiGraph::<String>::multi().weighted();
    let mut importer = CsvImporter::with_delimiter(CsvFormat::EdgeList, ';').unwrap();
    importer.set_parameter(CsvParameter::EdgeWeights, true);
    importer.set_vertex_factory(str::to_owned);
    importer.import_str(&mut copy, &text).unwrap();
    assert_eq!(copy.vertex_count(), 3);
    assert_eq!(copy.edge_count(), 2);
}

#[test]
fn csv_matrix_supports_node_ids_and_zero_weights() {
    let input = ";a;b;c\nc;0;0;0.0\na;0;2.0;0\nb;0;0;0\n";
    let mut graph = DiGraph::<String>::pseudo().weighted();
    let mut importer = CsvImporter::with_delimiter(CsvFormat::Matrix, ';').unwrap();
    importer.set_parameter(CsvParameter::MatrixNodeId, true);
    importer.set_parameter(CsvParameter::MatrixZeroWhenNoEdge, true);
    importer.set_parameter(CsvParameter::EdgeWeights, true);
    importer.set_vertex_factory(str::to_owned);
    importer.import_str(&mut graph, input).unwrap();
    assert_eq!(graph.edge_count(), 2);
    assert_eq!(
        graph.edge_weight(graph.edge_between(&"a".into(), &"b".into()).unwrap()),
        Some(2.0)
    );
    assert_eq!(
        graph.edge_weight(graph.edge_between(&"c".into(), &"c".into()).unwrap()),
        Some(0.0)
    );
}

#[test]
fn graph6_known_values_and_round_trip() {
    let mut path = UGraph::<usize>::simple();
    path.add_vertices(0..4);
    path.add_edge(&0, &1).unwrap();
    path.add_edge(&1, &2).unwrap();
    path.add_edge(&2, &3).unwrap();
    let mut exporter = Graph6Exporter::new(Graph6Format::Graph6);
    let encoded = exporter.export_to_string(&path).unwrap();
    assert_eq!(encoded, "Ch");

    let mut decoded = UGraph::<usize>::simple().with_vertex_factory(|id| id as usize);
    Graph6Importer::new()
        .import_str(&mut decoded, &encoded)
        .unwrap();
    assert_eq!(decoded.vertex_count(), 4);
    assert_eq!(decoded.edge_count(), 3);
}

#[test]
fn sparse6_preserves_parallel_edges_and_loops() {
    let mut graph = UGraph::<usize>::pseudo();
    graph.add_vertices(0..3);
    graph.add_edge(&0, &0).unwrap();
    graph.add_edge(&0, &1).unwrap();
    graph.add_edge(&0, &1).unwrap();
    graph.add_edge(&1, &2).unwrap();
    let mut exporter = Graph6Exporter::new(Graph6Format::Sparse6);
    let encoded = exporter.export_to_string(&graph).unwrap();

    let mut decoded = UGraph::<usize>::pseudo().with_vertex_factory(|id| id as usize);
    Graph6Importer::new()
        .import_str(&mut decoded, &encoded)
        .unwrap();
    assert_eq!(decoded.vertex_count(), 3);
    assert_eq!(decoded.edge_count(), 4);
}

#[test]
fn digraph6_preserves_direction_and_loops() {
    let mut graph = DiGraph::<usize>::pseudo();
    graph.add_vertices(0..3);
    graph.add_edge(&0, &1).unwrap();
    graph.add_edge(&1, &0).unwrap();
    graph.add_edge(&2, &2).unwrap();
    let mut exporter = Graph6Exporter::new(Graph6Format::Digraph6);
    let encoded = exporter.export_to_string(&graph).unwrap();
    assert!(encoded.starts_with('&'));

    let mut decoded = DiGraph::<usize>::pseudo().with_vertex_factory(|id| id as usize);
    Graph6Importer::new()
        .import_str(&mut decoded, &encoded)
        .unwrap();
    assert!(decoded.contains_edge_between(&0, &1));
    assert!(decoded.contains_edge_between(&1, &0));
    assert!(decoded.contains_edge_between(&2, &2));
}
