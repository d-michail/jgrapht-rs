// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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
use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, UGraph};
use jgrapht_io::dot::{DotExporter, DotImporter, is_valid_id};
use jgrapht_io::{Attribute, GraphExporter, GraphImporter};

#[test]
fn validates_dot_ids() {
    for valid in ["abc", "_abc123", "12.5", "-3", "\"two words\"", "<b>x</b>"] {
        assert!(is_valid_id(valid), "{valid}");
    }
    for invalid in ["", "two words", "1abc", "a-b"] {
        assert!(!is_valid_id(invalid), "{invalid}");
    }
}

#[test]
fn imports_chained_directed_edges_comments_and_ports() {
    let input = "digraph G { // comment\n a:out -> b -> c [label=\"chain\"]; /* x */ c -> c; }";
    let mut graph = DiGraph::<String>::pseudo();
    let mut importer = DotImporter::new();
    importer.set_vertex_factory(str::to_owned);
    importer.import_str(&mut graph, input).unwrap();
    assert_eq!(graph.vertex_count(), 3);
    assert_eq!(graph.edge_count(), 3);
    assert!(graph.contains_edge_between(&String::from("a"), &String::from("b")));
    assert!(graph.contains_edge_between(&String::from("c"), &String::from("c")));
}

#[test]
fn exporter_matches_basic_java_layout() {
    let mut graph = UGraph::<usize>::simple();
    graph.add_vertex(1);
    graph.add_vertex(2);
    graph.add_edge(&1, &2).unwrap();
    let mut exporter: DotExporter<usize, DefaultEdge> =
        DotExporter::with_vertex_id_provider(|vertex: &usize| vertex.to_string());
    exporter.set_vertex_attribute_provider(|vertex| {
        if *vertex == 1 {
            IndexMap::from([("label".into(), Attribute::from("one"))])
        } else {
            IndexMap::new()
        }
    });
    assert_eq!(
        exporter.export_to_string(&graph).unwrap(),
        "strict graph G {\n  1 [ label=\"one\" ];\n  2;\n  1 -- 2;\n}\n"
    );
}

#[test]
fn rejects_direction_mismatch() {
    let mut graph = UGraph::<String>::simple();
    let mut importer = DotImporter::new();
    importer.set_vertex_factory(str::to_owned);
    assert!(
        importer
            .import_str(&mut graph, "digraph { a -> b }")
            .is_err()
    );
}
