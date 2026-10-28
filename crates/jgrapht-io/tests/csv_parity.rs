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

use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut, GraphMutExt, UGraph};
use jgrapht_io::csv::{
    CsvExporter, CsvFormat, CsvImporter, CsvParameter, VisioExporter, escape_field, unescape_field,
};
use jgrapht_io::{GraphExporter, GraphImporter};

const EDGES: [(usize, usize); 10] = [
    (1, 2),
    (1, 3),
    (3, 1),
    (3, 4),
    (4, 5),
    (5, 1),
    (5, 2),
    (5, 3),
    (5, 4),
    (5, 5),
];

fn graph_with_edges(edges: &[(usize, usize)], weighted: bool) -> DiGraph<usize> {
    let mut graph = DiGraph::<usize>::pseudo();
    if weighted {
        graph = graph.weighted();
    }
    graph.add_vertices(1..=5);
    for &(source, target) in edges {
        let edge = graph.add_edge(&source, &target).unwrap().unwrap();
        if weighted {
            graph.set_edge_weight(&edge, 1.0).unwrap();
        }
    }
    graph
}

fn exporter(format: CsvFormat) -> CsvExporter<usize, DefaultEdge> {
    CsvExporter::with_vertex_id_provider(|vertex: &usize| vertex.to_string(), format, ';').unwrap()
}

fn import_graph(
    input: &str,
    format: CsvFormat,
    delimiter: char,
    weighted: bool,
    node_ids: bool,
    zero_missing: bool,
) -> Result<DiGraph<String>, jgrapht_io::IoError> {
    let mut graph = DiGraph::<String>::pseudo().with_vertex_factory(|id| (id + 1).to_string());
    if weighted {
        graph = graph.weighted();
    }
    let mut importer = CsvImporter::with_delimiter(format, delimiter)?;
    importer.set_parameter(CsvParameter::EdgeWeights, weighted);
    importer.set_parameter(CsvParameter::MatrixNodeId, node_ids);
    importer.set_parameter(CsvParameter::MatrixZeroWhenNoEdge, zero_missing);
    importer.import_str(&mut graph, input)?;
    Ok(graph)
}

fn assert_standard_graph(graph: &DiGraph<String>) {
    assert_eq!(graph.vertex_count(), 5);
    assert_eq!(graph.edge_count(), 10);
    for &(source, target) in &EDGES {
        assert!(graph.contains_edge_between(&source.to_string(), &target.to_string()));
    }
}

fn assert_weight(graph: &DiGraph<String>, source: &str, target: &str, weight: f64) {
    let source = source.to_owned();
    let target = target.to_owned();
    let edge = graph.get_edge(&source, &target).unwrap();
    assert_eq!(graph.edge_weight(edge), Some(weight));
}

mod dsv_utils_test {
    use super::*;

    #[test]
    fn escape() {
        assert_eq!(
            escape_field("nothing special in here", ';'),
            "nothing special in here"
        );
        assert_eq!(escape_field("foo;;;", ','), "foo;;;");
        assert_eq!(escape_field("foo;;;", ';'), "\"foo;;;\"");
        assert_eq!(escape_field("foo\n", ';'), "\"foo\n\"");
        assert_eq!(escape_field("foo\rfoo", ';'), "\"foo\rfoo\"");
        assert_eq!(
            escape_field("\"foo\"\n\"foo\"", ';'),
            "\"\"\"foo\"\"\n\"\"foo\"\"\""
        );
    }

    #[test]
    fn unescape() {
        assert_eq!(
            unescape_field("nothing special in here", ';'),
            "nothing special in here"
        );
        assert_eq!(unescape_field("\"foo;;;\"", ';'), "foo;;;");
        assert_eq!(unescape_field("\"foo;;;\"", ','), "\"foo;;;\"");
        assert_eq!(unescape_field("\"foo\n\"", ';'), "foo\n");
        assert_eq!(unescape_field("\"foo\rfoo\"", ';'), "foo\rfoo");
        assert_eq!(
            unescape_field("\"\"\"foo\"\"\n\"\"foo\"\"\"", ';'),
            "\"foo\"\n\"foo\""
        );
    }
}

mod csv_exporter_test {
    use super::*;
    use jgrapht::GraphExt;

