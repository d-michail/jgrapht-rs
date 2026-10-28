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
use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt, UGraph};
use jgrapht_io::gml::{GmlExporter, GmlImporter, GmlParameter};
use jgrapht_io::{
    Attribute, AttributeMap, AttributeType, GraphExporter, GraphImporter, IntegerIdProvider,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const BASIC: &str = r#"graph [
 comment "Sample Graph" directed 1
 node [ id 1 ] node [ id 2 ] node [ id 3 ] node [ ]
 edge [ source 1 target 2 ] edge [ source 2 target 3 ] edge [ source 3 target 1 ]
]"#;
const WEIGHTED: &str = r#"graph [
 node [ id 1 ] node [ id 2 ] node [ id 3 ]
 edge [ source 1 target 2 weight 2.0 ] edge [ source 3 target 1 weight 3.0 ]
]"#;

fn graph(weighted: bool) -> DiGraph<String> {
    let mut graph = DiGraph::<String>::pseudo().with_vertex_factory(|id| (id + 1).to_string());
    if weighted {
        graph = graph.weighted();
    }
    graph
}

fn import(input: &str, weighted: bool) -> Result<DiGraph<String>, jgrapht_io::IoError> {
    let mut graph = graph(weighted);
    GmlImporter::<String, DefaultEdge>::new().import_str(&mut graph, input)?;
    Ok(graph)
}

fn three_vertices(weighted: bool) -> UGraph<&'static str> {
    let mut graph = UGraph::<&str>::simple();
    if weighted {
        graph = graph.weighted();
    }
    graph.add_vertices(["v1", "v2", "v3"]);
    let e1 = graph.add_edge(&"v1", &"v2").unwrap().unwrap();
    let e2 = graph.add_edge(&"v3", &"v1").unwrap().unwrap();
    if weighted {
        graph.set_edge_weight(&e1, 2.0).unwrap();
        graph.set_edge_weight(&e2, 5.0).unwrap();
    }
    graph
}

fn exporter<E: Clone + Eq + std::hash::Hash + 'static>() -> GmlExporter<&'static str, E> {
    let mut exporter = GmlExporter::new();
    let mut ids = IntegerIdProvider::new();
    exporter.set_edge_id_provider(move |edge| ids.id(edge));
    exporter
}

mod importer_test {
    use super::*;

