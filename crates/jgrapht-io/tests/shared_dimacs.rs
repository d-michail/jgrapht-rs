use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt, UGraph};
use jgrapht_io::dimacs::{
    DimacsEdge, DimacsEventDrivenImporter, DimacsExporter, DimacsFormat, DimacsImporter,
    DimacsParameter,
};
use jgrapht_io::{
    EventDrivenImporter, GraphExporter, GraphImporter, ImportEvent, IntegerIdProvider,
};

const MYCIEL3: &str = include_str!("fixtures/myciel3.col");
const MYCIEL3_WEIGHTED: &str = include_str!("fixtures/myciel3_weighted.col");
const SHORTEST_PATH: &str = "p sp 3 3\na 1 2\na 2 1\na 2 3\n";

const ORIGINAL_EDGES: [(usize, usize); 20] = [
    (1, 2),
    (1, 4),
    (1, 7),
    (1, 9),
    (2, 3),
    (2, 6),
    (2, 8),
    (3, 5),
    (3, 7),
    (3, 10),
    (4, 5),
    (4, 6),
    (4, 10),
    (5, 8),
    (5, 9),
    (6, 11),
    (7, 11),
    (8, 11),
    (9, 11),
    (10, 11),
];

const UNDIRECTED: &str = concat!(
    "c\n",
    "c SOURCE: Generated using the JGraphT library\n",
    "c\n",
    "p sp 3 2\n",
    "a 1 2\n",
    "a 3 1\n",
);
const UNDIRECTED_WEIGHTED: &str = concat!(
    "c\n",
    "c SOURCE: Generated using the JGraphT library\n",
    "c\n",
    "p sp 3 2\n",
    "a 1 2 2.0\n",
    "a 3 1 5.0\n",
);
const UNDIRECTED_AS_UNWEIGHTED: &str = concat!(
    "c\n",
    "c SOURCE: Generated using the JGraphT library\n",
    "c\n",
    "p sp 3 2\n",
    "a 1 2 1.0\n",
    "a 3 1 1.0\n",
);
const DIRECTED: &str = concat!(
    "c\n",
    "c SOURCE: Generated using the JGraphT library\n",
    "c\n",
    "p sp 5 5\n",
    "a 1 2\n",
    "a 3 1\n",
    "a 2 3\n",
    "a 3 4\n",
    "a 4 5\n",
);
const DIRECTED_MAX_CLIQUE: &str = concat!(
    "c\n",
    "c SOURCE: Generated using the JGraphT library\n",
    "c\n",
    "p edge 5 5\n",
    "e 1 2\n",
    "e 3 1\n",
    "e 2 3\n",
    "e 3 4\n",
    "e 4 5\n",
);
const DIRECTED_COLORING: &str = concat!(
    "c\n",
    "c SOURCE: Generated using the JGraphT library\n",
    "c\n",
    "p col 5 5\n",
    "e 1 2\n",
    "e 3 1\n",
    "e 2 3\n",
    "e 3 4\n",
    "e 4 5\n",
);

