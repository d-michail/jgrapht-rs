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
use jgrapht_io::gexf::{
    GexfAttributeCategory, GexfAttributeType, GexfEventDrivenImporter, GexfExporter, GexfImporter,
    GexfParameter, SimpleGexfEventDrivenImporter, SimpleGexfImporter,
};
use jgrapht_io::{
    Attribute, AttributeType, EventDrivenImporter, GraphExporter, GraphImporter, ImportEvent,
};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

const BASIC: &str = r#"<gexf version="1.2"><graph defaultedgetype="undirected"><nodes><node id="1" label="1"/><node id="2" label="2"/><node id="3" label="3"/></nodes><edges><edge id="1" source="2" target="3"/><edge id="0" source="1" target="2"/><edge id="2" source="3" target="1"/></edges></graph></gexf>"#;
const ATTRS: &str = r#"<gexf version="1.2"><graph defaultedgetype="directed"><attributes class="node"><attribute id="0" title="color" type="string"/><attribute id="1" title="city" type="liststring"/><attribute id="2" title="weight" type="double"/></attributes><attributes class="edge"><attribute id="0" title="length" type="double"/></attributes><nodes><node id="1"><attvalues><attvalue for="0" value="Red"/><attvalue for="1" value="Paris"/><attvalue for="2" value="100.0"/></attvalues></node><node id="2" label="2"/><node id="3"><attvalues><attvalue for="0" value="Black"/></attvalues></node></nodes><edges><edge id="1" source="2" target="3" weight="1.2"/><edge id="0" source="1" target="2" weight="0.1"><attvalues><attvalue for="0" value="333.0"/></attvalues></edge><edge id="2" source="3" target="1" weight="2.0"/></edges></graph></gexf>"#;

fn import(input: &str) -> Result<UGraph<String>, jgrapht_io::IoError> {
    let mut g = UGraph::<String>::pseudo();
    let mut i = GexfImporter::new();
    i.set_vertex_factory(str::to_owned);
    i.import_str(&mut g, input)?;
    Ok(g)
}

mod importer_test {
    use super::*;
    #[test]
    fn undirected_unweighted() {
        let g = import(BASIC).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (3, 3));
    }
    #[test]
    fn undirected_unweighted_with_meta() {
        assert_eq!(
            import(&BASIC.replace("<graph", "<meta><creator>JGraphT</creator></meta><graph"))
                .unwrap()
                .edge_count(),
            3
        );
    }
    #[test]
    fn vertex_factory() {
        let g = import(BASIC).unwrap();
        assert!(g.contains_vertex(&"1".into()));
    }
    #[test]
    fn undirected_unweighted_with_consumers() {
        let values = Arc::new(Mutex::new(Vec::new()));
        let c = values.clone();
        let mut g = UGraph::<String>::pseudo();
        let mut i = SimpleGexfImporter::new();
        i.set_vertex_factory(str::to_owned);
        i.set_vertex_attribute_consumer(move |v, k, a| {
            c.lock().unwrap().push((v.clone(), k.to_owned(), a.clone()));
            Ok(())
        });
        i.import_str(&mut g, BASIC).unwrap();
        assert!(
            values
                .lock()
                .unwrap()
                .iter()
                .any(|(v, k, a)| v == "1" && k == "label" && a.value() == "1")
        );
    }
    #[test]
    fn attributes_weighted() {
        let attrs = Arc::new(Mutex::new(Vec::<(String, Attribute)>::new()));
        let c = attrs.clone();
        let mut g = DiGraph::<String>::pseudo().weighted();
        let mut i = GexfImporter::new();
        i.set_vertex_factory(str::to_owned);
        i.set_vertex_attribute_consumer(move |_, k, a| {
            c.lock().unwrap().push((k.into(), a.clone()));
            Ok(())
        });
        i.import_str(&mut g, ATTRS).unwrap();
        assert_eq!(
            g.edge_weight(g.get_edge(&"1".into(), &"2".into()).unwrap()),
            Some(0.1)
        );
        assert!(
            attrs
                .lock()
                .unwrap()
                .iter()
                .any(|(k, a)| k == "weight" && a.attribute_type() == AttributeType::Double)
        );
    }
    #[test]
    fn validate() {
        assert!(import("<gexf><graph><node id=\"1\"/></graph></gexf>").is_err());
    }
    #[test]
    fn ignoring_nested() {
        let input = r#"<gexf><graph><nodes><node id="1"><nodes><node id="4"/></nodes><edges><edge source="3" target="4"/></edges></node><node id="2"/><node id="3"/></nodes><edges><edge source="2" target="3"/><edge source="1" target="2"/><edge source="3" target="1"/></edges></graph></gexf>"#;
        let g = import(input).unwrap();
        assert_eq!((g.vertex_count(), g.edge_count()), (3, 3));
        assert!(!g.contains_vertex(&"4".into()));
    }
}

