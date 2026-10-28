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

use jgrapht::generate::named;
use jgrapht::graph::IncidenceGraph;
use jgrapht::{DiGraph, Graph, GraphMut, GraphMutExt, UGraph};
use jgrapht_io::graph6::{Graph6Exporter, Graph6Format, Graph6Importer};
use jgrapht_io::{GraphExporter, GraphImporter};

fn import_undirected(encoded: &str) -> UGraph<usize> {
    let mut graph = UGraph::<usize>::pseudo().with_vertex_factory(|id| id as usize);
    Graph6Importer::new()
        .import_str(&mut graph, encoded)
        .unwrap();
    graph
}

fn signature<G: IncidenceGraph>(graph: &G) -> (usize, usize, Vec<usize>) {
    let mut degrees: Vec<_> = graph.vertices().map(|v| graph.degree_of(v)).collect();
    degrees.sort_unstable();
    (graph.vertex_count(), graph.edge_count(), degrees)
}

fn compare_named(encoded: &str, structure: jgrapht::generate::Structure) {
    let expected = named::build(structure);
    assert_eq!(signature(&import_undirected(encoded)), signature(&expected));
}

fn round_trip(graph: &UGraph<usize>, format: Graph6Format) {
    let encoded = Graph6Exporter::new(format).export_to_string(graph).unwrap();
    assert_eq!(signature(&import_undirected(&encoded)), signature(graph));
}

fn encoded_named(structure: jgrapht::generate::Structure, format: Graph6Format) -> String {
    Graph6Exporter::new(format)
        .export_to_string(&named::build(structure))
        .unwrap()
}

mod importer_test {
    use super::*;

    #[test]
    fn example_graph() {
        let graph = import_undirected(":Fa@x^\n");
        assert_eq!((graph.vertex_count(), graph.edge_count()), (7, 4));
        for (source, target) in [(0, 1), (0, 2), (1, 2), (5, 6)] {
            assert!(graph.contains_edge_between(&source, &target));
        }
    }

    #[test]
    fn pseudo_graph() {
        let graph = import_undirected(":B_`V");
        assert_eq!((graph.vertex_count(), graph.edge_count()), (3, 5));
        assert_eq!(graph.edges_between(&0, &1).count(), 2);
        assert!(graph.contains_edge_between(&2, &2));
    }

    #[test]
    fn vertex_factory() {
        let mut graph = UGraph::<String>::pseudo();
        let mut importer = Graph6Importer::new();
        importer.set_vertex_factory(|id| format!("node{id}"));
        importer.import_str(&mut graph, ":B_`V").unwrap();
        assert!(graph.contains_edge_between(&"node0".into(), &"node1".into()));
        assert!(graph.contains_edge_between(&"node2".into(), &"node2".into()));
    }

    #[test]
    fn number_vertices_1() {
        let input = "~??~?????_@?CG??B??@OG?C?G???GO??W@a???CO???OACC?OA?P@G??O??????G??C????c?G?CC?_?@???C_??_?C????PO?C_??AA?OOAHCA___?CC?A?CAOGO??????A??G?GR?C?_o`???g???A_C?OG??O?G_IA????_QO@EG???O??C?_?C@?G???@?_??AC?AO?a???O?????A?_Dw?H???__O@AAOAACd?_C??G?G@??GO?_???O@?_O??W??@P???AG??B?????G??GG???A??@?aC_G@A??O??_?A?????O@Z?_@M????GQ@_G@?C?";
        assert_eq!(import_undirected(input).vertex_count(), 63);
    }

    #[test]
    fn number_vertices_2() {
        let input =
            "_???C?@AA?_?A?O?C??S??O?q_?P?CHD??@?C?GC???C??GG?C_??O?COG????I?J??Q??O?_@@??@??????";
        assert_eq!(import_undirected(input).vertex_count(), 32);
    }

    #[test]
    fn graph6_a() {
        compare_named(
            "WzK[WgIOT@Wq_A?NALPAq?{GDASCCXO?l?OJAGOY_D@__wb",
            named::klein7_regular(),
        );
    }

    #[test]
    fn graph6_b() {
        let encoded = encoded_named(named::ellingham_horton78(), Graph6Format::Graph6);
        compare_named(&encoded, named::ellingham_horton78());
    }

    #[test]
    fn graph6_c() {
        compare_named("JntIBcPEA~_", named::goldner_harary());
    }

    #[test]
    fn graph6_d() {
        let encoded = encoded_named(named::bucky_ball(), Graph6Format::Graph6);
        compare_named(&encoded, named::bucky_ball());
    }

    #[test]
    fn graph6_f() {
        compare_named("MhEGHC@AI?_PC@_G_", named::heawood());
    }

    #[test]
    fn sparse6_a() {
        compare_named(
            ":W__@`AaBbC_CDbDcE`F_AG_@DEH_IgHIJbFGIKaFHILeFGHMdFKN_EKOPaCNPQ`HOQRcGLRS`BKMSTdJKLPTU",
            named::klein7_regular(),
        );
    }

