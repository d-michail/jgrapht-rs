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

use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt, UGraph};
use jgrapht_io::gml::{GmlExporter, GmlImporter};
use jgrapht_io::lemon::{LemonExporter, LemonParameter};
use jgrapht_io::matrix::{MatrixExporter, MatrixFormat};
use jgrapht_io::{GraphExporter, GraphImporter};

#[test]
fn gml_imports_and_exports_weighted_graphs() {
    let input = r#"# comment
graph [
  directed 1
  node [ id 1 label "one" ]
  node [ id 2 ]
  edge [ id 7 source 1 target 2 weight 2.5 ]
]"#;
    let mut graph = DiGraph::<usize>::pseudo().weighted();
    let mut importer = GmlImporter::new();
    importer.set_vertex_factory(|id| id as usize);
    importer.import_str(&mut graph, input).unwrap();
    assert_eq!(graph.vertex_count(), 2);
    let edge = graph.get_edge(&1, &2).unwrap();
    assert_eq!(graph.edge_weight(edge), Some(2.5));

    let mut exporter: GmlExporter<usize, DefaultEdge> = GmlExporter::new();
    exporter.set_export_edge_weights(true);
    let output = exporter.export_to_string(&graph).unwrap();
    assert!(output.contains("\tdirected 1"));
    assert!(output.contains("\t\tweight 2.5"));
}

#[test]
fn matrix_formats_match_java_examples() {
    let mut graph = UGraph::<&str>::pseudo();
    graph.add_vertices(["v1", "v2", "v3"]);
    graph.add_edge(&"v1", &"v2").unwrap();
    graph.add_edge(&"v3", &"v1").unwrap();
    graph.add_edge(&"v1", &"v1").unwrap();
    let mut adjacency: MatrixExporter<&str, DefaultEdge> = MatrixExporter::default();
    assert_eq!(
        adjacency.export_to_string(&graph).unwrap(),
        "1 2 1\n1 3 1\n1 1 2\n2 1 1\n3 1 1\n"
    );

    let mut simple = UGraph::<&str>::simple();
    simple.add_vertices(["v1", "v2", "v3"]);
    simple.add_edge(&"v1", &"v2").unwrap();
    simple.add_edge(&"v3", &"v1").unwrap();
    let mut laplacian: MatrixExporter<&str, DefaultEdge> =
        MatrixExporter::new(MatrixFormat::SparseLaplacian);
    assert_eq!(
        laplacian.export_to_string(&simple).unwrap(),
        "1 1 2\n1 2 -1\n1 3 -1\n2 2 1\n2 1 -1\n3 3 1\n3 1 -1\n"
    );
}

#[test]
fn lemon_exporter_writes_weights_and_escaped_ids() {
    let mut graph = DiGraph::<String>::simple().weighted();
    graph.add_vertices(["a a".into(), "b\n".into()]);
    let edge = graph
        .add_edge(&String::from("a a"), &String::from("b\n"))
        .unwrap()
        .unwrap();
    graph.set_edge_weight(&edge, 2.0).unwrap();
    let mut exporter: LemonExporter<String, DefaultEdge> =
        LemonExporter::with_vertex_id_provider(Clone::clone);
    exporter.set_parameter(LemonParameter::ExportEdgeWeights, true);
    exporter.set_parameter(LemonParameter::EscapeStringsAsJava, true);
    let output = exporter.export_to_string(&graph).unwrap();
    assert!(output.contains("\"a a\"\n\"b\\n\""));
    assert!(output.contains("\"a a\"\t\"b\\n\"\t2.0"));
}
