use indexmap::IndexMap;
use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt, GraphType, UGraph};
use jgrapht_io::json::{JsonExporter, JsonImporter};
use jgrapht_io::{
    Attribute, AttributeMap, AttributeType, GraphExporter, GraphImporter, IntegerIdProvider,
};
use std::sync::{Arc, Mutex};

const BASIC: &str = r#"{
  "nodes":[{"id":"1"},{"id":"2"},{"id":"3"},{"id":"4"}],
  "edges":[{"source":"1","target":"2"},{"source":"1","target":"3"}]
}"#;
const WEIGHTED: &str = r#"{
  "nodes":[{"id":"1"},{"id":"2"},{"id":"3"},{"id":"4"}],
  "edges":[{"source":"1","target":"2","weight":2.0},{"source":"1","target":"3","weight":3.0},{"source":"2","target":"3"}]
}"#;

fn string_graph(weighted: bool) -> DiGraph<String> {
    let mut graph = DiGraph::<String>::pseudo().with_vertex_factory(|id| (id + 1).to_string());
    if weighted {
        graph = graph.weighted();
    }
    graph
}

fn import_strings(input: &str, weighted: bool) -> DiGraph<String> {
    let mut graph = string_graph(weighted);
    JsonImporter::<String, DefaultEdge>::new()
        .import_str(&mut graph, input)
        .unwrap();
    graph
}

fn four_vertex_graph() -> DiGraph<usize> {
    let mut graph = DiGraph::<usize>::simple();
    graph.add_vertices(1..=4);
    for edge in [(1, 2), (2, 3), (3, 4), (1, 4)] {
        graph.add_edge(&edge.0, &edge.1).unwrap();
    }
    graph
}

fn configured_exporter() -> JsonExporter<usize, DefaultEdge> {
    let mut exporter = JsonExporter::with_vertex_id_provider(|vertex: &usize| vertex.to_string());
    let mut ids = IntegerIdProvider::new();
    exporter.set_edge_id_provider(move |edge| ids.id(edge));
    exporter
}

mod importer_test {
    use super::*;

