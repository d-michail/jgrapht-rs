use jgrapht::{DefaultEdge, GraphMut, GraphMutExt, UGraph};
use jgrapht_io::lemon::{LemonExporter, LemonParameter};
use jgrapht_io::GraphExporter;

const UNDIRECTED: &str = "#Creator: JGraphT Lemon (LGF) Exporter\n\
#Version: 1\n\
\n\
@nodes\n\
label\n\
1\n\
2\n\
3\n\
\n\
@arcs\n\
\t\t-\n\
1\t2\n\
3\t1\n\
\n";

const UNDIRECTED_DEFAULT_WEIGHTS: &str = "#Creator: JGraphT Lemon (LGF) Exporter\n\
#Version: 1\n\
\n\
@nodes\n\
label\n\
1\n\
2\n\
3\n\
\n\
@arcs\n\
\t\tweight\n\
1\t2\t1.0\n\
3\t1\t1.0\n\
\n";

const UNDIRECTED_WEIGHTED: &str = "#Creator: JGraphT Lemon (LGF) Exporter\n\
#Version: 1\n\
\n\
@nodes\n\
label\n\
1\n\
2\n\
3\n\
\n\
@arcs\n\
\t\tweight\n\
1\t2\t2.0\n\
3\t1\t5.0\n\
\n";

const UNDIRECTED_WITH_ESCAPE: &str = "#Creator: JGraphT Lemon (LGF) Exporter\n\
#Version: 1\n\
\n\
@nodes\n\
label\n\
\"1\"\n\
\"2\"\n\
\"3\"\n\
\n\
@arcs\n\
\t\t-\n\
\"1\"\t\"2\"\n\
\"3\"\t\"1\"\n\
\n";

fn graph(weighted: bool) -> UGraph<&'static str> {
    let mut graph = if weighted {
        UGraph::<&str>::simple().weighted()
    } else {
        UGraph::<&str>::simple()
    };
    graph.add_vertices(["v1", "v2", "v3"]);
    let first = graph.add_edge(&"v1", &"v2").unwrap().unwrap();
    let second = graph.add_edge(&"v3", &"v1").unwrap().unwrap();
    if weighted {
        graph.set_edge_weight(&first, 2.0).unwrap();
        graph.set_edge_weight(&second, 5.0).unwrap();
    }
    graph
}

#[test]
fn undirected() {
    let mut exporter: LemonExporter<&str, DefaultEdge> = LemonExporter::new();
    assert_eq!(exporter.export_to_string(&graph(false)).unwrap(), UNDIRECTED);
}

#[test]
fn unweighted_undirected_with_weights() {
    let mut exporter: LemonExporter<&str, DefaultEdge> = LemonExporter::new();
    exporter.set_parameter(LemonParameter::ExportEdgeWeights, true);
    assert_eq!(
        exporter.export_to_string(&graph(false)).unwrap(),
        UNDIRECTED_DEFAULT_WEIGHTS
    );
}

#[test]
fn weighted_undirected() {
    let mut exporter: LemonExporter<&str, DefaultEdge> = LemonExporter::new();
    exporter.set_parameter(LemonParameter::ExportEdgeWeights, true);
    assert_eq!(
        exporter.export_to_string(&graph(true)).unwrap(),
        UNDIRECTED_WEIGHTED
    );
}

#[test]
fn undirected_with_escape() {
    let mut exporter: LemonExporter<&str, DefaultEdge> = LemonExporter::new();
    exporter.set_parameter(LemonParameter::EscapeStringsAsJava, true);
    assert_eq!(
        exporter.export_to_string(&graph(false)).unwrap(),
        UNDIRECTED_WITH_ESCAPE
    );
}

#[test]
fn parameters() {
    let mut exporter: LemonExporter<&str, DefaultEdge> = LemonExporter::new();
    for parameter in [
        LemonParameter::ExportEdgeWeights,
        LemonParameter::EscapeStringsAsJava,
    ] {
        assert!(!exporter.is_parameter(parameter));
        exporter.set_parameter(parameter, true);
        assert!(exporter.is_parameter(parameter));
        exporter.set_parameter(parameter, false);
        assert!(!exporter.is_parameter(parameter));
    }
}