    const DIRECTED_EDGE_LIST: &str = "1;2\n1;3\n3;1\n3;4\n4;5\n5;1\n";
    const UNDIRECTED_EDGE_LIST: &str = "1;2\n1;3\n3;4\n4;5\n5;1\n";
    const DIRECTED_ADJACENCY_LIST: &str = "1;2;3\n2\n3;1;4\n4;5\n5;1;2;3;4;5;5\n";
    const UNDIRECTED_ADJACENCY_LIST: &str =
        "1;2;3;3;5\n2;1;5\n3;1;1;4;5\n4;3;5;5\n5;4;1;2;3;4;5;5\n";
    const MATRIX: &str = ";1;1;;\n;;;;\n1;;;1;\n;;;;1\n1;1;1;1;1\n";
    const MATRIX_ZERO: &str = "0;1;1;0;0\n0;0;0;0;0\n1;0;0;1;0\n0;0;0;0;1\n1;1;1;1;1\n";
    const MATRIX_IDS: &str = concat!(
        ";1;2;3;4;5\n",
        "1;;1;1;;\n",
        "2;;;;;\n",
        "3;1;;;1;\n",
        "4;;;;;1\n",
        "5;1;1;1;1;1\n",
    );
    const MATRIX_IDS_ZERO: &str = concat!(
        ";1;2;3;4;5\n",
        "1;0;1;1;0;0\n",
        "2;0;0;0;0;0\n",
        "3;1;0;0;1;0\n",
        "4;0;0;0;0;1\n",
        "5;1;1;1;1;1\n",
    );

    #[test]
    fn undirected_edge_list() {
        let mut graph = UGraph::<usize>::simple();
        graph.add_vertices(1..=5);
        for edge in [(1, 2), (1, 3), (3, 1), (3, 4), (4, 5), (5, 1)] {
            let _ = graph.add_edge(&edge.0, &edge.1);
        }
        assert_eq!(
            exporter(CsvFormat::EdgeList)
                .export_to_string(&graph)
                .unwrap(),
            UNDIRECTED_EDGE_LIST
        );
    }

    #[test]
    fn directed_edge_list() {
        let graph = graph_with_edges(&[(1, 2), (1, 3), (3, 1), (3, 4), (4, 5), (5, 1)], false);
        assert_eq!(
            exporter(CsvFormat::EdgeList)
                .export_to_string(&graph)
                .unwrap(),
            DIRECTED_EDGE_LIST
        );
    }

    #[test]
    fn directed_weighted_edge_list() {
        let mut graph = graph_with_edges(&[(1, 2), (1, 3), (3, 1), (3, 4), (4, 5), (5, 1)], true);
        for edge in graph.edges_cloned().collect::<Vec<_>>() {
            graph.set_edge_weight(&edge, 2.0).unwrap();
        }
        let mut csv = exporter(CsvFormat::EdgeList);
        csv.set_parameter(CsvParameter::EdgeWeights, true);
        assert_eq!(
            csv.export_to_string(&graph).unwrap(),
            "1;2;2.0\n1;3;2.0\n3;1;2.0\n3;4;2.0\n4;5;2.0\n5;1;2.0\n"
        );
    }

    #[test]
    fn directed_adjacency_list() {
        let edges = [EDGES.as_slice(), &[(5, 5)]].concat();
        let graph = graph_with_edges(&edges, false);
        assert_eq!(
            exporter(CsvFormat::AdjacencyList)
                .export_to_string(&graph)
                .unwrap(),
            DIRECTED_ADJACENCY_LIST
        );
    }

    #[test]
    fn directed_weighted_adjacency_list() {
        let edges = [EDGES.as_slice(), &[(5, 5)]].concat();
        let mut graph = graph_with_edges(&edges, true);
        for edge in graph.edges_cloned().collect::<Vec<_>>() {
            graph.set_edge_weight(&edge, 3.3).unwrap();
        }
        let mut csv = exporter(CsvFormat::AdjacencyList);
        csv.set_parameter(CsvParameter::EdgeWeights, true);
        assert_eq!(
            csv.export_to_string(&graph).unwrap(),
            "1;2;3.3;3;3.3\n2\n3;1;3.3;4;3.3\n4;5;3.3\n5;1;3.3;2;3.3;3;3.3;4;3.3;5;3.3;5;3.3\n"
        );
    }