    #[test]
    fn undirected_unweighted() {
        let mut graph = UGraph::<String>::pseudo().with_vertex_factory(|id| (id + 1).to_string());
        JsonImporter::<String, DefaultEdge>::new()
            .import_str(&mut graph, BASIC)
            .unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (4, 2));
        assert!(graph.contains_edge_between(&"1".into(), &"2".into()));
    }

    #[test]
    fn vertex_factory() {
        let mut graph = UGraph::<String>::pseudo();
        let mut importer = JsonImporter::<String, DefaultEdge>::new();
        importer.set_vertex_factory(|id| format!("node{id}"));
        importer.import_str(&mut graph, BASIC).unwrap();
        assert!(graph.contains_edge_between(&"node1".into(), &"node3".into()));
    }

    #[test]
    fn mixed_string_and_integer_ids() {
        let graph = import_strings(
            r#"{"nodes":[{"id":1},{"id":"2"},{"id":"3"},{"id":4}],"edges":[{"source":1,"target":"2"},{"source":1,"target":3}]}"#,
            false,
        );
        assert_eq!((graph.vertex_count(), graph.edge_count()), (4, 2));
    }

    #[test]
    fn duplicate_node_ids() {
        let input = r#"{"nodes":[{"id":1},{"id":"2"},{"id":1}],"edges":[]}"#;
        let mut graph = string_graph(false);
        assert!(
            JsonImporter::<String, DefaultEdge>::new()
                .import_str(&mut graph, input)
                .is_err()
        );
    }

    #[test]
    fn missing_source_on_edge() {
        let input = r#"{"nodes":[{"id":1},{"id":2}],"edges":[{"target":2}]}"#;
        let mut graph = string_graph(false);
        assert!(
            JsonImporter::<String, DefaultEdge>::new()
                .import_str(&mut graph, input)
                .is_err()
        );
    }

    #[test]
    fn missing_target_on_edge() {
        let input = r#"{"nodes":[{"id":1},{"id":2}],"edges":[{"source":1}]}"#;
        let mut graph = string_graph(false);
        assert!(
            JsonImporter::<String, DefaultEdge>::new()
                .import_str(&mut graph, input)
                .is_err()
        );
    }

    #[test]
    fn weights_on_weighted() {
        let graph = import_strings(WEIGHTED, true);
        for (source, target, weight) in [("1", "2", 2.0), ("1", "3", 3.0), ("2", "3", 1.0)] {
            let source = source.to_owned();
            let target = target.to_owned();
            let edge = graph.get_edge(&source, &target).unwrap();
            assert_eq!(graph.edge_weight(edge), Some(weight));
        }
    }

    #[test]
    fn weights_on_unweighted() {
        let graph = import_strings(WEIGHTED, false);
        assert_eq!(
            graph.edge_weight(graph.get_edge(&"1".into(), &"2".into()).unwrap()),
            Some(1.0)
        );
    }

    #[test]
    fn node_attributes() {
        let input = r#"{"nodes":[{"id":"1","label":"Label","int":4,"double":0.5,"boolean":true,"boolean1":false,"novalue":null}],"edges":null}"#;
        let captured = Arc::new(Mutex::new(AttributeMap::new()));
        let sink = captured.clone();
        let mut importer = JsonImporter::<String, DefaultEdge>::new();
        importer.set_vertex_with_attributes_factory(move |id, attrs| {
            *sink.lock().unwrap() = attrs.clone();
            id.to_owned()
        });
        let mut graph = UGraph::<String>::pseudo();
        importer.import_str(&mut graph, input).unwrap();
        let attrs = captured.lock().unwrap();
        assert_eq!(attrs["label"].attribute_type(), AttributeType::String);
        assert_eq!(attrs["int"].attribute_type(), AttributeType::Int);
        assert_eq!(attrs["double"].attribute_type(), AttributeType::Double);
        assert_eq!(attrs["boolean"].attribute_type(), AttributeType::Boolean);
        assert_eq!(attrs["novalue"].attribute_type(), AttributeType::Null);
    }

    #[test]
    fn edge_attributes() {
        let input = r#"{"nodes":[{"id":"1"}],"edges":[{"source":"1","target":"1","label":"Label","int":4,"double":0.5,"boolean":true,"boolean1":false,"novalue":null}]}"#;
        let captured = Arc::new(Mutex::new(AttributeMap::new()));
        let sink = captured.clone();
        let mut importer = JsonImporter::<String, DefaultEdge>::new();
        importer.set_edge_with_attributes_factory(move |attrs| {
            *sink.lock().unwrap() = attrs.clone();
            DefaultEdge::from_id(99)
        });
        let mut graph = UGraph::<String>::pseudo().with_vertex_factory(|id| (id + 1).to_string());
        importer.import_str(&mut graph, input).unwrap();
        let attrs = captured.lock().unwrap();
        assert_eq!(attrs["label"].value(), "Label");
        assert_eq!(attrs["int"].attribute_type(), AttributeType::Int);
        assert_eq!(attrs["novalue"].attribute_type(), AttributeType::Null);
    }

    #[test]
    fn nested_attributes() {
        let input = r#"{"nodes":[{"id":"1","custom":{"pi":3.14}},{"id":"2","array":[{"obj":3.14}]}],"edges":[{"source":"1","target":"2","array":[{"key1":1},{"key2":2}]},{"source":"2","target":"1","obj":{"key1":[{"key1":1},{"key2":2}]}}]}"#;
        let values = Arc::new(Mutex::new(Vec::new()));
        let vertex_values = values.clone();
        let edge_values = values.clone();
        let mut importer = JsonImporter::<String, DefaultEdge>::new();
        importer.set_vertex_with_attributes_factory(move |id, attrs| {
            vertex_values
                .lock()
                .unwrap()
                .extend(attrs.values().map(|a| a.value().to_owned()));
            id.to_owned()
        });
        importer.set_edge_with_attributes_factory(move |attrs| {
            edge_values
                .lock()
                .unwrap()
                .extend(attrs.values().map(|a| a.value().to_owned()));
            DefaultEdge::from_id(edge_values.lock().unwrap().len() as u64)
        });
        let mut graph = DiGraph::<String>::pseudo();
        importer.import_str(&mut graph, input).unwrap();
        let values = values.lock().unwrap();
        assert!(values.contains(&"{\"pi\":3.14}".into()));
        assert!(values.contains(&"[{\"key1\":1},{\"key2\":2}]".into()));
    }

    #[test]
    fn singletons() {
        let graph = import_strings(
            r#"{"nodes":[{"id":"1"},{"id":"2"},{},{}],"edges":[{"source":"1","target":"2"}]}"#,
            false,
        );
        assert_eq!((graph.vertex_count(), graph.edge_count()), (4, 1));
    }

    #[test]
    fn negative_integer_weights() {
        let graph = import_strings(
            r#"{"nodes":[{"id":"1"},{"id":"2"}],"edges":[{"source":"1","target":"2","weight":-2}]}"#,
            true,
        );
        let source = "1".to_owned();
        let target = "2".to_owned();
        assert_eq!(
            graph.edge_weight(graph.get_edge(&source, &target).unwrap()),
            Some(-2.0)
        );
    }

    #[test]
    fn create_vertices_with_attributes() {
        let input = r#"{"nodes":[{"id":"a0","color":"gray"},{"id":"a1","color":"green"},{"id":"a2","color":"white"}],"edges":[{"source":"a0","target":"a1"},{"source":"a0","target":"a2"}]}"#;
        let mut importer = JsonImporter::<String, DefaultEdge>::new();
        importer.set_vertex_with_attributes_factory(|id, attrs| {
            format!("{id}-{}", attrs["color"].value())
        });
        let mut graph = DiGraph::<String>::pseudo();
        importer.import_str(&mut graph, input).unwrap();
        assert!(graph.contains_vertex(&"a0-gray".into()));
        assert!(graph.contains_vertex(&"a2-white".into()));
    }

    #[test]
    fn create_edges_with_attributes() {
        let input = r#"{"nodes":[{"id":"a0"},{"id":"a1"},{"id":"a2"}],"edges":[{"source":"a0","target":"a1","label":"e1"},{"source":"a0","target":"a2","label":"e2"}]}"#;
        let mut importer = JsonImporter::<String, String>::new();
        importer.set_vertex_factory(str::to_owned);
        importer.set_edge_with_attributes_factory(|attrs| attrs["label"].value().to_owned());
        let mut graph = DiGraph::<String, String>::without_edge_factory(GraphType::pseudograph());
        importer.import_str(&mut graph, input).unwrap();
        assert!(graph.contains_edge(&"e1".into()));
        assert!(graph.contains_edge(&"e2".into()));
    }

    #[test]
    fn other_collection_names() {
        let input = r#"{"vertices":[{"id":"1"},{"id":"2"},{"id":"3"},{"id":"4"}],"rels":[{"source":"1","target":"2"},{"source":"1","target":"3"}]}"#;
        let mut importer = JsonImporter::<String, DefaultEdge>::new();
        importer.set_vertices_collection_name("vertices");
        importer.set_edges_collection_name("rels");
        let mut graph = string_graph(false);
        importer.import_str(&mut graph, input).unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (4, 2));
    }
}