    #[test]
    fn undirected_unweighted() {
        let mut graph = UGraph::<String>::pseudo().with_vertex_factory(|id| (id + 1).to_string());
        GmlImporter::<String, DefaultEdge>::new()
            .import_str(&mut graph, BASIC)
            .unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (4, 3));
    }

    #[test]
    fn ignore_weights_undirected_unweighted() {
        let input = "graph [ node [ id 1 ] node [ id 2 ] edge [ source 1 target 2 weight 2.0 ] edge [ source 2.0 target 3 ] edge [ source 3.3 target 1 ] ]";
        let graph = import(input, false).unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (2, 1));
    }

    #[test]
    fn no_graph() {
        assert_eq!(import("GRAPH [ ]", false).unwrap().vertex_count(), 0);
        assert_eq!(import("graph [ ]", false).unwrap().vertex_count(), 0);
    }

    #[test]
    fn vertex_factory() {
        let mut graph = DiGraph::<String>::pseudo();
        let mut importer = GmlImporter::<String, DefaultEdge>::new();
        importer.set_vertex_factory(|id| (id + 100).to_string());
        importer
            .import_str(&mut graph, "graph [ node [ id 1 ] node [ id 8 ] ]")
            .unwrap();
        assert!(graph.contains_vertex(&"101".into()));
        assert!(graph.contains_vertex(&"108".into()));
    }

    #[test]
    fn ignore() {
        let input = "graph [ ignore [ node [ id 5 ] ] node [ id 1 ] node [ id 2 ] node [ id 3 ] node [ ] edge [ source 1 target 2 ] ignore [ edge [ source 5 target 1 ] ] edge [ source 2 target 3 ] edge [ source 3 target 1 ] ] node [ id 6 ]";
        let graph = import(input, false).unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (4, 3));
    }

    #[test]
    fn undirected_unweighted_wrong_order() {
        let input = "graph [ edge [ source 1 target 2 ] edge [ source 3 target 1 ] edge [ source 2 target 3 ] node [ id 1 ] node [ id 2 ] node [ id 3 ] node [ ] ]";
        let graph = import(input, false).unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (4, 3));
    }

    #[test]
    fn directed_pseudograph_unweighted() {
        let input = "graph [ node [ id 1 ] node [ id 2 ] node [ id 3 ] node [ ] edge [ source 1 target 2 ] edge [ source 1 target 2 ] edge [ source 1 target 2 ] edge [ source 2 target 3 ] edge [ source 3 target 1 ] edge [ source 1 target 1 ] edge [ source 2 target 2 ] ]";
        let graph = import(input, false).unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (4, 7));
    }

    fn assert_weighted(input: &str) {
        let graph = import(input, true).unwrap();
        let a = "1".to_owned();
        let b = "2".to_owned();
        let c = "3".to_owned();
        assert_eq!(
            graph.edge_weight(graph.edge_between(&a, &b).unwrap()),
            Some(2.0)
        );
        assert_eq!(
            graph.edge_weight(graph.edge_between(&c, &a).unwrap()),
            Some(3.0)
        );
    }

    #[test]
    fn directed_weighted() {
        assert_weighted(WEIGHTED);
    }
    #[test]
    fn directed_weighted_with_comments() {
        assert_weighted(&format!("# comment\n{WEIGHTED}"));
    }
    #[test]
    fn directed_weighted_single_line() {
        assert_weighted(
            "graph [ node [ id 1 ] node [ id 2 ] node [ id 3 ] edge [ source 1 target 2 weight 2.0 ] edge [ source 3 target 1 weight 3.0 ] ]",
        );
    }

    #[test]
    fn parser_error() {
        assert!(import("graph [ [ node ] ]", false).is_err());
    }

    #[test]
    fn export_import() {
        let mut source = DiGraph::<String>::pseudo().weighted();
        source.add_vertices(["1".into(), "2".into(), "3".into()]);
        for (a, b, w) in [("1", "2", 2.0), ("2", "3", 3.0), ("3", "3", 5.0)] {
            let e = source.add_edge(&a.into(), &b.into()).unwrap().unwrap();
            source.set_edge_weight(&e, w).unwrap();
        }
        let mut exporter: GmlExporter<String, DefaultEdge> = GmlExporter::new();
        exporter.set_parameter(GmlParameter::ExportEdgeWeights, true);
        let copy = import(&exporter.export_to_string(&source).unwrap(), true).unwrap();
        assert_eq!((copy.vertex_count(), copy.edge_count()), (3, 3));
    }

    #[test]
    fn not_supported_graph() {
        let mut graph = UGraph::<String>::simple().with_vertex_factory(|id| (id + 1).to_string());
        assert!(
            GmlImporter::<String, DefaultEdge>::new()
                .import_str(
                    &mut graph,
                    "graph [ node [ id 1 ] edge [ source 1 target 1 ] ]"
                )
                .is_err()
        );
    }

    #[test]
    fn node_id_bad_type() {
        assert!(import("graph [ node [ id \"1\" ] ]", false).is_err());
    }
    #[test]
    fn edge_source_bad_type() {
        assert!(
            import(
                "graph [ node [ id 1 ] node [ id 2 ] edge [ source \"1\" target 2 ] ]",
                false
            )
            .is_err()
        );
    }
    #[test]
    fn edge_target_bad_type() {
        assert!(
            import(
                "graph [ node [ id 1 ] node [ id 2 ] edge [ source 1 target \"2\" ] ]",
                false
            )
            .is_err()
        );
    }
    #[test]
    fn edge_weight_bad_type() {
        assert!(
            import(
                "graph [ node [ id 1 ] node [ id 2 ] edge [ source 1 target 2 weight \"2.0\" ] ]",
                false
            )
            .is_err()
        );
    }

    type Attrs = Arc<Mutex<HashMap<String, AttributeMap>>>;
    fn capture_import(input: &str) -> (DiGraph<String>, Attrs, Arc<Mutex<Vec<AttributeMap>>>) {
        let vertices: Attrs = Arc::new(Mutex::new(HashMap::new()));
        let edges = Arc::new(Mutex::new(Vec::new()));
        let vs = vertices.clone();
        let es = edges.clone();
        let mut importer = GmlImporter::<String, DefaultEdge>::new();
        importer.set_vertex_attribute_consumer(move |v, key, value| {
            vs.lock()
                .unwrap()
                .entry(v.clone())
                .or_default()
                .insert(key.into(), value.clone());
            Ok(())
        });
        importer.set_edge_attribute_consumer(move |_, key, value| {
            let mut maps = es.lock().unwrap();
            if maps.is_empty() {
                maps.push(AttributeMap::new());
            }
            maps.last_mut().unwrap().insert(key.into(), value.clone());
            Ok(())
        });
        let mut graph = graph(true);
        importer.import_str(&mut graph, input).unwrap();
        (graph, vertices, edges)
    }

    #[test]
    fn attributes_support() {
        let input = "graph [ edge [ source 1 target 2 label \"Edge 1-2\" name \"Name 12\" ] node [ id 1 label \"Node 1\" ] node [ id 2 label \"Node 2\" ] node [ label \"Node ?\" ] ]";
        let (_, vertices, edges) = capture_import(input);
        let vertices = vertices.lock().unwrap();
        assert_eq!(vertices["1"]["label"].value(), "Node 1");
        assert_eq!(edges.lock().unwrap()[0]["name"].value(), "Name 12");
    }

    #[test]
    fn custom_number_attributes_support() {
        let input = "graph [ edge [ source 1 target 2 frequency 6 customweight 7.5 ] node [ id 1 frequency 2 customweight 1.2 ] node [ id 2 frequency 3 customweight 2.1 ] node [ frequency 5 customweight 5.5 ] ]";
        let (_, vertices, edges) = capture_import(input);
        let vertices = vertices.lock().unwrap();
        assert_eq!(
            vertices["1"]["frequency"].attribute_type(),
            AttributeType::Int
        );
        assert_eq!(
            edges.lock().unwrap()[0]["customweight"].attribute_type(),
            AttributeType::Double
        );
    }

    #[test]
    fn nested_structure() {
        let input = "graph [ edge [ source 1 target 2 points [ x 1.0 y 2.0 ] deep [ one [ one 1 two 2 ] two [ one 1 two 2 ] ] ] node [ id 1 deep [ one [ one 1.0 two 2.0 ] two [ one \"1\" two \"2\" ] ] ] node [ id 2 points [ x 1.0 y 2.0 z 3.0 ] ] ]";
        let (_, vertices, edges) = capture_import(input);
        let vertices = vertices.lock().unwrap();
        assert_eq!(
            vertices["1"]["deep"].attribute_type(),
            AttributeType::Unknown
        );
        assert_eq!(
            edges.lock().unwrap()[0]["points"].value(),
            "[ x 1.0 y 2.0 ]"
        );
    }
}

