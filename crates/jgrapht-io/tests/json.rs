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

use indexmap::IndexMap;
use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt};
use jgrapht_io::json::{JsonExporter, JsonImporter};
use jgrapht_io::{Attribute, AttributeType, GraphExporter, GraphImporter, IntegerIdProvider};

#[test]
fn imports_ids_weights_and_nested_attributes() {
    let input = r#"{
        "nodes": [
          {"id": 1, "label": "one", "custom": {"pi": 3.14}},
          {"id": "2", "enabled": true}
        ],
        "edges": [{"source": 1, "target": "2", "weight": -2.5}]
    }"#;
    let mut graph = DiGraph::<String>::pseudo().weighted();
    let mut importer = JsonImporter::<String, DefaultEdge>::new();
    importer.set_vertex_factory(str::to_owned);
    importer.import_str(&mut graph, input).unwrap();
    assert_eq!(graph.vertex_count(), 2);
    let one = String::from("1");
    let two = String::from("2");
    let edge = graph.get_edge(&one, &two).unwrap();
    assert_eq!(graph.edge_weight(edge), Some(-2.5));
}

#[test]
fn rejects_duplicate_and_incomplete_edges() {
    for input in [
        r#"{"nodes":[{"id":1},{"id":"1"}],"edges":[]}"#,
        r#"{"nodes":[{"id":1}],"edges":[{"target":1}]}"#,
        r#"{"nodes":[{"id":1}],"edges":[{"source":1}]}"#,
    ] {
        let mut graph = DiGraph::<String>::pseudo();
        let mut importer = JsonImporter::<String, DefaultEdge>::new();
        importer.set_vertex_factory(str::to_owned);
        assert!(importer.import_str(&mut graph, input).is_err());
    }
}

#[test]
fn exporter_matches_java_compact_layout_and_round_trips() {
    let mut graph = DiGraph::<usize>::simple().weighted();
    graph.add_vertices([1, 2]);
    let edge = graph.add_edge(&1, &2).unwrap().unwrap();
    graph.set_edge_weight(&edge, 100.0).unwrap();

    let mut exporter: JsonExporter<usize, DefaultEdge> =
        JsonExporter::with_vertex_id_provider(|vertex: &usize| vertex.to_string());
    let mut edge_ids = IntegerIdProvider::new();
    exporter.set_edge_id_provider(move |edge| edge_ids.id(edge));
    exporter.set_vertex_attribute_provider(|vertex| {
        let mut attributes = IndexMap::new();
        if *vertex == 1 {
            attributes.insert("label".into(), Attribute::from("one"));
            attributes.insert("active".into(), Attribute::from(true));
        }
        attributes
    });
    exporter.set_edge_attribute_provider(|_| {
        let mut attributes = IndexMap::new();
        attributes.insert(
            "weight".into(),
            Attribute::new("100.0", AttributeType::Double),
        );
        attributes
    });
    let output = exporter.export_to_string(&graph).unwrap();
    assert_eq!(
        output,
        "{\"creator\":\"JGraphT JSON Exporter\",\"version\":\"1\",\"nodes\":[{\"id\":\"1\",\"label\":\"one\",\"active\":true},{\"id\":\"2\"}],\"edges\":[{\"id\":\"1\",\"source\":\"1\",\"target\":\"2\",\"weight\":100.0}]}"
    );
}

#[test]
fn exporter_rejects_non_finite_numbers() {
    let mut graph = DiGraph::<usize>::simple();
    graph.add_vertex(1);
    let mut exporter: JsonExporter<usize, DefaultEdge> = JsonExporter::new();
    exporter.set_vertex_attribute_provider(|_| {
        IndexMap::from([("bad".into(), Attribute::new("NaN", AttributeType::Double))])
    });
    assert!(exporter.export_to_string(&graph).is_err());
}
