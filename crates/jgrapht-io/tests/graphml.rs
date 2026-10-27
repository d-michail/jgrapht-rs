use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt};
use jgrapht_io::graphml::{GraphMlExporter, GraphMlImporter};
use jgrapht_io::{GraphExporter, GraphImporter};

#[test]
fn imports_typed_keys_defaults_and_weights() {
    let input = r#"<?xml version="1.0"?>
    <graphml xmlns="http://graphml.graphdrawing.org/xmlns">
      <key id="label" for="node" attr.name="label" attr.type="string"><default>unknown</default></key>
      <key id="w" for="edge" attr.name="cost" attr.type="double"><default>1.5</default></key>
      <graph id="G" edgedefault="directed">
        <node id="a"><data key="label">Alpha</data></node>
        <node id="b"/>
        <edge source="a" target="b"><data key="w">2.5</data></edge>
      </graph>
    </graphml>"#;
    let mut graph = DiGraph::<String>::multi().weighted();
    let mut importer = GraphMlImporter::new();
    importer.set_vertex_factory(str::to_owned);
    importer.set_edge_weight_attribute_name("cost");
    importer.import_str(&mut graph, input).unwrap();
    assert_eq!(graph.vertex_count(), 2);
    let a = String::from("a");
    let b = String::from("b");
    assert_eq!(
        graph.edge_weight(graph.get_edge(&a, &b).unwrap()),
        Some(2.5)
    );
}

#[test]
fn exports_and_reimports_graphml() {
    let mut graph = DiGraph::<usize>::simple().weighted();
    graph.add_vertices([1, 2]);
    let edge = graph.add_edge(&1, &2).unwrap().unwrap();
    graph.set_edge_weight(&edge, 3.0).unwrap();
    let mut exporter: GraphMlExporter<usize, DefaultEdge> = GraphMlExporter::new();
    let xml = exporter.export_to_string(&graph).unwrap();
    assert!(xml.contains("edgedefault=\"directed\""));
    assert!(xml.contains(">3.0</data>"));

    let mut copy = DiGraph::<usize>::simple().weighted();
    let mut importer = GraphMlImporter::new();
    importer.set_vertex_factory(|id| id.parse().unwrap());
    importer.import_str(&mut copy, &xml).unwrap();
    assert_eq!(copy.edge_count(), 1);
    assert_eq!(copy.edge_weight(copy.edges().next().unwrap()), Some(3.0));
}
