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
use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt, UGraph};
use jgrapht_io::graphml::{
    AttributeCategory, GraphMlEventDrivenImporter, GraphMlExporter, GraphMlImporter,
    SimpleGraphMlEdgeListImporter, SimpleGraphMlEventDrivenImporter, SimpleGraphMlImporter,
};
use jgrapht_io::{
    Attribute, AttributeMap, AttributeType, EventDrivenImporter, GraphExporter, GraphImporter,
    ImportEvent,
};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

const BASIC: &str = r#"<graphml><graph edgedefault="undirected"><node id="1"/><node id="2"/><node id="3"/><edge source="1" target="2"/><edge source="3" target="1"/><edge source="2" target="3"/></graph></graphml>"#;
const WRONG_ORDER: &str = r#"<graphml><graph edgedefault="undirected"><edge source="2" target="3"/><node id="1"/><node id="2"/><node id="3"/><edge source="1" target="2"/><edge source="3" target="1"/></graph></graphml>"#;
const ATTRS: &str = r#"<graphml><key id="c" for="node" attr.name="color" attr.type="string"><default>yellow</default></key><key id="w" for="edge" attr.name="weight" attr.type="double"><default>3.0</default></key><graph edgedefault="undirected"><node id="n0"><data key="c">green</data></node><node id="n1"/><node id="n2"><data key="c">blue</data></node><edge id="e0" source="n0" target="n2"><data key="w">2.0</data></edge><edge source="n0" target="n1"><data key="w">1.0</data></edge><edge source="n1" target="n2"/></graph></graphml>"#;

fn import_u(input: &str) -> Result<UGraph<String>, jgrapht_io::IoError> {
    let mut g = UGraph::<String>::pseudo();
    let mut i = GraphMlImporter::new();
    i.set_vertex_factory(str::to_owned);
    i.import_str(&mut g, input)?;
    Ok(g)
}
fn import_d(input: &str) -> Result<DiGraph<String>, jgrapht_io::IoError> {
    let mut g = DiGraph::<String>::pseudo();
    let mut i = GraphMlImporter::new();
    i.set_vertex_factory(str::to_owned);
    i.import_str(&mut g, input)?;
    Ok(g)
}