    #[test]
    fn undirected_adjacency_list() {
        let mut graph = UGraph::<usize>::pseudo();
        graph.add_vertices(1..=5);
        for &(source, target) in &EDGES {
            graph.add_edge(&source, &target).unwrap();
        }
        graph.add_edge(&5, &5).unwrap();
        assert_eq!(
            exporter(CsvFormat::AdjacencyList)
                .export_to_string(&graph)
                .unwrap(),
            UNDIRECTED_ADJACENCY_LIST
        );
    }

    #[test]
    fn directed_matrix_node_id() {
        let mut csv = exporter(CsvFormat::Matrix);
        csv.set_parameter(CsvParameter::MatrixNodeId, true);
        assert_eq!(
            csv.export_to_string(&graph_with_edges(&EDGES, false))
                .unwrap(),
            MATRIX_IDS
        );
    }

    #[test]
    fn directed_matrix_no_node_id() {
        assert_eq!(
            exporter(CsvFormat::Matrix)
                .export_to_string(&graph_with_edges(&EDGES, false))
                .unwrap(),
            MATRIX
        );
    }

    #[test]
    fn directed_matrix_node_id_zero_missing_edges() {
        let mut csv = exporter(CsvFormat::Matrix);
        csv.set_parameter(CsvParameter::MatrixNodeId, true);
        csv.set_parameter(CsvParameter::MatrixZeroWhenNoEdge, true);
        assert_eq!(
            csv.export_to_string(&graph_with_edges(&EDGES, false))
                .unwrap(),
            MATRIX_IDS_ZERO
        );
    }

    #[test]
    fn directed_matrix_no_node_id_zero_missing_edges() {
        let mut csv = exporter(CsvFormat::Matrix);
        csv.set_parameter(CsvParameter::MatrixZeroWhenNoEdge, true);
        assert_eq!(
            csv.export_to_string(&graph_with_edges(&EDGES, false))
                .unwrap(),
            MATRIX_ZERO
        );
    }

    #[test]
    fn directed_matrix_no_node_id_zero_missing_edges_weighted() {
        let mut graph = graph_with_edges(&EDGES, true);
        let e13 = *graph.get_edge(&1, &3).unwrap();
        let e53 = *graph.get_edge(&5, &3).unwrap();
        graph.set_edge_weight(&e13, 13.0).unwrap();
        graph.set_edge_weight(&e53, 53.0).unwrap();
        let mut csv = exporter(CsvFormat::Matrix);
        csv.set_parameter(CsvParameter::MatrixZeroWhenNoEdge, true);
        csv.set_parameter(CsvParameter::EdgeWeights, true);
        assert_eq!(
            csv.export_to_string(&graph).unwrap(),
            "0;1.0;13.0;0;0\n0;0;0;0;0\n1.0;0;0;1.0;0\n0;0;0;0;1.0\n1.0;1.0;53.0;1.0;1.0\n"
        );
    }

    #[test]
    fn directed_matrix_no_node_id_weighted() {
        let mut graph = graph_with_edges(&EDGES, true);
        let e13 = *graph.get_edge(&1, &3).unwrap();
        let e53 = *graph.get_edge(&5, &3).unwrap();
        graph.set_edge_weight(&e13, 13.0).unwrap();
        graph.set_edge_weight(&e53, 53.0).unwrap();
        let mut csv = exporter(CsvFormat::Matrix);
        csv.set_parameter(CsvParameter::EdgeWeights, true);
        assert_eq!(
            csv.export_to_string(&graph).unwrap(),
            ";1.0;13.0;;\n;;;;\n1.0;;;1.0;\n;;;;1.0\n1.0;1.0;53.0;1.0;1.0\n"
        );
    }

    #[test]
    fn edge_list_with_strings_directed_unweighted_with_semicolon() {
        let mut graph = DiGraph::<String>::pseudo();
        let names = ["'john doe'", "fred", "fred\n\"21\"", "who;;"];
        graph.add_vertices(names.map(str::to_owned));
        for (source, target) in names
            .into_iter()
            .zip(names.into_iter().cycle().skip(1))
            .take(4)
        {
            graph
                .add_edge(&source.to_owned(), &target.to_owned())
                .unwrap();
        }
        let mut csv =
            CsvExporter::with_vertex_id_provider(|v: &String| v.clone(), CsvFormat::EdgeList, ';')
                .unwrap();
        assert_eq!(
            csv.export_to_string(&graph).unwrap(),
            "'john doe';fred\nfred;\"fred\n\"\"21\"\"\"\n\"fred\n\"\"21\"\"\";\"who;;\"\n\"who;;\";'john doe'\n"
        );
    }
}

