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
use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphType, UGraph};
use jgrapht_io::dot::{DotExporter, DotImporter, DotSubgraph, is_valid_id};
use jgrapht_io::{Attribute, AttributeMap, AttributeType, GraphExporter, GraphImporter};
use std::sync::{Arc, Mutex};

fn directed(input: &str) -> Result<DiGraph<String>, jgrapht_io::IoError> {
    let mut graph = DiGraph::<String>::pseudo();
    let mut importer = DotImporter::new();
    importer.set_vertex_factory(str::to_owned);
    importer.import_str(&mut graph, input)?;
    Ok(graph)
}

fn undirected(input: &str) -> Result<UGraph<String>, jgrapht_io::IoError> {
    let mut graph = UGraph::<String>::multi();
    let mut importer = DotImporter::new();
    importer.set_vertex_factory(str::to_owned);
    importer.import_str(&mut graph, input)?;
    Ok(graph)
}

fn vertex_attributes(input: &str) -> IndexMap<String, AttributeMap> {
    let values = Arc::new(Mutex::new(IndexMap::<String, AttributeMap>::new()));
    let captured = values.clone();
    let mut graph = DiGraph::<String>::pseudo();
    let mut importer = DotImporter::new();
    importer.set_vertex_factory(str::to_owned);
    importer.set_vertex_attribute_consumer(move |vertex, key, value| {
        captured
            .lock()
            .unwrap()
            .entry(vertex.clone())
            .or_default()
            .insert(key.into(), value.clone());
        Ok(())
    });
    importer.import_str(&mut graph, input).unwrap();
    drop(importer);
    Arc::try_unwrap(values).unwrap().into_inner().unwrap()
}

mod exporter_test {
    use super::*;
    use jgrapht::GraphExt;

    fn sample(multiple: bool) -> UGraph<&'static str> {
        let mut graph = if multiple {
            UGraph::multi()
        } else {
            UGraph::simple()
        };
        for vertex in ["v1", "v2", "v3"] {
            graph.add_vertex(vertex);
        }
        graph.add_edge(&"v1", &"v2").unwrap();
        graph.add_edge(&"v3", &"v1").unwrap();
        graph
    }

    #[test]
    fn undirected() {
        let graph = sample(false);
        let mut exporter = DotExporter::new();
        exporter.set_vertex_attribute_provider(|vertex| match *vertex {
            "v1" => IndexMap::from([("label".into(), Attribute::from("a"))]),
            "v2" => IndexMap::from([("x".into(), Attribute::from("y"))]),
            _ => IndexMap::new(),
        });
        assert_eq!(
            exporter.export_to_string(&graph).unwrap(),
            "strict graph G {\n  1 [ label=\"a\" ];\n  2 [ x=\"y\" ];\n  3;\n  1 -- 2;\n  3 -- 1;\n}\n"
        );
    }

    #[test]
    fn valid_node_ids() {
        for id in ["-9.78", "-.5", "12", "a", "abc_78", "\"--34asdf\""] {
            let mut graph = DiGraph::<String>::simple();
            graph.add_vertex(id.into());
            assert!(
                DotExporter::<String, DefaultEdge>::with_vertex_id_provider(Clone::clone)
                    .export_to_string(&graph)
                    .is_ok()
            );
        }
        for id in ["2test", "--4", "foo-bar", "", "t:32"] {
            let mut graph = DiGraph::<String>::simple();
            graph.add_vertex(id.into());
            assert!(
                DotExporter::<String, DefaultEdge>::with_vertex_id_provider(Clone::clone)
                    .export_to_string(&graph)
                    .is_err()
            );
        }
    }

    #[test]
    fn quoted_node_ids() {
        let mut graph = DiGraph::<String>::simple();
        graph.add_vertex("\"abc\"".into());
        let mut exporter = DotExporter::with_vertex_id_provider(Clone::clone);
        exporter.set_vertex_attribute_provider(|v| {
            IndexMap::from([("label".into(), Attribute::from(v.clone()))])
        });
        assert!(
            exporter
                .export_to_string(&graph)
                .unwrap()
                .contains("label=\"\\\"abc\\\"\"")
        );
    }

    #[test]
    fn node_html_label_from_attribute() {
        let mut graph = DiGraph::<String>::simple();
        graph.add_vertex("v".into());
        let mut exporter = DotExporter::with_vertex_id_provider(Clone::clone);
        exporter.set_vertex_attribute_provider(|_| {
            IndexMap::from([(
                "label".into(),
                Attribute::new("<b>html label</b>", AttributeType::Html),
            )])
        });
        assert!(
            exporter
                .export_to_string(&graph)
                .unwrap()
                .contains("label=<<b>html label</b>>")
        );
    }

    #[test]
    fn different_graph_id() {
        let graph = UGraph::<String>::simple();
        let mut exporter = DotExporter::<String, DefaultEdge>::new();
        exporter.set_graph_id_provider(|| "MyGraph".into());
        assert_eq!(
            exporter.export_to_string(&graph).unwrap(),
            "strict graph MyGraph {\n}\n"
        );
    }

    #[test]
    fn with_subgraph() {
        let graph = sample(false);
        let edges: Vec<_> = graph.edges_cloned().collect();
        let mut subgraph = DotSubgraph::new(["v1", "v2"], [edges[0]]);
        subgraph.graph_attributes.insert(
            "pencolor".into(),
            Attribute::new("transparent", AttributeType::Identifier),
        );
        subgraph
            .cluster_attributes
            .insert("label".into(), Attribute::from(""));
        let mut exporter = DotExporter::new();
        exporter.set_subgraph_provider(move || IndexMap::from([("subg".into(), subgraph.clone())]));
        let output = exporter.export_to_string(&graph).unwrap();
        assert!(output.contains("  subgraph subg {\n    subg [ label=\"\" ];\n    pencolor=transparent;\n    1;\n    2;\n    1 -- 2;\n  }"));
    }
}