mod exporter_test {
    use super::*;
    fn graph(directed: bool, weighted: bool) -> Box<dyn dyn_marker::Sample> {
        if directed {
            Box::new(dyn_marker::D::new(weighted))
        } else {
            Box::new(dyn_marker::U::new(weighted))
        }
    }
    mod dyn_marker {
        use super::*;
        pub trait Sample {
            fn xml(
                &self,
                configure: &mut dyn FnMut(&mut GraphMlExporter<String, DefaultEdge>),
            ) -> String;
        }
        pub struct U(UGraph<String>);
        impl U {
            pub fn new(w: bool) -> Self {
                let mut g = UGraph::simple();
                if w {
                    g = g.weighted();
                }
                g.add_vertices(["v1".into(), "v2".into(), "v3".into()]);
                g.add_edge(&"v1".into(), &"v2".into()).unwrap();
                g.add_edge(&"v3".into(), &"v1".into()).unwrap();
                Self(g)
            }
        }
        pub struct D(DiGraph<String>);
        impl D {
            pub fn new(w: bool) -> Self {
                let mut g = DiGraph::simple();
                if w {
                    g = g.weighted();
                }
                g.add_vertices(["v1".into(), "v2".into(), "v3".into()]);
                g.add_edge(&"v1".into(), &"v2".into()).unwrap();
                g.add_edge(&"v3".into(), &"v1".into()).unwrap();
                Self(g)
            }
        }
        impl Sample for U {
            fn xml(&self, c: &mut dyn FnMut(&mut GraphMlExporter<String, DefaultEdge>)) -> String {
                let mut e = GraphMlExporter::new();
                c(&mut e);
                e.export_to_string(&self.0).unwrap()
            }
        }
        impl Sample for D {
            fn xml(&self, c: &mut dyn FnMut(&mut GraphMlExporter<String, DefaultEdge>)) -> String {
                let mut e = GraphMlExporter::new();
                c(&mut e);
                e.export_to_string(&self.0).unwrap()
            }
        }
    }
    #[test]
    fn undirected() {
        assert!(
            graph(false, false)
                .xml(&mut |_| {})
                .contains("edgedefault=\"undirected\"")
        );
    }
    #[test]
    fn undirected_weighted() {
        assert!(
            !graph(false, true)
                .xml(&mut |_| {})
                .contains("edge_weight_key")
        );
    }
    #[test]
    fn directed() {
        assert!(
            graph(true, false)
                .xml(&mut |_| {})
                .contains("edgedefault=\"directed\"")
        );
    }
    #[test]
    fn undirected_unweighted_with_weights() {
        assert!(
            graph(false, false)
                .xml(&mut |e| e.set_export_edge_weights(true))
                .contains("<default>1.0</default>")
        );
    }
    #[test]
    fn undirected_weighted_with_weights() {
        assert!(
            graph(false, true)
                .xml(&mut |e| e.set_export_edge_weights(true))
                .contains("edge_weight_key")
        );
    }
    #[test]
    fn undirected_weighted_with_custom_name_weights() {
        assert!(
            graph(false, true)
                .xml(&mut |e| {
                    e.set_edge_weight_attribute_name("value").unwrap();
                    e.set_export_edge_weights(true)
                })
                .contains("attr.name=\"value\"")
        );
    }
    #[test]
    fn no_register_weight_attribute() {
        let mut e = GraphMlExporter::<String, DefaultEdge>::new();
        assert!(
            e.register_attribute(
                "weight",
                AttributeCategory::All,
                AttributeType::String,
                None
            )
            .is_err()
        );
    }
    #[test]
    fn register_weight_attribute() {
        let mut e = GraphMlExporter::<String, DefaultEdge>::new();
        e.set_edge_weight_attribute_name("another").unwrap();
        assert!(
            e.register_attribute(
                "weight",
                AttributeCategory::All,
                AttributeType::String,
                None
            )
            .is_ok()
        );
    }
    #[test]
    fn no_already_registered_attribute_as_weight_name() {
        let mut e = GraphMlExporter::<String, DefaultEdge>::new();
        e.register_attribute(
            "length",
            AttributeCategory::Edge,
            AttributeType::String,
            None,
        )
        .unwrap();
        assert!(e.set_edge_weight_attribute_name("length").is_err());
    }
    #[test]
    fn weighted_with_weights_and_labels() {
        let x = graph(false, true).xml(&mut |e| {
            e.set_export_edge_weights(true);
            e.set_export_vertex_labels(true);
            e.set_export_edge_labels(true)
        });
        assert!(x.contains("vertex_label_key") && x.contains("edge_label_key"));
    }
    #[test]
    fn weighted_with_labels_and_custom_names() {
        let x = graph(false, true).xml(&mut |e| {
            e.set_vertex_label_attribute_name("vl").unwrap();
            e.set_edge_label_attribute_name("el").unwrap();
            e.set_export_vertex_labels(true);
            e.set_export_edge_labels(true)
        });
        assert!(x.contains("attr.name=\"vl\"") && x.contains("attr.name=\"el\""));
    }
    #[test]
    fn weighted_with_weights_and_color() {
        let mut e = GraphMlExporter::<String, DefaultEdge>::new();
        e.register_attribute(
            "color",
            AttributeCategory::Node,
            AttributeType::String,
            None,
        )
        .unwrap();
        e.set_vertex_attribute_provider(|_| {
            IndexMap::from([("color".into(), Attribute::from("red"))])
        });
        let g = UGraph::<String>::simple();
        assert!(
            e.export_to_string(&g)
                .unwrap()
                .contains("attr.name=\"color\"")
        );
    }
    #[test]
    fn null_component_provider() {
        let mut e = GraphMlExporter::<String, DefaultEdge>::new();
        e.set_vertex_attribute_provider(|_| AttributeMap::new());
        assert!(e.export_to_string(&UGraph::<String>::simple()).is_ok());
    }
}