mod csv_importer_test {
    use super::*;

    const MATRIX: &str = ";1;1;;\n;;;;\n1;;;1;\n;;;;1\n1;1;1;1;1\n";
    const MATRIX_ZERO: &str = "0;1;1;0;0\n0;0;0;0;0\n1;0;0;1;0\n0;0;0;0;1\n1;1;1;1;1\n";
    const MATRIX_WEIGHTED: &str = ";1.0;13.0;;\n;;;;\n1.0;;;1.0;\n;;;;1.0\n1.0;1.0;53.0;1.0;1.0\n";
    const MATRIX_WEIGHTED_ZERO: &str =
        "0;1.0;13.0;0;0\n0;0;0;0;0\n1.0;0;0;1.0;0\n0;0;0;0;1.0\n1.0;1.0;53.0;1.0;1.0\n";
    const MATRIX_IDS_ZERO: &str =
        ";A;B;C;D;E\nA;0;1;1;0;0\nB;0;0;0;0;0\nC;1;0;0;1;0\nD;0;0;0;0;1\nE;1;1;1;1;1\n";
    const MATRIX_IDS_SHUFFLED: &str =
        ";A;B;C;D;E\nC;1;0;0;1;0\nD;0;0;0;0;1\nB;0;0;0;0;0\nA;0;1;1;0;0\nE;1;1;1;1;1\n";