mod exporter_test {
    use super::*;

    #[test]
    fn undirected() {
        let text = exporter::<DefaultEdge>()
            .export_to_string(&three_vertices(false))
            .unwrap();
        assert!(text.contains("\tdirected 0"));
        assert!(text.contains("\t\tid 1\n\t\tsource 1\n\t\ttarget 2"));
    }

    #[test]
    fn graphics_section() {
        let mut graph = UGraph::<&str>::simple();
        graph.add_vertices(["v1", "v2"]);
        graph.add_edge(&"v1", &"v2").unwrap();
        let mut exporter = exporter::<DefaultEdge>();
        exporter.set_parameter(GmlParameter::ExportCustomVertexGraphicsAttributes, true);
        exporter.set_vertex_graphics_attribute_provider(|_| {
            IndexMap::from([("fill".into(), Attribute::from("#FF0000"))])
        });
        assert!(
            exporter
                .export_to_string(&graph)
                .unwrap()
                .contains("graphics\n\t\t[\n\t\t\tfill \"#FF0000\"")
        );
    }

    #[test]
    fn unweighted_undirected() {
        let mut exporter = exporter::<DefaultEdge>();
        exporter.set_parameter(GmlParameter::ExportEdgeWeights, true);
        assert!(
            !exporter
                .export_to_string(&three_vertices(false))
                .unwrap()
                .contains("weight")
        );
    }

    #[test]
    fn directed() {
        let mut graph = DiGraph::<&str>::simple();
        graph.add_vertices(["v1", "v2"]);
        graph.add_edge(&"v1", &"v2").unwrap();
        assert!(
            exporter::<DefaultEdge>()
                .export_to_string(&graph)
                .unwrap()
                .contains("\tdirected 1")
        );
    }

