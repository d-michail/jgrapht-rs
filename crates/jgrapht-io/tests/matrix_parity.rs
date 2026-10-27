use jgrapht::{DefaultEdge, DiGraph, GraphMut, GraphMutExt, UGraph};
use jgrapht_io::matrix::{MatrixExporter, MatrixFormat};
use jgrapht_io::GraphExporter;

const LAPLACIAN: &str = "1 1 2\n1 2 -1\n1 3 -1\n2 2 1\n2 1 -1\n3 3 1\n3 1 -1\n";
const NORMALIZED_LAPLACIAN: &str = "1 1 1\n1 2 -0.7071067811865475\n1 3 -0.7071067811865475\n2 2 1\n2 1 -0.7071067811865475\n3 3 1\n3 1 -0.7071067811865475\n";
const UNDIRECTED_ADJACENCY: &str = "1 2 1\n1 3 1\n1 1 2\n2 1 1\n3 1 1\n";
const DIRECTED_ADJACENCY: &str = "1 2 1\n3 1 2\n";

#[test]
fn laplacian() {
    let mut graph = UGraph::<&str>::simple();
    graph.add_vertices(["v1", "v2", "v3"]);
    graph.add_edge(&"v1", &"v2").unwrap();
    graph.add_edge(&"v3", &"v1").unwrap();

    let mut exporter: MatrixExporter<&str, DefaultEdge> =
        MatrixExporter::new(MatrixFormat::SparseLaplacian);
    assert_eq!(exporter.format(), MatrixFormat::SparseLaplacian);
    assert_eq!(exporter.export_to_string(&graph).unwrap(), LAPLACIAN);

    exporter.set_format(MatrixFormat::SparseNormalizedLaplacian);
    assert_eq!(
        exporter.format(),
        MatrixFormat::SparseNormalizedLaplacian
    );
    assert_eq!(
        exporter.export_to_string(&graph).unwrap(),
        NORMALIZED_LAPLACIAN
    );
}

#[test]
fn adjacency_undirected() {
    let mut graph = UGraph::<&str>::pseudo();
    graph.add_vertices(["v1", "v2", "v3"]);
    graph.add_edge(&"v1", &"v2").unwrap();
    graph.add_edge(&"v3", &"v1").unwrap();
    graph.add_edge(&"v1", &"v1").unwrap();

    let mut exporter: MatrixExporter<&str, DefaultEdge> = MatrixExporter::default();
    assert_eq!(exporter.format(), MatrixFormat::SparseAdjacency);
    assert_eq!(
        exporter.export_to_string(&graph).unwrap(),
        UNDIRECTED_ADJACENCY
    );
}

#[test]
fn adjacency_directed() {
    let mut graph = DiGraph::<&str>::multi();
    graph.add_vertices(["v1", "v2", "v3"]);
    graph.add_edge(&"v1", &"v2").unwrap();
    graph.add_edge(&"v3", &"v1").unwrap();
    graph.add_edge(&"v3", &"v1").unwrap();

    let mut exporter: MatrixExporter<&str, DefaultEdge> = MatrixExporter::default();
    assert_eq!(
        exporter.export_to_string(&graph).unwrap(),
        DIRECTED_ADJACENCY
    );
}