mod utils_test {
    use super::*;
    #[test]
    fn accepts_id_with_digits() {
        assert!(is_valid_id("abc123"));
    }
    #[test]
    fn rejects_id_that_starts_with_digit() {
        assert!(!is_valid_id("2test"));
    }
    #[test]
    fn accepts_id_that_starts_with_underscore() {
        assert!(is_valid_id("_test"));
    }
}

mod importer1_test {
    use super::*;

    #[test]
    fn import_id() {
        let seen = Arc::new(Mutex::new(None));
        let copy = seen.clone();
        let mut graph = UGraph::<String>::multi();
        let mut importer = DotImporter::new();
        importer.set_vertex_factory(str::to_owned);
        importer.set_graph_attribute_consumer(move |k, v| {
            if k == "ID" {
                *copy.lock().unwrap() = Some(v.value().to_owned());
            }
            Ok(())
        });
        importer
            .import_str(&mut graph, "strict graph MyGraph {}")
            .unwrap();
        assert_eq!(*seen.lock().unwrap(), Some("MyGraph".into()));
    }
    #[test]
    fn import_wrong_id() {
        assert!(undirected("graph 2test {}").is_err());
    }
    #[test]
    fn invalid_header() {
        for input in [
            " {}",
            "strict {}",
            "id {}",
            "strict id {}",
            "id strict {}",
            "id strict graph {}",
            "graph strict id {}",
        ] {
            assert!(undirected(input).is_err(), "{input}");
        }
    }
    #[test]
    fn import_only_graph_keyword() {
        assert_eq!(undirected("graph {}").unwrap().vertex_count(), 0);
    }
    #[test]
    fn import_no_id() {
        assert_eq!(undirected("strict graph {}").unwrap().vertex_count(), 0);
    }
    #[test]
    fn undirected_with_labels() {
        let graph = undirected("graph G { 1 [label=abc123]; 2 [label=fred]; 1 -- 2 }").unwrap();
        assert!(graph.contains_edge_between(&"1".into(), &"2".into()));
    }
    #[test]
    fn directed_no_labels() {
        let g = directed("digraph graphname { a -> b -> c; b -> d }").unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (4, 3));
    }
    #[test]
    fn directed_same_labels() {
        let a = vertex_attributes(
            "digraph sample { a->b; b->c; a[label=Test]; b[label=Test]; c[label=Test] }",
        );
        assert!(a.values().all(|m| m["label"].value() == "Test"));
    }
    #[test]
    fn multi_links_undirected() {
        assert_eq!(
            undirected("graph { 1--2 [label=friend]; 1--2 }")
                .unwrap()
                .edge_count(),
            2
        );
    }
    #[test]
    fn export_import_loop() {
        let start = directed("digraph { 0->1; 1->2; 1->3 }").unwrap();
        let text = DotExporter::with_vertex_id_provider(Clone::clone)
            .export_to_string(&start)
            .unwrap();
        let result = directed(&text).unwrap();
        assert_eq!((result.vertex_count(), result.edge_count()), (4, 3));
    }
    #[test]
    fn dash_label_vertex() {
        assert_eq!(
            vertex_attributes(
                "digraph { a[label=\"------this------contains-------dashes------\"] }"
            )["a"]["label"]
                .value(),
            "------this------contains-------dashes------"
        );
    }
    #[test]
    fn attributes_with_no_quotes() {
        let attrs = vertex_attributes(
            "digraph { 1[label=bob foo=bar]; 2[label=fred]; 1->2[label=friend foo=wibble] }",
        );
        assert_eq!(attrs["1"]["foo"].value(), "bar");
    }
    #[test]
    fn empty_string() {
        assert!(directed("").is_err());
    }
    #[test]
    fn garbage_string_enough_lines() {
        assert!(directed("jsfhg kjdsf\nfdsgfdsgfd\ngfdgfdsgfdsg\n").is_err());
    }
    #[test]
    fn garbage_string_invalid_first_line() {
        assert!(directed("jsfhgkjdsfhgkfds\nfdsgfdsgfd\n").is_err());
    }
    #[test]
    fn garbage_string_not_enough_lines() {
        assert!(directed("jsfhgkjdsfhgkfds\n").is_err());
    }
    #[test]
    fn attributes_with_no_values() {
        assert!(directed("digraph { 1[label=bob foo]; }").is_err());
    }
    #[test]
    fn updating_vertex() {
        assert_eq!(
            vertex_attributes("digraph { a->b; a[foo=bar] }")["a"]["foo"].value(),
            "bar"
        );
    }
    #[test]
    fn parameters_with_semicolons() {
        assert_eq!(vertex_attributes("digraph { 1[label=\"this label; contains a semi colon\"] }")["1"]["label"].value(), "this label; contains a semi colon");
    }
    #[test]
    fn labels_with_escaped_semicolons() {
        assert_eq!(vertex_attributes("digraph { node[label=\"this \\\"label; \\\"contains an escaped semi colon\"]; node0 }")["node0"]["label"].value(), "this \"label; \"contains an escaped semi colon");
    }
    #[test]
    fn no_line_end_between_nodes() {
        assert_eq!(
            directed("digraph { 1[label=x]; 2[label=y] }")
                .unwrap()
                .vertex_count(),
            2
        );
    }
    #[test]
    fn with_reader() {
        assert_eq!(
            undirected("graph { 1[label=abc]; 2[label=fred]; 1--2 }")
                .unwrap()
                .edge_count(),
            1
        );
    }
}