fn collect_edges(importer: &mut DimacsEventDrivenImporter, input: &str) -> Vec<DimacsEdge> {
    let mut vertex_count = None;
    let mut edges = Vec::new();
    importer
        .import_events(input.as_bytes(), |event| {
            match event {
                ImportEvent::VertexCount(count) => vertex_count = Some(count),
                ImportEvent::Edge(edge) => edges.push(edge),
                _ => {}
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(vertex_count, Some(11));
    edges
}

fn imported_graph(input: &str, weighted: bool) -> DiGraph<usize> {
    let mut graph = DiGraph::<usize>::pseudo().with_vertex_factory(|id| id as usize);
    if weighted {
        graph = graph.weighted();
    }
    DimacsImporter::new().import_str(&mut graph, input).unwrap();
    graph
}

fn three_vertex_graph(weighted: bool) -> UGraph<&'static str> {
    let mut graph = UGraph::<&str>::simple();
    if weighted {
        graph = graph.weighted();
    }
    graph.add_vertices(["v1", "v2", "v3"]);
    let first = graph.add_edge(&"v1", &"v2").unwrap().unwrap();
    let second = graph.add_edge(&"v3", &"v1").unwrap().unwrap();
    if weighted {
        graph.set_edge_weight(&first, 2.0).unwrap();
        graph.set_edge_weight(&second, 5.0).unwrap();
    }
    graph
}

fn five_vertex_graph() -> DiGraph<&'static str> {
    let mut graph = DiGraph::<&str>::simple();
    graph.add_vertices(["v1", "v2", "v3", "v4", "v5"]);
    for (source, target) in [
        ("v1", "v2"),
        ("v3", "v1"),
        ("v2", "v3"),
        ("v3", "v4"),
        ("v4", "v5"),
    ] {
        graph.add_edge(&source, &target).unwrap();
    }
    graph
}

mod integer_id_provider_test {
    use super::*;

    #[test]
    fn default_constructor() {
        let mut provider = IntegerIdProvider::new();
        assert_eq!(provider.id(&"first"), "1");
        assert_eq!(provider.id(&"second"), "2");
        assert_eq!(provider.id(&"first"), "1");
    }

    #[test]
    fn constructor_with_parameter() {
        let mut provider = IntegerIdProvider::with_start(0);
        assert_eq!(provider.id(&"first"), "0");
        assert_eq!(provider.id(&"second"), "1");
    }
}

mod dimacs_event_driven_importer_test {
    use super::*;

    #[test]
    fn read_dimacs_instance() {
        let edges = collect_edges(&mut DimacsEventDrivenImporter::new(), MYCIEL3);
        let encounter_order = [1, 2, 4, 7, 9, 3, 6, 8, 5, 10, 11];
        let mut names = [0; 12];
        for (mapped, original) in encounter_order.into_iter().enumerate() {
            names[original] = mapped;
        }
        let expected: Vec<_> = ORIGINAL_EDGES
            .iter()
            .map(|&(source, target)| (names[source], names[target], None))
            .collect();
        assert_eq!(edges, expected);
    }

    #[test]
    fn read_weighted_dimacs_instance() {
        let edges = collect_edges(&mut DimacsEventDrivenImporter::new(), MYCIEL3_WEIGHTED);
        let encounter_order = [1, 2, 4, 7, 9, 3, 6, 8, 5, 10, 11];
        let mut names = [0; 12];
        for (mapped, original) in encounter_order.into_iter().enumerate() {
            names[original] = mapped;
        }
        let expected: Vec<_> = ORIGINAL_EDGES
            .iter()
            .enumerate()
            .map(|(index, &(source, target))| {
                (names[source], names[target], Some((index + 1) as f64))
            })
            .collect();
        assert_eq!(edges, expected);
    }

    #[test]
    fn read_weighted_without_zero_based_numbering() {
        let mut importer = DimacsEventDrivenImporter::new().zero_based_numbering(false);
        let edges = collect_edges(&mut importer, MYCIEL3_WEIGHTED);
        let encounter_order = [1, 2, 4, 7, 9, 3, 6, 8, 5, 10, 11];
        let mut names = [0; 12];
        for (mapped, original) in (1..).zip(encounter_order) {
            names[original] = mapped;
        }
        let expected: Vec<_> = ORIGINAL_EDGES
            .iter()
            .enumerate()
            .map(|(index, &(source, target))| {
                (names[source], names[target], Some((index + 1) as f64))
            })
            .collect();
        assert_eq!(edges, expected);
    }

    #[test]
    fn read_weighted_without_renumbering() {
        let mut importer = DimacsEventDrivenImporter::new().renumber_vertices(false);
        let edges = collect_edges(&mut importer, MYCIEL3_WEIGHTED);
        let expected: Vec<_> = ORIGINAL_EDGES
            .iter()
            .enumerate()
            .map(|(index, &(source, target))| (source - 1, target - 1, Some((index + 1) as f64)))
            .collect();
        assert_eq!(edges, expected);
    }

    #[test]
    fn read_weighted_without_renumbering_or_zero_based_numbering() {
        let mut importer = DimacsEventDrivenImporter::new()
            .renumber_vertices(false)
            .zero_based_numbering(false);
        let edges = collect_edges(&mut importer, MYCIEL3_WEIGHTED);
        let expected: Vec<_> = ORIGINAL_EDGES
            .iter()
            .enumerate()
            .map(|(index, &(source, target))| (source, target, Some((index + 1) as f64)))
            .collect();
        assert_eq!(edges, expected);
    }
}

mod dimacs_importer_test {
    use super::*;

    #[test]
    fn read_dimacs_instance() {
        let graph = imported_graph(MYCIEL3, false);
        assert_eq!(graph.vertex_count(), 11);
        assert_eq!(graph.edge_count(), 20);
        for (source, target) in ORIGINAL_EDGES {
            assert!(graph.contains_edge_between(&(source - 1), &(target - 1)));
        }
    }

    #[test]
    fn read_weighted_dimacs_instance() {
        let graph = imported_graph(MYCIEL3_WEIGHTED, true);
        assert_eq!(graph.vertex_count(), 11);
        assert_eq!(graph.edge_count(), 20);
        for (index, (source, target)) in ORIGINAL_EDGES.into_iter().enumerate() {
            let source = source - 1;
            let target = target - 1;
            let edge = graph.get_edge(&source, &target).unwrap();
            assert_eq!(graph.edge_weight(edge), Some((index + 1) as f64));
        }
    }

    #[test]
    fn read_shortest_path_format() {
        let graph = imported_graph(SHORTEST_PATH, false);
        assert_eq!(graph.vertex_count(), 3);
        assert_eq!(graph.edge_count(), 3);
        for (source, target) in [(0, 1), (1, 0), (1, 2)] {
            let edge = graph.get_edge(&source, &target).unwrap();
            assert_eq!(graph.edge_weight(edge), Some(1.0));
        }
    }

    #[test]
    fn read_shortest_path_format_with_vertex_factory() {
        let mut graph = DiGraph::<usize>::pseudo().weighted();
        let mut importer = DimacsImporter::new();
        importer.set_vertex_factory(|id| id + 100);
        importer.import_str(&mut graph, SHORTEST_PATH).unwrap();
        assert_eq!(graph.vertex_count(), 3);
        assert_eq!(graph.edge_count(), 3);
        for (source, target) in [(101, 102), (102, 101), (102, 103)] {
            let edge = graph.get_edge(&source, &target).unwrap();
            assert_eq!(graph.edge_weight(edge), Some(1.0));
        }
    }

    #[test]
    fn wrong_dimacs_instance_1() {
        assert_import_fails("p edge ERROR 5\ne 1 2\ne 1 4\n");
    }

    #[test]
    fn wrong_dimacs_instance_2() {
        assert_import_fails("p edge -10 5\ne 1 2\ne 1 4\n");
    }

    #[test]
    fn wrong_dimacs_instance_3() {
        assert_import_fails("p edge 2 5\ne 1 2\ne 1 4\n");
    }

    #[test]
    fn wrong_dimacs_instance_4() {
        assert_import_fails("p edge 2 2\ne 2\ne 1 2\n");
    }

    fn assert_import_fails(input: &str) {
        let mut graph = DiGraph::<usize>::pseudo().with_vertex_factory(|id| id as usize);
        assert!(DimacsImporter::new().import_str(&mut graph, input).is_err());
    }
}

mod dimacs_exporter_test {
    use super::*;

    #[test]
    fn undirected() {
        let graph = three_vertex_graph(false);
        let mut exporter: DimacsExporter<&str, DefaultEdge> = DimacsExporter::new();
        exporter.set_format(DimacsFormat::ShortestPath);
        assert_eq!(exporter.export_to_string(&graph).unwrap(), UNDIRECTED);
    }

    #[test]
    fn unweighted_undirected_with_exported_weights() {
        let graph = three_vertex_graph(false);
        let mut exporter: DimacsExporter<&str, DefaultEdge> = DimacsExporter::new();
        exporter.set_format(DimacsFormat::ShortestPath);
        exporter.set_parameter(DimacsParameter::ExportEdgeWeights, true);
        assert_eq!(
            exporter.export_to_string(&graph).unwrap(),
            UNDIRECTED_AS_UNWEIGHTED
        );
    }

    #[test]
    fn directed() {
        let graph = five_vertex_graph();
        let mut exporter: DimacsExporter<&str, DefaultEdge> = DimacsExporter::new();
        exporter.set_format(DimacsFormat::ShortestPath);
        assert_eq!(exporter.export_to_string(&graph).unwrap(), DIRECTED);
    }

    #[test]
    fn weighted_undirected() {
        let graph = three_vertex_graph(true);
        let mut exporter: DimacsExporter<&str, DefaultEdge> = DimacsExporter::new();
        exporter.set_format(DimacsFormat::ShortestPath);
        exporter.set_parameter(DimacsParameter::ExportEdgeWeights, true);
        assert_eq!(
            exporter.export_to_string(&graph).unwrap(),
            UNDIRECTED_WEIGHTED
        );
    }

    #[test]
    fn parameters() {
        let mut exporter: DimacsExporter<&str, DefaultEdge> = DimacsExporter::new();
        assert!(!exporter.is_parameter(DimacsParameter::ExportEdgeWeights));
        exporter.set_parameter(DimacsParameter::ExportEdgeWeights, true);
        assert!(exporter.is_parameter(DimacsParameter::ExportEdgeWeights));
        exporter.set_parameter(DimacsParameter::ExportEdgeWeights, false);
        assert!(!exporter.is_parameter(DimacsParameter::ExportEdgeWeights));
    }

    #[test]
    fn default_format() {
        let exporter: DimacsExporter<&str, DefaultEdge> = DimacsExporter::new();
        assert_eq!(exporter.format(), DimacsFormat::MaxClique);
    }

    #[test]
    fn directed_coloring() {
        let graph = five_vertex_graph();
        let mut exporter: DimacsExporter<&str, DefaultEdge> = DimacsExporter::new();
        exporter.set_format(DimacsFormat::Coloring);
        assert_eq!(
            exporter.export_to_string(&graph).unwrap(),
            DIRECTED_COLORING
        );
    }

    #[test]
    fn directed_max_clique() {
        let graph = five_vertex_graph();
        let mut exporter: DimacsExporter<&str, DefaultEdge> = DimacsExporter::new();
        exporter.set_format(DimacsFormat::MaxClique);
        assert_eq!(
            exporter.export_to_string(&graph).unwrap(),
            DIRECTED_MAX_CLIQUE
        );
    }
}