    #[test]
    fn weighted_undirected() {
        let mut exporter = exporter::<DefaultEdge>();
        exporter.set_parameter(GmlParameter::ExportEdgeWeights, true);
        let text = exporter.export_to_string(&three_vertices(true)).unwrap();
        assert!(text.contains("weight 2.0"));
        assert!(text.contains("weight 5.0"));
    }

    #[test]
    fn weighted_undirected_with_edge_labels() {
        let mut exporter = exporter::<DefaultEdge>();
        exporter.set_parameter(GmlParameter::ExportEdgeWeights, true);
        exporter.set_parameter(GmlParameter::ExportEdgeLabels, true);
        assert!(
            exporter
                .export_to_string(&three_vertices(true))
                .unwrap()
                .contains("label \"(v1 : v2)\"")
        );
    }

    #[test]
    fn undirected_with_vertex_labels() {
        let mut exporter = exporter::<DefaultEdge>();
        exporter.set_parameter(GmlParameter::ExportVertexLabels, true);
        assert!(
            exporter
                .export_to_string(&three_vertices(true))
                .unwrap()
                .contains("label \"v1\"")
        );
    }

    #[test]
    fn parameters() {
        let mut exporter: GmlExporter<&str, DefaultEdge> = GmlExporter::new();
        for parameter in [
            GmlParameter::ExportVertexLabels,
            GmlParameter::ExportEdgeLabels,
            GmlParameter::ExportEdgeWeights,
        ] {
            assert!(!exporter.is_parameter(parameter));
            exporter.set_parameter(parameter, true);
            assert!(exporter.is_parameter(parameter));
            exporter.set_parameter(parameter, false);
            assert!(!exporter.is_parameter(parameter));
        }
    }

    #[test]
    fn custom_vertex_attributes_and_labels() {
        let graph = three_vertices(true);
        let first = *graph.edges().next().unwrap();
        let mut exporter = exporter::<DefaultEdge>();
        exporter.set_parameter(GmlParameter::ExportVertexLabels, true);
        exporter.set_parameter(GmlParameter::ExportCustomVertexAttributes, true);
        exporter.set_parameter(GmlParameter::ExportCustomEdgeAttributes, true);
        exporter.set_vertex_attribute_provider(|v| match *v {
            "v1" => IndexMap::from([("color".into(), Attribute::from("red"))]),
            "v3" => IndexMap::from([
                ("cost".into(), Attribute::from(5.5)),
                ("length".into(), Attribute::from(100i64)),
                ("visited".into(), Attribute::from(false)),
            ]),
            _ => IndexMap::new(),
        });
        exporter.set_edge_attribute_provider(move |e| {
            if *e == first {
                IndexMap::from([("name".into(), Attribute::from("first edge"))])
            } else {
                IndexMap::new()
            }
        });
        let text = exporter.export_to_string(&graph).unwrap();
        assert!(text.contains("color \"red\""));
        assert!(text.contains("cost 5.5"));
        assert!(text.contains("visited \"false\""));
        assert!(text.contains("name \"first edge\""));
    }

    fn reserved(kind: &str) -> bool {
        let mut graph = UGraph::<&str>::simple();
        graph.add_vertices(["v1", "v2"]);
        let edge = graph.add_edge(&"v1", &"v2").unwrap().unwrap();
        let mut exporter = exporter::<DefaultEdge>();
        if kind == "vertex" {
            exporter.set_parameter(GmlParameter::ExportCustomVertexAttributes, true);
            exporter.set_vertex_attribute_provider(|_| {
                IndexMap::from([("id".into(), Attribute::from("bad"))])
            });
        } else {
            let key = kind.to_owned();
            exporter.set_parameter(GmlParameter::ExportCustomEdgeAttributes, true);
            exporter.set_edge_attribute_provider(move |e| {
                if *e == edge {
                    IndexMap::from([(key.clone(), Attribute::from("bad"))])
                } else {
                    IndexMap::new()
                }
            });
        }
        exporter.export_to_string(&graph).is_err()
    }

    #[test]
    fn bad_custom_vertex_attributes() {
        assert!(reserved("vertex"));
    }
    #[test]
    fn bad_custom_edge_attribute_id() {
        assert!(reserved("id"));
    }
    #[test]
    fn bad_custom_edge_attribute_source() {
        assert!(reserved("source"));
    }
    #[test]
    fn bad_custom_edge_attribute_target() {
        assert!(reserved("target"));
    }
}
