use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt, UGraph};
use jgrapht_io::dimacs::{
    DimacsEventDrivenImporter, DimacsExporter, DimacsFormat, DimacsImporter, DimacsParameter,
};
use jgrapht_io::{
    EventDrivenImporter, GraphExporter, GraphImporter, ImportEvent, IntegerIdProvider,
};

#[test]
fn integer_ids_are_stable_and_configurable() {
    let mut ids = IntegerIdProvider::new();
    assert_eq!(ids.id(&"a"), "1");
    assert_eq!(ids.id(&"b"), "2");
    assert_eq!(ids.id(&"a"), "1");

    let mut ids = IntegerIdProvider::with_start(0);
    assert_eq!(ids.id(&10), "0");
    assert_eq!(ids.id(&20), "1");
}

const DIMACS: &str = "c example\np edge 4 3\ne 1 2\ne 1 4\ne 3 4\n";
const WEIGHTED: &str = "p edge 4 3\ne 1 2 2.5\ne 1 4 3\ne 3 4 nope\n";

#[test]
fn dimacs_event_numbering_modes() {
    let mut importer = DimacsEventDrivenImporter::new();
    let mut edges = Vec::new();
    importer
        .import_events(DIMACS.as_bytes(), |event| {
            if let ImportEvent::Edge(edge) = event {
                edges.push(edge);
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(edges, vec![(0, 1, None), (0, 2, None), (3, 2, None)]);

    let mut importer = DimacsEventDrivenImporter::new()
        .renumber_vertices(false)
        .zero_based_numbering(false);
    edges.clear();
    importer
        .import_events(WEIGHTED.as_bytes(), |event| {
            if let ImportEvent::Edge(edge) = event {
                edges.push(edge);
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(
        edges,
        vec![(1, 2, Some(2.5)), (1, 4, Some(3.0)), (3, 4, None)]
    );
}

#[test]
fn dimacs_imports_weighted_graph_and_factory() {
    let mut graph = DiGraph::<usize>::multi()
        .weighted()
        .with_vertex_factory(|id| id as usize);
    let mut importer = DimacsImporter::new();
    importer.import_str(&mut graph, WEIGHTED).unwrap();
    assert_eq!(graph.vertex_count(), 4);
    assert_eq!(graph.edge_count(), 3);
    assert_eq!(
        graph.edge_weight(graph.get_edge(&0, &1).unwrap()),
        Some(2.5)
    );
    assert_eq!(
        graph.edge_weight(graph.get_edge(&2, &3).unwrap()),
        Some(1.0)
    );

    let mut graph = DiGraph::<usize>::multi().with_vertex_factory(|id| id as usize);
    let mut importer = DimacsImporter::new();
    importer.set_vertex_factory(|id| id + 100);
    importer.import_str(&mut graph, DIMACS).unwrap();
    assert!(graph.contains_edge_between(&101, &102));
    assert!(graph.contains_edge_between(&103, &104));
}

#[test]
fn dimacs_rejects_bad_input() {
    for input in [
        "p edge ERROR 2\ne 1 2\n",
        "p edge -1 0\n",
        "p edge 2 1\ne 1\n",
        "p edge 2 1\ne 1 4\n",
        "c no header\n",
    ] {
        let mut graph = UGraph::<usize>::multi().with_vertex_factory(|id| id as usize);
        assert!(DimacsImporter::new().import_str(&mut graph, input).is_err());
    }
}

#[test]
fn dimacs_export_matches_java_layout() {
    let mut graph = DiGraph::<usize>::multi().weighted();
    graph.add_vertices([1, 2, 3]);
    let first = graph.add_edge(&1, &2).unwrap().unwrap();
    graph.set_edge_weight(&first, 2.0).unwrap();
    let second = graph.add_edge(&3, &1).unwrap().unwrap();
    graph.set_edge_weight(&second, 5.0).unwrap();

    let mut exporter: DimacsExporter<usize, DefaultEdge> =
        DimacsExporter::with_vertex_id_provider(|vertex: &usize| vertex.to_string());
    exporter.set_format(DimacsFormat::ShortestPath);
    exporter.set_parameter(DimacsParameter::ExportEdgeWeights, true);
    assert_eq!(
        exporter.export_to_string(&graph).unwrap(),
        "c\nc SOURCE: Generated using the JGraphT library\nc\np sp 3 2\na 1 2 2.0\na 3 1 5.0\n"
    );
}
