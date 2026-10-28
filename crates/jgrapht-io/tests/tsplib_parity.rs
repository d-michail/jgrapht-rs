// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

use jgrapht::{Graph, UGraph};
use jgrapht_io::tsplib::{TsplibImporter, TsplibMetadata, TsplibNode};
use jgrapht_io::{GraphImporter, IoError};

fn points_2d(kind: &str) -> String {
    format!(
        "TYPE : TSP\nDIMENSION : 4\nEDGE_WEIGHT_TYPE : {kind}\nNODE_COORD_SECTION\n\
         1 10.2 15.0\n2 14.2 15.0\n3  14.8    20.0\n4\t10.8\t\t20.0\nEOF\n"
    )
}

fn points_3d(kind: &str) -> String {
    format!(
        "TYPE : TSP\nDIMENSION : 4\nEDGE_WEIGHT_TYPE : {kind}\nNODE_COORD_SECTION\n\
         1 10.0 15.0 3.7\n2 14.0 15.0 3.7\n3 14.0 20.0 3.7\n4 14.0 20.0 3.7\n"
    )
}

fn import(input: &str) -> (UGraph<usize>, TsplibMetadata<usize>) {
    let mut graph = UGraph::<usize>::simple()
        .weighted()
        .with_vertex_factory(|id| id as usize);
    let mut importer = TsplibImporter::new();
    importer.import_str(&mut graph, input).unwrap();
    (graph, importer.metadata().unwrap().clone())
}

fn vertex_number(metadata: &TsplibMetadata<usize>, number: i32) -> usize {
    *metadata
        .nodes
        .iter()
        .find(|(_, node)| node.number == number)
        .unwrap()
        .0
}

fn assert_distances(input: &str, expected: [f64; 6], locations: bool, distances: bool) {
    let (graph, metadata) = import(input);
    assert_eq!(graph.vertex_count(), 4);
    assert_eq!(graph.edge_count(), 6);
    let pairs = [(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)];
    for ((source, target), expected) in pairs.into_iter().zip(expected) {
        let source = vertex_number(&metadata, source);
        let target = vertex_number(&metadata, target);
        let edge = graph.edge_between(&source, &target).unwrap();
        assert_eq!(graph.edge_weight(edge), Some(expected));
    }
    assert_eq!(metadata.has_distinct_node_locations(), locations);
    assert_eq!(metadata.has_distinct_neighbor_distances(), distances);
}

fn assert_import_error(input: &str, expected: &str) {
    let mut graph = UGraph::<usize>::simple()
        .weighted()
        .with_vertex_factory(|id| id as usize);
    let mut importer = TsplibImporter::new();
    let error = importer.import_str(&mut graph, input).unwrap_err();
    assert!(error.to_string().contains(expected), "{error}");
    assert!(importer.metadata().is_none());
}

#[test]
fn metadata_values() {
    let input = "NAME : theNameOfThisFile\n\
COMMENT : The first line of the comment\n\
COMMENT : A second line\n\
TYPE : TSP\n\
DIMENSION : 4\n\
EDGE_WEIGHT_TYPE : EUC_2D\n\
NODE_COORD_TYPE: THREED_COORDS\n\
CAPACITY : 7\n\
EDGE_WEIGHT_FORMAT: FULL_MATRIX\n\
EDGE_DATA_FORMAT: ADJ_LIST\n\
DISPLAY_DATA_TYPE  :   TWOD_DISPLAY\n\
NODE_COORD_SECTION\n\
1 10.2 15.0\n\
EOF\n";
    let (graph, metadata) = import(input);
    assert_eq!(graph.vertex_count(), 1);
    assert_eq!(metadata.name.as_deref(), Some("theNameOfThisFile"));
    assert_eq!(metadata.problem_type.as_deref(), Some("TSP"));
    assert_eq!(
        metadata.comments,
        ["The first line of the comment", "A second line"]
    );
    assert_eq!(metadata.dimension, 4);
    assert_eq!(metadata.capacity, Some(7));
    assert_eq!(metadata.edge_weight_type, "EUC_2D");
    assert_eq!(metadata.edge_weight_format.as_deref(), Some("FULL_MATRIX"));
    assert_eq!(metadata.edge_data_format.as_deref(), Some("ADJ_LIST"));
    assert_eq!(metadata.node_coord_type.as_deref(), Some("THREED_COORDS"));
    assert_eq!(metadata.display_data_type.as_deref(), Some("TWOD_DISPLAY"));
    assert!(metadata.has_distinct_node_locations());
    assert!(metadata.has_distinct_neighbor_distances());
}