    #[test]
    fn edge_list_directed_unweighted() {
        let graph = import_graph(
            "1,2\n2,3\n3,4\n4,1\n",
            CsvFormat::EdgeList,
            ',',
            false,
            false,
            false,
        )
        .unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (4, 4));
        for edge in [("1", "2"), ("2", "3"), ("3", "4"), ("4", "1")] {
            assert!(graph.contains_edge_between(&edge.0.into(), &edge.1.into()));
        }
    }

    #[test]
    fn edge_list_directed_weighted() {
        let graph = import_graph(
            "1,2,1.0\n2,3,2.0\n3,4,3.0\n4,1,4.0\n",
            CsvFormat::EdgeList,
            ',',
            true,
            false,
            false,
        )
        .unwrap();
        for (source, target, weight) in [(1, 2, 1.0), (2, 3, 2.0), (3, 4, 3.0), (4, 1, 4.0)] {
            assert_weight(&graph, &source.to_string(), &target.to_string(), weight);
        }
    }

    #[test]
    fn edge_list_directed_unweighted_with_semicolon() {
        let graph = import_graph(
            "1;2\n2;3\n3;4\n4;1\n",
            CsvFormat::EdgeList,
            ';',
            false,
            false,
            false,
        )
        .unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (4, 4));
    }

    #[test]
    fn adjacency_list_directed_unweighted_with_semicolon() {
        let graph = import_graph(
            "1;2;3;4\n2;3\n3;4;5;6\n4;1;5;6\n",
            CsvFormat::AdjacencyList,
            ';',
            false,
            false,
            false,
        )
        .unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (6, 10));
    }

    #[test]
    fn adjacency_list_directed_weighted_with_semicolon() {
        let graph = import_graph(
            "1;2;2.1;3;3.1;4;4.1\n2;3;3.1\n3;4;4.1;5;5.1;6;6.1\n4;1;1.1;5;5.1;6;6.1\n",
            CsvFormat::AdjacencyList,
            ';',
            true,
            false,
            false,
        )
        .unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (6, 10));
        assert_weight(&graph, "4", "1", 1.1);
    }

    #[test]
    fn edge_list_with_strings_directed_unweighted_with_semicolon() {
        let input = "'john doe';fred\nfred;\"fred\n\"\"21\"\"\"\n\"fred\n\"\"21\"\"\";\"who;;\"\n\"who;;\";'john doe'\n";
        let graph = import_graph(input, CsvFormat::EdgeList, ';', false, false, false).unwrap();
        assert_eq!((graph.vertex_count(), graph.edge_count()), (4, 4));
    }

    #[test]
    fn directed_matrix_no_node_id_zero_no_edge_weighted() {
        let graph = import_graph(
            MATRIX_WEIGHTED_ZERO,
            CsvFormat::Matrix,
            ';',
            true,
            false,
            true,
        )
        .unwrap();
        assert_standard_graph(&graph);
        assert_weight(&graph, "5", "3", 53.0);
    }

    #[test]
    fn directed_matrix_no_node_id_weighted() {
        let graph =
            import_graph(MATRIX_WEIGHTED, CsvFormat::Matrix, ';', true, false, false).unwrap();
        assert_standard_graph(&graph);
    }

    #[test]
    fn directed_matrix_no_node_id_zero_no_edge() {
        assert_standard_graph(
            &import_graph(MATRIX_ZERO, CsvFormat::Matrix, ';', false, false, true).unwrap(),
        );
    }

    #[test]
    fn directed_matrix_no_node_id() {
        assert_standard_graph(
            &import_graph(MATRIX, CsvFormat::Matrix, ';', false, false, false).unwrap(),
        );
    }

    #[test]
    fn directed_matrix_node_id_zero_no_edge() {
        assert_standard_graph(
            &import_graph(MATRIX_IDS_ZERO, CsvFormat::Matrix, ';', false, true, true).unwrap(),
        );
    }

    #[test]
    fn directed_matrix_node_id_zero_no_edge_shuffled() {
        assert_standard_graph(
            &import_graph(
                MATRIX_IDS_SHUFFLED,
                CsvFormat::Matrix,
                ';',
                false,
                true,
                true,
            )
            .unwrap(),
        );
    }

    #[test]
    fn directed_matrix_node_id_zero_no_edge_weighted_shuffled_zero_weights_as_double() {
        let input =
            ";A;B;C;D;E\nC;1;0;0;1;0\nD;0;0;0.0;0;1\nB;0;0;0;0;0\nA;0;1;1;0;0\nE;1;1;0;1;1\n";
        let graph = import_graph(input, CsvFormat::Matrix, ';', true, true, true).unwrap();
        assert_eq!(graph.edge_count(), 10);
        assert_weight(&graph, "4", "3", 0.0);
    }

    #[test]
    fn double_on_unweighted() {
        let input =
            ";A;B;C;D;E\nC;1;0;0;1;0\nD;0;0;0.0;0;1\nB;0;0;0;0;0\nA;0;1;1;0;0\nE;1;1;0;1;1\n";
        assert!(import_graph(input, CsvFormat::Matrix, ';', false, true, true).is_err());
    }

    #[test]
    fn wrong_header_node_ids() {
        let input =
            ";A;B;  ;D;E\nC;1;0;0;1;0\nD;0;0;0;0;1\nB;0;0;0;0;0\nA;0;1;1;0;0\nE;1;1;0;1;1\n";
        assert!(import_graph(input, CsvFormat::Matrix, ';', false, true, true).is_err());
    }

    #[test]
    fn directed_matrix_no_node_id_missing_entries() {
        let input = ";1;1;;\n;;;;\n1;;;1;\n;;;1\n1;1;1;1;1\n";
        assert!(import_graph(input, CsvFormat::Matrix, ';', false, false, false).is_err());
    }

    #[test]
    fn directed_matrix_node_id_zero_no_edge_shuffled_and_tab_delimiter() {
        let input = "\tA\tB\t\"C\tC\"\tD\tE\n\"C\tC\"\t1\t0\t0\t1\t0\nD\t0\t0\t0\t0\t1\nB\t0\t0\t0\t0\t0\nA\t0\t1\t1\t0\t0\nE\t1\t1\t1\t1\t1\n";
        assert_standard_graph(
            &import_graph(input, CsvFormat::Matrix, '\t', false, true, true).unwrap(),
        );
    }
}

#[test]
fn visio_exporter_matches_java_layout() {
    let mut graph = DiGraph::<&str>::simple();
    graph.add_vertices(["a", "b"]);
    graph.add_edge(&"a", &"b").unwrap();
    let mut exporter = VisioExporter::with_vertex_id_provider(|v: &&str| (*v).to_owned());
    assert_eq!(
        exporter.export_to_string(&graph).unwrap(),
        "Shape,a,,a\nShape,b,,b\nLink,a-->b,,,a,b\n"
    );
}
