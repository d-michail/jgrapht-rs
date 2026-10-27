use jgrapht_io::dimacs::DimacsEventDrivenImporter;
use jgrapht_io::{
    Attribute, AttributeMap, AttributeType, BaseExporter, EventDrivenImporter, ImportEvent,
    IntegerIdProvider, IoError,
};
use std::str::FromStr;

#[test]
fn attribute_types_round_trip() {
    let types = [
        AttributeType::Null,
        AttributeType::Boolean,
        AttributeType::Int,
        AttributeType::Long,
        AttributeType::Float,
        AttributeType::Double,
        AttributeType::String,
        AttributeType::Html,
        AttributeType::Unknown,
        AttributeType::Identifier,
    ];
    for attribute_type in types {
        assert_eq!(
            AttributeType::from_str(attribute_type.as_str()).unwrap(),
            attribute_type
        );
        assert_eq!(attribute_type.to_string(), attribute_type.as_str());
    }
    assert!(AttributeType::from_str("integer").is_err());
}

#[test]
fn default_attributes_preserve_value_and_type() {
    let values = [
        (Attribute::from(true), "true", AttributeType::Boolean),
        (Attribute::from(7_i32), "7", AttributeType::Int),
        (Attribute::from(8_i64), "8", AttributeType::Long),
        (Attribute::from(1.5_f32), "1.5", AttributeType::Float),
        (Attribute::from(2.5_f64), "2.5", AttributeType::Double),
        (Attribute::from("value"), "value", AttributeType::String),
        (Attribute::null(), "null", AttributeType::Null),
    ];
    for (attribute, value, attribute_type) in values {
        assert_eq!(attribute.value(), value);
        assert_eq!(attribute.to_string(), value);
        assert_eq!(attribute.attribute_type(), attribute_type);
    }
    assert_eq!(
        Attribute::from(7_i32),
        Attribute::new("7", AttributeType::Int)
    );
}

#[test]
fn integer_id_provider_is_stable_and_configurable() {
    let mut ids = IntegerIdProvider::new();
    assert_eq!(ids.id(&"first"), "1");
    assert_eq!(ids.id(&"second"), "2");
    assert_eq!(ids.id(&"first"), "1");

    let mut zero_based = IntegerIdProvider::with_start(0);
    assert_eq!(zero_based.id(&"first"), "0");
    assert_eq!(zero_based.id(&"second"), "1");
}

#[test]
fn base_exporter_manages_all_provider_kinds() {
    let mut exporter = BaseExporter::<i32, i32>::new(|vertex| format!("v{vertex}"));
    assert_eq!(exporter.vertex_id(&3).unwrap(), "v3");

    exporter.set_graph_id_provider(|| "graph".into());
    exporter.set_graph_attribute_provider(|| {
        AttributeMap::from_iter([("title".into(), Attribute::from("demo"))])
    });
    exporter.set_vertex_attribute_provider(|vertex| {
        AttributeMap::from_iter([("value".into(), Attribute::from(*vertex))])
    });
    exporter.set_edge_id_provider(|edge| format!("e{edge}"));
    exporter.set_edge_attribute_provider(|edge| {
        AttributeMap::from_iter([("weight".into(), Attribute::from(*edge as f64))])
    });

    assert!(exporter.has_graph_id_provider());
    assert!(exporter.has_graph_attribute_provider());
    assert!(exporter.has_vertex_attribute_provider());
    assert!(exporter.has_edge_id_provider());
    assert!(exporter.has_edge_attribute_provider());
    assert_eq!(exporter.graph_id().as_deref(), Some("graph"));
    assert_eq!(exporter.graph_attribute("title").unwrap().value(), "demo");
    assert_eq!(exporter.vertex_attribute(&4, "value").unwrap().value(), "4");
    assert_eq!(exporter.edge_id(&5).as_deref(), Some("e5"));
    assert_eq!(exporter.edge_attribute(&5, "weight").unwrap().value(), "5");

    exporter.clear_graph_id_provider();
    exporter.clear_graph_attribute_provider();
    exporter.clear_vertex_attribute_provider();
    exporter.clear_edge_id_provider();
    exporter.clear_edge_attribute_provider();
    assert!(!exporter.has_graph_id_provider());
    assert!(!exporter.has_graph_attribute_provider());
    assert!(!exporter.has_vertex_attribute_provider());
    assert!(!exporter.has_edge_id_provider());
    assert!(!exporter.has_edge_attribute_provider());
}

#[test]
fn event_import_str_and_error_conversion() {
    let mut importer = DimacsEventDrivenImporter::new();
    let mut events = Vec::new();
    importer
        .import_str("p edge 2 1\ne 1 2\n", |event| {
            events.push(event);
            Ok(())
        })
        .unwrap();
    assert!(matches!(events.first(), Some(ImportEvent::Start)));
    assert!(events.contains(&ImportEvent::VertexCount(2)));
    assert!(events.contains(&ImportEvent::EdgeCount(1)));
    assert!(matches!(events.last(), Some(ImportEvent::End)));

    let io_error: IoError = std::io::Error::other("failure").into();
    assert!(io_error.to_string().contains("failure"));
}
