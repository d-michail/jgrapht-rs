use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, UGraph};
use jgrapht_io::gexf::{GexfExporter, GexfImporter};
use jgrapht_io::tsplib::TsplibImporter;
use jgrapht_io::{GraphExporter, GraphImporter};

#[test]
fn gexf_round_trip_preserves_edges_and_weights() {
    let mut graph = DiGraph::<usize>::simple().weighted();
    graph.add_vertex(1);
    graph.add_vertex(2);
    let edge = graph.add_edge(&1, &2).unwrap().unwrap();
    graph.set_edge_weight(&edge, 4.5).unwrap();
    let mut exporter: GexfExporter<usize, DefaultEdge> = GexfExporter::new();
    let xml = exporter.export_to_string(&graph).unwrap();
    assert!(xml.contains("defaultedgetype=\"directed\""));
    assert!(xml.contains("weight=\"4.5\""));

    let mut copy = DiGraph::<usize>::simple().weighted();
    let mut importer = GexfImporter::new();
    importer.set_vertex_factory(|id| id.parse().unwrap());
    importer.import_str(&mut copy, &xml).unwrap();
    assert_eq!(copy.edge_weight(copy.edges().next().unwrap()), Some(4.5));
}

#[test]
fn tsplib_builds_complete_weighted_graph_and_tour() {
    let input = "NAME: triangle\nTYPE: TSP\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EUC_2D\nNODE_COORD_SECTION\n1 0 0\n2 3 4\n3 0 4\nTOUR_SECTION\n1\n3\n2\n-1\nEOF\n";
    let mut graph = UGraph::<usize>::simple()
        .weighted()
        .with_vertex_factory(|id| id as usize);
    let mut importer = TsplibImporter::new();
    importer.import_str(&mut graph, input).unwrap();
    assert_eq!(graph.vertex_count(), 3);
    assert_eq!(graph.edge_count(), 3);
    assert_eq!(
        graph.edge_weight(graph.get_edge(&0, &1).unwrap()),
        Some(5.0)
    );
    let metadata = importer.metadata().unwrap();
    assert_eq!(metadata.name.as_deref(), Some("triangle"));
    assert_eq!(metadata.tour.as_deref(), Some(&[0, 2, 1][..]));
}