mod importer_test {
    use super::*;
    #[test]
    fn undirected_unweighted() {
        let g = import_u(BASIC).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (3, 3));
    }
    #[test]
    fn vertex_factory() {
        let g = import_u(BASIC).unwrap();
        assert!(g.contains_vertex(&"1".into()));
    }
    #[test]
    fn from_input_stream() {
        assert_eq!(import_u(BASIC).unwrap().edge_count(), 3);
    }
    #[test]
    fn pseudograph() {
        let g=import_u("<graphml><graph><node id=\"a\"/><edge source=\"a\" target=\"a\"/><edge source=\"a\" target=\"a\"/></graph></graphml>").unwrap();
        assert_eq!(g.edge_count(), 2);
    }
    #[test]
    fn string_keys() {
        assert!(import_u("<graphml><key id=\"foo\" for=\"node\" attr.name=\"x\"/><graph><node id=\"a\"><data key=\"foo\">v</data></node></graph></graphml>").is_ok());
    }
    #[test]
    fn wrong_order() {
        assert_eq!(import_u(WRONG_ORDER).unwrap().edge_count(), 3);
    }
    #[test]
    fn directed_unweighted() {
        assert_eq!(import_d(BASIC).unwrap().edge_count(), 3);
    }
    #[test]
    fn prefix() {
        assert_eq!(
            import_u("<g:graphml xmlns:g=\"x\"><g:graph><g:node id=\"a\"/></g:graph></g:graphml>")
                .unwrap()
                .vertex_count(),
            1
        );
    }
    #[test]
    fn with_attributes() {
        assert_eq!(import_u(ATTRS).unwrap().edge_count(), 3);
    }
    #[test]
    fn with_map_attributes() {
        let got = Arc::new(Mutex::new(Vec::<(String, Attribute)>::new()));
        let c = got.clone();
        let mut g = UGraph::<String>::pseudo();
        let mut i = GraphMlImporter::new();
        i.set_vertex_factory(str::to_owned);
        i.set_vertex_attribute_consumer(move |_, k, v| {
            c.lock().unwrap().push((k.into(), v.clone()));
            Ok(())
        });
        i.import_str(&mut g, ATTRS).unwrap();
        assert!(
            got.lock()
                .unwrap()
                .iter()
                .any(|(k, v)| k == "color" && v.value() == "yellow")
        );
    }
    #[test]
    fn with_attributes_weighted() {
        let mut g = UGraph::<String>::pseudo().weighted();
        let mut i = GraphMlImporter::new();
        i.set_vertex_factory(str::to_owned);
        i.import_str(&mut g, ATTRS).unwrap();
        let n1 = "n1".to_owned();
        let n2 = "n2".to_owned();
        let e = g.get_edge(&n1, &n2).unwrap();
        assert_eq!(g.edge_weight(e), Some(3.0));
    }
    #[test]
    fn custom_named_weight() {
        let x = ATTRS.replace("attr.name=\"weight\"", "attr.name=\"cost\"");
        let mut g = UGraph::<String>::pseudo().weighted();
        let mut i = GraphMlImporter::new();
        i.set_vertex_factory(str::to_owned);
        i.set_edge_weight_attribute_name("cost");
        i.import_str(&mut g, &x).unwrap();
        assert_eq!(
            g.edge_weight(g.get_edge(&"n0".into(), &"n2".into()).unwrap()),
            Some(2.0)
        );
    }
    #[test]
    fn custom_named_weight_for_all() {
        let x = ATTRS.replace(
            "for=\"edge\" attr.name=\"weight\"",
            "for=\"all\" attr.name=\"weight\"",
        );
        assert_eq!(import_u(&x).unwrap().edge_count(), 3);
    }
    #[test]
    fn hyper_edges() {
        assert!(import_u("<graphml><graph><hyperedge/></graph></graphml>").is_err());
    }
    #[test]
    fn validate() {
        assert!(import_u("<notgraphml/>").is_err());
    }
    #[test]
    fn validate_no_node_id() {
        assert!(import_u("<graphml><graph><node/></graph></graphml>").is_err());
    }
    #[test]
    fn validate_duplicate_node() {
        assert!(
            import_u("<graphml><graph><node id=\"a\"/><node id=\"a\"/></graph></graphml>").is_err()
        );
    }
    #[test]
    fn valid_xlink_node_attribute() {
        assert!(import_u("<graphml><graph><node id=\"a\" xlink:href=\"x\" xmlns:xlink=\"x\"/></graph></graphml>").is_ok());
    }
    #[test]
    fn export_import() {
        let g = import_u(BASIC).unwrap();
        let xml = GraphMlExporter::with_vertex_id_provider(Clone::clone)
            .export_to_string(&g)
            .unwrap();
        assert_eq!(import_u(&xml).unwrap().edge_count(), 3);
    }
    #[test]
    fn attributes_at_graph_level() {
        assert!(import_u("<graphml><key id=\"c\" for=\"graph\" attr.name=\"color\"/><graph><node id=\"a\"/><data key=\"c\">green</data></graph></graphml>").is_ok());
    }
    #[test]
    fn attributes_at_graphml_level() {
        assert!(import_u("<graphml><key id=\"c\" for=\"graph\" attr.name=\"color\"/><graph><node id=\"a\"/></graph><data key=\"c\">green</data></graphml>").is_ok());
    }
    #[test]
    fn nested_graphs() {
        let g=import_u("<graphml><graph><node id=\"a\"><graph><node id=\"b\"/><node id=\"c\"/><edge source=\"b\" target=\"c\"/></graph></node><node id=\"d\"/><edge source=\"a\" target=\"d\"/></graph></graphml>").unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (4, 2));
    }
    #[test]
    fn unsupported_graph() {
        assert!(import_u("<graphml><graph><port/></graph></graphml>").is_err());
    }
    #[test]
    fn non_valid_apoc() {
        assert!(
            import_u(
                "<!DOCTYPE x [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><graphml><graph/></graphml>"
            )
            .is_err()
        );
    }
    #[test]
    fn non_valid_no_vertex_id() {
        assert!(import_u("<graphml><graph><node /></graph></graphml>").is_err());
    }
}