    #[test]
    fn sparse6_b() {
        let encoded = encoded_named(named::ellingham_horton78(), Graph6Format::Sparse6);
        compare_named(&encoded, named::ellingham_horton78());
    }

    #[test]
    fn sparse6_c() {
        compare_named(":J`E?POAMHGpCKsrrHCXAeM`N", named::goldner_harary());
    }

    #[test]
    fn sparse6_d() {
        let encoded = encoded_named(named::bucky_ball(), Graph6Format::Sparse6);
        compare_named(&encoded, named::bucky_ball());
    }

    #[test]
    fn sparse6_f() {
        compare_named(":M`ESwCjGtyGaeqhj_`f", named::heawood());
    }

    #[test]
    fn sparse6_g() {
        compare_named(
            include_str!("fixtures/ellingham_horton78.s6"),
            named::ellingham_horton78(),
        );
    }

    #[test]
    fn from_file() {
        let graph = import_undirected(include_str!("fixtures/ellingham_horton78.s6"));
        assert_eq!(
            signature(&graph),
            signature(&named::build(named::ellingham_horton78()))
        );
    }
}

mod exporter_test {
    use super::*;

    #[test]
    fn empty_graph() {
        assert_eq!(
            Graph6Exporter::new(Graph6Format::Sparse6)
                .export_to_string(&UGraph::<usize>::simple())
                .unwrap(),
            ":?"
        );
    }

    #[test]
    fn graph_without_edges() {
        let mut graph = UGraph::<usize>::simple();
        graph.add_vertices([0, 1]);
        assert_eq!(
            Graph6Exporter::new(Graph6Format::Sparse6)
                .export_to_string(&graph)
                .unwrap(),
            ":A"
        );
    }

    #[test]
    fn example_graph() {
        let mut graph = UGraph::<usize>::simple();
        graph.add_vertices(0..7);
        for (source, target) in [(0, 1), (0, 2), (1, 2), (5, 6)] {
            graph.add_edge(&source, &target).unwrap();
        }
        assert_eq!(
            Graph6Exporter::new(Graph6Format::Sparse6)
                .export_to_string(&graph)
                .unwrap(),
            ":Fa@x^"
        );
    }

    macro_rules! named_round_trip {
        ($name:ident, $graph:expr, $format:expr) => {
            #[test]
            fn $name() {
                round_trip(&named::build($graph), $format);
            }
        };
    }

    named_round_trip!(graph_1a, named::petersen(), Graph6Format::Sparse6);
    named_round_trip!(graph_2a, named::ellingham_horton78(), Graph6Format::Sparse6);
    named_round_trip!(graph_3a, named::klein3_regular(), Graph6Format::Sparse6);

    #[test]
    fn pseudo_graph() {
        let mut graph = UGraph::<usize>::pseudo();
        graph.add_vertices(0..3);
        for edge in [(0, 1), (0, 1), (1, 2), (2, 0), (2, 2)] {
            graph.add_edge(&edge.0, &edge.1).unwrap();
        }
        round_trip(&graph, Graph6Format::Sparse6);
    }

    #[test]
    fn random_graphs_s6() {
        for seed in 0..20 {
            let mut graph = UGraph::<usize>::pseudo();
            graph.add_vertices(0..40);
            for high in 0..40 {
                for low in 0..=high {
                    if (low * 31 + high * 17 + seed) % 11 < 6 {
                        graph.add_edge(&low, &high).unwrap();
                    }
                }
            }
            round_trip(&graph, Graph6Format::Sparse6);
        }
    }

    named_round_trip!(graph_1b, named::petersen(), Graph6Format::Graph6);
    named_round_trip!(graph_2b, named::ellingham_horton78(), Graph6Format::Graph6);
    named_round_trip!(graph_3b, named::klein3_regular(), Graph6Format::Graph6);

    #[test]
    fn random_graphs_g6() {
        for seed in 0..20 {
            let mut graph = UGraph::<usize>::simple();
            graph.add_vertices(0..40);
            for high in 0..40 {
                for low in 0..high {
                    if (low * 31 + high * 17 + seed) % 11 < 6 {
                        graph.add_edge(&low, &high).unwrap();
                    }
                }
            }
            round_trip(&graph, Graph6Format::Graph6);
        }
    }

    #[test]
    fn digraph6_round_trip() {
        let mut graph = DiGraph::<usize>::simple();
        graph.add_vertices(0..4);
        for edge in [(0, 1), (1, 2), (2, 0), (3, 0)] {
            graph.add_edge(&edge.0, &edge.1).unwrap();
        }
        let encoded = Graph6Exporter::new(Graph6Format::Digraph6)
            .export_to_string(&graph)
            .unwrap();
        assert!(encoded.starts_with('&'));
        let mut copy = DiGraph::<usize>::pseudo().with_vertex_factory(|id| id as usize);
        Graph6Importer::new()
            .import_str(&mut copy, &encoded)
            .unwrap();
        for edge in [(0, 1), (1, 2), (2, 0), (3, 0)] {
            assert!(copy.contains_edge_between(&edge.0, &edge.1));
        }
    }
}