mod exporter_test {
    use super::*;

    #[test]
    fn basic() {
        let expected = "{\"creator\":\"JGraphT JSON Exporter\",\"version\":\"1\",\"nodes\":[{\"id\":\"1\"},{\"id\":\"2\"},{\"id\":\"3\"},{\"id\":\"4\"}],\"edges\":[{\"id\":\"1\",\"source\":\"1\",\"target\":\"2\"},{\"id\":\"2\",\"source\":\"2\",\"target\":\"3\"},{\"id\":\"3\",\"source\":\"3\",\"target\":\"4\"},{\"id\":\"4\",\"source\":\"1\",\"target\":\"4\"}]}";
        assert_eq!(
            configured_exporter()
                .export_to_string(&four_vertex_graph())
                .unwrap(),
            expected
        );
    }

    #[test]
    fn weighted_with_weights_and_color() {
        let mut graph = DiGraph::<usize>::simple().weighted();
        graph.add_vertices(1..=3);
        let e12 = graph.add_edge(&1, &2).unwrap().unwrap();
        let e13 = graph.add_edge(&1, &3).unwrap().unwrap();
        let e23 = graph.add_edge(&2, &3).unwrap().unwrap();
        graph.set_edge_weight(&e23, 100.0).unwrap();
        let mut exporter = configured_exporter();
        exporter.set_vertex_attribute_provider(|v| match v {
            1 => IndexMap::from([
                ("color".into(), Attribute::from("yellow")),
                ("label".into(), Attribute::from("V1")),
            ]),
            2 => IndexMap::from([
                ("color".into(), Attribute::from("red")),
                ("label".into(), Attribute::from("V2")),
            ]),
            _ => IndexMap::from([("label".into(), Attribute::from("V3"))]),
        });
        exporter.set_edge_attribute_provider(move |e| {
            let (color, label, weight) = if *e == e12 {
                ("what?", "e12", 1.0)
            } else if *e == e13 {
                ("I have no color!", "e13", 1.0)
            } else {
                ("I have no color!", "e13", 100.0)
            };
            IndexMap::from([
                ("color".into(), Attribute::from(color)),
                ("label".into(), Attribute::from(label)),
                ("weight".into(), Attribute::from(weight)),
            ])
        });
        let text = exporter.export_to_string(&graph).unwrap();
        assert!(text.contains("\"color\":\"yellow\",\"label\":\"V1\""));
        assert!(text.contains("\"weight\":100.0"));
    }