mod simple_importer_test {
    use super::*;
    #[test]
    fn undirected_unweighted() {
        let mut g = UGraph::<String>::pseudo();
        let mut i = SimpleGraphMlImporter::new();
        i.set_vertex_factory(str::to_owned);
        i.import_str(&mut g, BASIC).unwrap();
        assert_eq!(g.edge_count(), 3);
    }
    #[test]
    fn vertex_factory() {
        assert_eq!(import_u(BASIC).unwrap().vertex_count(), 3);
    }
    #[test]
    fn with_consumers() {
        let mut g = UGraph::<String>::pseudo();
        let mut i = SimpleGraphMlImporter::new();
        i.set_vertex_factory(str::to_owned);
        i.set_vertex_attribute_consumer(|_, _, _| Ok(()));
        i.import_str(&mut g, ATTRS).unwrap();
        assert_eq!(g.vertex_count(), 3);
    }
    #[test]
    fn attributes_weighted() {
        let mut g = UGraph::<String>::pseudo().weighted();
        let mut i = SimpleGraphMlImporter::new();
        i.set_vertex_factory(str::to_owned);
        i.import_str(&mut g, ATTRS).unwrap();
        assert_eq!(g.edge_count(), 3);
    }
    #[test]
    fn validate() {
        assert!(import_u("<graphml/>").is_err());
    }
    #[test]
    fn nested_graphs() {
        assert_eq!(import_u("<graphml><graph><node id=\"a\"><graph><node id=\"b\"/></graph></node></graph></graphml>").unwrap().vertex_count(),2);
    }
    #[test]
    fn attributes_at_graphml_level() {
        assert!(import_u("<graphml><graph><node id=\"a\"/></graph></graphml>").is_ok());
    }
}

mod simple_event_test {
    use super::*;
    #[test]
    fn undirected_unweighted() {
        let mut n = 0;
        SimpleGraphMlEventDrivenImporter::new()
            .import_events(Cursor::new(WRONG_ORDER.as_bytes()), |e| {
                if matches!(e, ImportEvent::Edge(_)) {
                    n += 1;
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(n, 3);
    }
    #[test]
    fn attributes_weighted() {
        let mut weights = Vec::new();
        GraphMlEventDrivenImporter::new()
            .import_events(Cursor::new(ATTRS.as_bytes()), |e| {
                if let ImportEvent::Edge((_, _, w)) = e {
                    weights.push(w)
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(weights, [Some(2.0), Some(1.0), Some(3.0)]);
    }
}

mod edge_list_test {
    use super::*;
    #[test]
    fn undirected_unweighted() {
        let mut edges = Vec::new();
        SimpleGraphMlEdgeListImporter::new()
            .import_events(Cursor::new(WRONG_ORDER.as_bytes()), |e| {
                if let ImportEvent::Edge(e) = e {
                    edges.push(e)
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(edges[0], (0, 1, None));
    }
    #[test]
    fn attributes_weighted() {
        let mut edges = Vec::new();
        SimpleGraphMlEdgeListImporter::new()
            .import_events(Cursor::new(ATTRS.as_bytes()), |e| {
                if let ImportEvent::Edge(e) = e {
                    edges.push(e)
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(edges[0].2, Some(2.0));
    }
}