mod event_test {
    use super::*;
    #[test]
    fn undirected_unweighted() {
        let mut count = 0;
        SimpleGexfEventDrivenImporter::new()
            .import_events(Cursor::new(BASIC), |e| {
                if matches!(e, ImportEvent::Edge(_)) {
                    count += 1
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(count, 3);
    }
    #[test]
    fn attributes_weighted() {
        let mut weights = Vec::new();
        GexfEventDrivenImporter::new()
            .import_events(Cursor::new(ATTRS), |e| {
                if let ImportEvent::Edge((_, _, w)) = e {
                    weights.push(w)
                }
                Ok(())
            })
            .unwrap();
        assert_eq!(weights, [Some(1.2), Some(0.1), Some(2.0)]);
    }
}

mod exporter_test {
    use super::*;
    #[test]
    fn directed() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertices(["v1".into(), "v2".into(), "v3".into()]);
        let first = g.add_edge(&"v1".into(), &"v2".into()).unwrap().unwrap();
        g.add_edge(&"v3".into(), &"v1".into()).unwrap();
        let mut e = GexfExporter::new();
        e.set_parameter(GexfParameter::ExportEdgeWeights, true);
        e.set_parameter(GexfParameter::ExportEdgeTypes, true);
        e.set_parameter(GexfParameter::ExportEdgeLabels, true);
        e.register_attribute(
            "color",
            GexfAttributeCategory::Node,
            GexfAttributeType::String,
            None,
            None,
        )
        .unwrap();
        e.register_attribute(
            "hobby",
            GexfAttributeCategory::Node,
            GexfAttributeType::String,
            None,
            Some("ski|dance|photo".into()),
        )
        .unwrap();
        e.register_attribute(
            "length",
            GexfAttributeCategory::Edge,
            GexfAttributeType::Double,
            None,
            None,
        )
        .unwrap();
        e.set_vertex_attribute_provider(|v| {
            if v == "v1" {
                IndexMap::from([("color".into(), Attribute::from("Red"))])
            } else {
                IndexMap::new()
            }
        });
        e.set_edge_attribute_provider(move |edge| {
            if *edge == first {
                IndexMap::from([
                    (
                        "label".into(),
                        Attribute::from("Edge from node 1 to node 2"),
                    ),
                    ("length".into(), Attribute::from("100.0")),
                ])
            } else {
                IndexMap::new()
            }
        });
        let xml = e.export_to_string(&g).unwrap();
        assert!(
            xml.contains("defaultedgetype=\"directed\"")
                && xml.contains("<options>ski|dance|photo</options>")
                && xml.contains("label=\"Edge from node 1 to node 2\"")
        );
    }
    #[test]
    fn undirected() {
        let mut g = UGraph::<String>::pseudo().weighted();
        g.add_vertices(["v1".into(), "v2".into(), "v3".into()]);
        g.add_edge(&"v1".into(), &"v2".into()).unwrap();
        let edge = g.add_edge(&"v3".into(), &"v1".into()).unwrap().unwrap();
        g.set_edge_weight(&edge, 13.5).unwrap();
        let mut e = GexfExporter::new();
        e.set_description("Test");
        e.set_parameter(GexfParameter::ExportEdgeWeights, true);
        e.set_parameter(GexfParameter::ExportEdgeTypes, true);
        let xml = e.export_to_string(&g).unwrap();
        assert!(
            xml.contains("<description>Test</description>")
                && xml.contains("weight=\"13.5\"")
                && xml.contains("type=\"undirected\"")
        );
    }
    #[test]
    fn custom_vertex_and_edge_ids() {
        let mut g = UGraph::<String>::pseudo();
        g.add_vertices(["v1".into(), "v2".into(), "v3".into()]);
        g.add_edge(&"v1".into(), &"v2".into()).unwrap();
        g.add_edge(&"v3".into(), &"v1".into()).unwrap();
        let mut e = GexfExporter::new();
        e.set_vertex_id_provider(|v: &String| format!("node-{v}"));
        e.set_edge_id_provider(|edge: &DefaultEdge| format!("link-{}", edge.0 % 1_000_000));
        let xml = e.export_to_string(&g).unwrap();
        assert!(xml.contains("<node id=\"node-v1\""));
        assert!(xml.contains("source=\"node-v1\" target=\"node-v2\""));
        assert!(xml.contains("id=\"link-"));
        assert!(!xml.contains("id=\"1\""));
    }
    #[test]
    fn empty_custom_vertex_id_is_an_error() {
        let mut g = UGraph::<String>::pseudo();
        g.add_vertex("v1".into());
        let mut e = GexfExporter::new();
        e.set_vertex_id_provider(|_: &String| String::new());
        assert!(e.export_to_string(&g).is_err());
    }
}