#[test]
fn metadata_values_slightly_off_spec() {
    let input = "NAME : theNameOfThisFile\n\
TYPE : TSP (Some comment)\n\
DIMENSION : 4 (Comment: number of elements)\n\
EDGE_WEIGHT_TYPE : EUC_2D #use 2D edges\n\
NODE_COORD_TYPE: THREED_COORDS\n\
CAPACITY : 7 # Capacitated vehicle routing problem data\n\
EDGE_WEIGHT_FORMAT: FULL_MATRIX\n\
EDGE_DATA_FORMAT: ADJ_LIST\n\
DISPLAY_DATA_TYPE : TWOD_DISPLAY\n\
NODE_COORD_SECTION\n\
1   10.2\t15.0\n\
EOF\n";
    let (_, metadata) = import(input);
    assert_eq!(metadata.problem_type.as_deref(), Some("TSP"));
    assert_eq!(metadata.dimension, 4);
    assert_eq!(metadata.capacity, Some(7));
    assert_eq!(metadata.edge_weight_type, "EUC_2D");
}

#[test]
fn graph_with_node_coord_section() {
    let (graph, metadata) = import(&points_2d("EUC_2D"));
    assert_eq!(graph.vertex_count(), 4);
    assert_eq!(
        metadata.nodes.values().cloned().collect::<Vec<_>>(),
        vec![
            TsplibNode {
                number: 1,
                coordinates: vec![10.2, 15.0]
            },
            TsplibNode {
                number: 2,
                coordinates: vec![14.2, 15.0]
            },
            TsplibNode {
                number: 3,
                coordinates: vec![14.8, 20.0]
            },
            TsplibNode {
                number: 4,
                coordinates: vec![10.8, 20.0]
            },
        ]
    );
}

#[test]
fn graph_with_node_coord_and_tour_sections() {
    let input = "TYPE : TSP\nDIMENSION : 4\nEDGE_WEIGHT_TYPE : EUC_2D\n\
NODE_COORD_SECTION\n7 10.2 15.0\n2 14.2 15.0\n9 14.8 20.0\n4 10.8 20.0\n\
TOUR_SECTION\n9\n4\n7\n2\n-1\nEOF\n";
    let (_, metadata) = import(input);
    let tour_numbers: Vec<_> = metadata
        .tour
        .as_ref()
        .unwrap()
        .iter()
        .map(|vertex| metadata.nodes[vertex].number)
        .collect();
    assert_eq!(tour_numbers, [9, 4, 7, 2]);
}

#[test]
fn standalone_tour_section() {
    let (_, reference) = import(
        "DIMENSION : 4\nEDGE_WEIGHT_TYPE : EUC_2D\nNODE_COORD_SECTION\n\
         7 10.2 15.0\n2 14.2 15.0\n9 14.8 20.0\n4 10.8 20.0\n",
    );
    let input = "TYPE : TSP\nDIMENSION : 4\nEDGE_WEIGHT_TYPE : EUC_2D\n\
                 TOUR_SECTION\n9\n4\n7\n2\n-1\nEOF\n";
    let mut importer = TsplibImporter::new();
    let tour = importer
        .import_tour(&reference, input.as_bytes())
        .unwrap()
        .unwrap();
    let numbers: Vec<_> = tour
        .iter()
        .map(|vertex| reference.nodes[vertex].number)
        .collect();
    assert_eq!(numbers, [9, 4, 7, 2]);
    assert_eq!(importer.metadata().unwrap().nodes, reference.nodes);
}

#[test]
fn edge_weight_type_euc_2d() {
    assert_distances(&points_2d("EUC_2D"), [4., 7., 5., 5., 6., 4.], true, true);
}

#[test]
fn edge_weight_type_euc_3d() {
    assert_distances(&points_3d("EUC_3D"), [4., 6., 6., 5., 5., 0.], false, false);
}

#[test]
fn edge_weight_type_max_2d() {
    assert_distances(&points_2d("MAX_2D"), [4., 5., 5., 5., 5., 4.], true, false);
}

#[test]
fn edge_weight_type_max_3d() {
    assert_distances(&points_3d("MAX_3D"), [4., 5., 5., 5., 5., 0.], false, false);
}

#[test]
fn edge_weight_type_man_2d() {
    assert_distances(&points_2d("MAN_2D"), [4., 10., 6., 6., 8., 4.], true, true);
}

#[test]
fn edge_weight_type_man_3d() {
    assert_distances(&points_3d("MAN_3D"), [4., 9., 9., 5., 5., 0.], false, false);
}