mod importer2_test {
    use super::*;

    #[test]
    fn dot1() {
        assert_eq!(
            directed("digraph { a--b--c; k:1--q:a:3--d:3 }")
                .unwrap()
                .edge_count(),
            4
        );
    }
    #[test]
    fn dot2() {
        let g = directed("digraph G { subgraph cluster0 { a0->a1->a2->a3 } subgraph cluster1 { b0->b1->b2->b3 } start->a0; start->b0; a1->b3; b2->a3; a3->a0; a3->end; b3->end }").unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (10, 13));
    }
    #[test]
    fn dot3() {
        let g = directed("digraph { subgraph { a0->a1 } -> subgraph { b0->b1 } }").unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (4, 6));
    }
    #[test]
    fn dot4() {
        let g = directed("digraph { subgraph { a0->a1; subgraph { a00->a11 } } -> subgraph { b0->b1; subgraph { b00->b11 } } }").unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (8, 20));
    }
    #[test]
    fn dot5() {
        let g = directed("digraph { a--b--c; k:1--q:a:3--d:3 }").unwrap();
        assert!(g.contains_vertex(&"q".into()));
    }
    #[test]
    fn node_subgraph_edges() {
        let g = directed("digraph { a0->{a00->a11}->b0->{b00->b11} }").unwrap();
        assert_eq!(g.edge_count(), 8);
    }
    #[test]
    fn node_subgraph_edges_undirected() {
        let g = directed("digraph { a0->{a00--a11}->b0--{b00--b11} }").unwrap();
        assert_eq!(g.edge_count(), 8);
    }
    #[test]
    fn nested_subgraphs() {
        let g = directed("digraph { a0->{{{{a00->a11}}}}->b0->{{b00->b11}} }").unwrap();
        assert_eq!(g.edge_count(), 8);
    }
    #[test]
    fn edge_attributes() {
        let values = Arc::new(Mutex::new(Vec::<AttributeMap>::new()));
        let copy = values.clone();
        let mut graph = DiGraph::<String>::pseudo();
        let mut importer = DotImporter::new();
        importer.set_vertex_factory(str::to_owned);
        importer.set_edge_attribute_consumer(move |_, k, v| {
            let mut lock = copy.lock().unwrap();
            if k == "weight" {
                lock.push(IndexMap::from([(k.into(), v.clone())]));
            }
            Ok(())
        });
        importer.import_str(&mut graph, "digraph { edge[weight=5.0]; a0->a1; subgraph { edge[weight=2.0]; a2->a3 }; a4->a5[weight=15.0] }").unwrap();
        let got: Vec<_> = values
            .lock()
            .unwrap()
            .iter()
            .map(|m| m["weight"].value().to_owned())
            .collect();
        assert_eq!(got, ["5.0", "2.0", "15.0"]);
    }
    #[test]
    fn comments() {
        let g = directed("digraph { // first\n a0 -> /* middle */ a1; # last\n }").unwrap();
        assert_eq!(g.edge_count(), 1);
    }
    #[test]
    fn node_attributes() {
        let attrs = vertex_attributes(
            "digraph { node[color=gray]; a0->a1; subgraph { node[color=black]; a2->a3 }; a4[color=white]; a4->a5 }",
        );
        assert_eq!(
            (
                attrs["a0"]["color"].value(),
                attrs["a2"]["color"].value(),
                attrs["a4"]["color"].value(),
                attrs["a5"]["color"].value()
            ),
            ("gray", "black", "white", "gray")
        );
    }
    #[test]
    fn unescape() {
        let attrs = vertex_attributes(
            "digraph { a0[name=myname]; a1[name=\"name with ; semicolon\"]; a2[name=\"name with \\\"internal\\\" quotes\"]; a3[name=<<h2>name ✅</h2>>]; a4[name=\"\"] }",
        );
        assert_eq!(attrs["a2"]["name"].value(), "name with \"internal\" quotes");
        assert_eq!(attrs["a3"]["name"].value(), "<h2>name ✅</h2>");
        assert_eq!(attrs["a4"]["name"].value(), "");
    }
    #[test]
    fn unescape2() {
        let a = vertex_attributes("digraph { a[label=\" /\\n \"]; b[label=\"\\\"Test\\\"\"] }");
        assert_eq!(a["a"]["label"].value(), " /\\n ");
        assert_eq!(a["b"]["label"].value(), "\"Test\"");
    }
    #[test]
    fn non_valid_html_string() {
        assert!(directed("digraph { a0[name=< >> >] }").is_err());
    }
    #[test]
    fn valid_html_string() {
        assert_eq!(
            vertex_attributes("digraph { a0[name=<<h1/>>] }")["a0"]["name"].value(),
            "<h1/>"
        );
    }
    #[test]
    fn loop_error() {
        let mut graph = DiGraph::<String>::multi();
        let mut importer = DotImporter::new();
        importer.set_vertex_factory(str::to_owned);
        assert!(
            importer
                .import_str(&mut graph, "digraph { a0->a0 }")
                .is_err()
        );
    }
    #[test]
    fn example_dot1() {
        assert_eq!(directed("digraph G { subgraph cluster0 { node[style=filled,color=white]; style=filled; color=lightgrey; a0->a1->a2->a3; label=\"process #1\" } start->a0 }").unwrap().edge_count(), 4);
    }
    #[test]
    fn shapes1() {
        let a = vertex_attributes(
            "digraph structs { node[shape=plaintext]; struct1[label=<<TABLE><TR><TD>line 1</TD></TR></TABLE>>] }",
        );
        assert!(a["struct1"]["label"].value().contains("TABLE"));
    }
    #[test]
    fn empty_list() {
        let g = directed("digraph g { name=TEST; graph[] anyNodeWithAttributes[color=red]; anyNodeWithoutAttributes; anyNodeWithEmptyAttributes[] }").unwrap();
        assert_eq!(g.vertex_count(), 3);
    }
    #[test]
    fn create_vertices_with_attributes() {
        let mut graph = DiGraph::<String>::pseudo();
        let mut importer = DotImporter::new();
        importer.set_vertex_with_attributes_factory(|id, attrs| {
            format!("{id}-{}", attrs["color"].value())
        });
        importer
            .import_str(
                &mut graph,
                "digraph { a0[color=gray]; a1[color=green]; a0->a1; a2[color=white] }",
            )
            .unwrap();
        assert!(graph.contains_vertex(&"a2-white".into()));
    }
    #[test]
    fn create_edges_with_attributes() {
        let mut graph = DiGraph::<String, String>::without_edge_factory(
            GraphType::pseudograph().directed(true),
        );
        let mut importer = DotImporter::new();
        importer.set_vertex_factory(str::to_owned);
        importer.set_edge_with_attributes_factory(|attrs| attrs["label"].value().to_owned());
        importer
            .import_str(&mut graph, "digraph { a0->a1[label=e1]; a1->a2[label=e2] }")
            .unwrap();
        assert!(graph.contains_edge(&"e1".into()));
        assert!(graph.contains_edge(&"e2".into()));
    }
}