    #[test]
    fn attribute_types() {
        let mut graph = DiGraph::<usize>::simple().weighted();
        graph.add_vertices([1, 2]);
        let edge = graph.add_edge(&1, &2).unwrap().unwrap();
        graph.set_edge_weight(&edge, 100.0).unwrap();
        let mut exporter = configured_exporter();
        exporter.set_vertex_attribute_provider(|v| {
            if *v == 1 {
                IndexMap::from([
                    ("stringAttribute".into(), Attribute::from("yellow")),
                    ("doubleAttribute".into(), Attribute::from(3.4f64)),
                    ("intAttribute".into(), Attribute::from(3i32)),
                    ("floatAttribute".into(), Attribute::from(3.4f32)),
                    ("longAttribute".into(), Attribute::from(3i64)),
                    ("booleanAttribute".into(), Attribute::from(true)),
                ])
            } else {
                IndexMap::new()
            }
        });
        let text = exporter.export_to_string(&graph).unwrap();
        assert!(text.contains("\"doubleAttribute\":3.4"));
        assert!(text.contains("\"booleanAttribute\":true"));
    }

    #[test]
    fn not_allowed_nan_double() {
        let mut graph = DiGraph::<usize>::simple();
        graph.add_vertex(1);
        let mut exporter: JsonExporter<usize, DefaultEdge> = JsonExporter::new();
        exporter.set_vertex_attribute_provider(|_| {
            IndexMap::from([("NaNAttribute".into(), Attribute::from(f64::NAN))])
        });
        assert!(exporter.export_to_string(&graph).is_err());
    }

    fn round_trip(difficult: bool) {
        let difficult_id = "I have \"\" in my id";
        let ids = if difficult {
            vec!["1", difficult_id, "3", "4", "5"]
        } else {
            vec!["1", "2", "3", "4", "5"]
        };
        let mut graph = DiGraph::<String>::pseudo();
        graph.add_vertices(ids.iter().map(|s| (*s).to_owned()));
        for (s, t) in [
            ("1", ids[1]),
            ("1", "3"),
            ("1", "4"),
            ("1", "4"),
            ("1", "4"),
            ("4", "4"),
        ] {
            graph.add_edge(&s.into(), &t.into()).unwrap();
        }
        let mut exporter = JsonExporter::with_vertex_id_provider(|v: &String| v.clone());
        let text = exporter.export_to_string(&graph).unwrap();
        let mut copy = DiGraph::<String>::pseudo();
        let mut importer = JsonImporter::<String, DefaultEdge>::new();
        importer.set_vertex_factory(str::to_owned);
        importer.import_str(&mut copy, &text).unwrap();
        assert_eq!((copy.vertex_count(), copy.edge_count()), (5, 6));
        assert_eq!(copy.edges_between(&"1".into(), &"4".into()).count(), 3);
        if difficult {
            assert!(copy.contains_vertex(&difficult_id.into()));
        }
    }

    #[test]
    fn export_and_import() {
        round_trip(false);
    }
    #[test]
    fn export_and_import_with_escape() {
        round_trip(true);
    }

    #[test]
    fn other_collection_names() {
        let mut exporter = configured_exporter();
        exporter.set_vertices_collection_name("vertices");
        exporter.set_edges_collection_name("links");
        let text = exporter.export_to_string(&four_vertex_graph()).unwrap();
        assert!(text.contains("\"vertices\":["));
        assert!(text.contains("\"links\":["));
    }
}