#[test]
fn edge_weight_type_ceil_2d() {
    assert_distances(&points_2d("CEIL_2D"), [4., 7., 6., 6., 7., 4.], true, true);
}

#[test]
fn edge_weight_type_geo() {
    assert_distances(
        &points_2d("GEO"),
        [446., 727., 549., 541., 680., 446.],
        true,
        true,
    );
}

#[test]
fn geographical_distance_cases() {
    let node = |coordinates: &[f64]| TsplibNode {
        number: -1,
        coordinates: coordinates.to_vec(),
    };
    let cases = [
        ([0.0, 0.0], [0.0, 90.0], 10019),
        ([23.0, 15.0], [-23.0, 105.0], 10997),
        ([0.0, -90.2], [0.0, 89.8], 20038),
        ([20.0, -90.7], [-20.0, 89.3], 20038),
        ([20.0, -70.0], [-20.0, 110.0], 20038),
        ([40.48, -74.0], [52.3, 13.24], 6386),
        ([1.48, 113.24], [-6.36, -65.24], 19488),
    ];
    for (first, second, expected) in cases {
        let actual =
            TsplibImporter::<usize>::geographical_distance_2d(&node(&first), &node(&second))
                .unwrap();
        assert!((actual - expected).abs() <= 1, "{actual} != {expected}");
    }
}

#[test]
fn geographical_distance_needs_two_coordinates() {
    let node = |coordinates: &[f64]| TsplibNode {
        number: 7,
        coordinates: coordinates.to_vec(),
    };
    for (a, b) in [(&[][..], &[1.0, 2.0][..]), (&[1.0][..], &[1.0, 2.0][..])] {
        let error =
            TsplibImporter::<usize>::geographical_distance_2d(&node(a), &node(b)).unwrap_err();
        assert!(matches!(error, IoError::Import(_)), "{error}");
    }
}

#[test]
fn edge_weight_type_att() {
    assert_distances(&points_2d("ATT"), [2., 3., 2., 2., 2., 2.], true, false);
}

#[test]
fn rejects_unweighted_graph() {
    let mut graph = UGraph::<usize>::simple().with_vertex_factory(|id| id as usize);
    let mut importer = TsplibImporter::new();
    let error = importer.import_str(&mut graph, "").unwrap_err();
    assert!(matches!(error, IoError::Import(_)));
    assert!(error.to_string().contains("Graph must be weighted"));
}

#[test]
fn rejects_missing_value() {
    assert_import_error("NAME : ", "Missing value for key NAME");
}

#[test]
fn rejects_multiple_values() {
    assert_import_error("TYPE : TSP\nTYPE : TSP\n", "Multiple values for key TYPE");
}

#[test]
fn rejects_invalid_specification_value() {
    assert_import_error(
        "EDGE_WEIGHT_FORMAT : some_String\n",
        "Invalid EDGE_WEIGHT_FORMAT value <some_String>",
    );
}

#[test]
fn rejects_non_number_dimension() {
    assert_import_error(
        "DIMENSION : A_STRING\n",
        "Invalid DIMENSION integer value <A_STRING>",
    );
}

#[test]
fn rejects_unsupported_edge_weight_type() {
    assert_import_error(
        "EDGE_WEIGHT_TYPE : XRAY1\nDIMENSION:1\nNODE_COORD_SECTION\n1 10.2 15.0\n",
        "Unsupported EDGE_WEIGHT_TYPE <XRAY1>",
    );
}

#[test]
fn rejects_node_section_without_specification() {
    assert_import_error(
        "NODE_COORD_SECTION\n1 10.2 15.0\n2 14.2 15.0\n",
        "Missing data to read <NODE_COORD_SECTION>",
    );
}

#[test]
fn rejects_wrong_coordinate_count() {
    assert_import_error(
        "EDGE_WEIGHT_TYPE : EUC_3D\nDIMENSION: 1\nNODE_COORD_SECTION\n1 10.2 15.0\n",
        "Unexpected number of elements <3> in line: 1 10.2 15.0",
    );
}

#[test]
fn rejects_missing_vertex_in_standalone_tour() {
    let (_, reference) = import(&points_2d("EUC_2D"));
    let mut importer = TsplibImporter::new();
    let error = importer
        .import_tour(&reference, "DIMENSION: 1\nTOUR_SECTION\n8\n-1\n".as_bytes())
        .unwrap_err();
    assert!(error.to_string().contains("Missing vertex with number 8"));
    assert!(importer.metadata().is_none());
}
