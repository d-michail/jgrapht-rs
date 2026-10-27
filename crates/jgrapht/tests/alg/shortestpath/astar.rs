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

use jgrapht::alg::shortestpath::AStarShortestPath;
use jgrapht::{DiGraph, GraphMut, GraphMutExt, UGraph};
use std::collections::HashMap;

pub(super) type Node = (usize, usize);

pub(super) const LABYRINTH_1: [&str; 15] = [
    ". . . . . . . . . . . . . . . . . . . . . ####. . . . . . .",
    ". . . . . . . . . . . . . . . . . . . . . ####. . . . . . .",
    ". . . . . . . . . . . . . . . . . . . . . ####. . . . . . .",
    ". . . ####. . . . . . . . . . . . . . . . ####. . . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . ####T . . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . ##########. . . .",
    ". . . ####. . . . . . . . ####. . . . . . ##########. . . .",
    ". . . ####. . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . . . . . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . . . . . . . . . . . ####. . . . . . . . . . . . . . .",
    "S . . . . . . . . . . . . ####. . . . . . . . . . . . . . .",
];

pub(super) const LABYRINTH_2: [&str; 15] = [
    ". . . . . . . . . . . . . . . . . . . . . ####. . . . . . .",
    ". . . . . . . . . . . . . . . . . . . . . ####. . . . . . .",
    ". . . . . . . . . . . . . . . . . . . . . ####. . . . . . .",
    ". . . ####. . . . . . . . . . . . . . . . ####### . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . ####T## . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . ##########. . . .",
    ". . . ####. . . . . . . . ####. . . . . . ##########. . . .",
    ". . . ####. . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . ####. . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . . . . . . . . . . . ####. . . . . . . . . . . . . . .",
    ". . . . . . . . . . . . . ####. . . . . . . . . . . . . . .",
    "S . . . . . . . . . . . . ####. . . . . . . . . . . . . . .",
];

pub(super) fn labyrinth(rows: &[&str]) -> (UGraph<Node>, Node, Node) {
    let mut graph = UGraph::<Node>::simple().weighted();
    let mut nodes = HashMap::new();
    let mut source = None;
    let mut target = None;
    for (row, line) in rows.iter().enumerate() {
        for (column, character) in line.chars().enumerate() {
            if character == '#' || character == ' ' {
                continue;
            }
            let node = (row, column / 2);
            graph.add_vertex(node);
            nodes.insert((row, column), node);
            if character == 'S' {
                source = Some(node);
            } else if character == 'T' {
                target = Some(node);
            }
        }
    }
    for (&(row, column), &node) in &nodes {
        if let Some(right) = nodes.get(&(row, column + 2)) {
            graph.add_weighted_edge(&node, right, 1.0).unwrap();
        }
        if let Some(down) = nodes.get(&(row + 1, column)) {
            graph.add_weighted_edge(&node, down, 1.0).unwrap();
        }
    }
    (graph, source.unwrap(), target.unwrap())
}

pub(super) fn manhattan(source: &Node, target: &Node) -> f64 {
    (source.0.abs_diff(target.0) + source.1.abs_diff(target.1)) as f64
}

#[test]
fn test_labyrinth_1() {
    let (graph, source, target) = labyrinth(&LABYRINTH_1);
    let path = AStarShortestPath::new(&graph, manhattan)
        .path(&source, &target)
        .unwrap()
        .unwrap();
    assert_eq!(path.weight(), 47.0);
    assert_eq!(path.edge_list().len(), 47);
    assert_eq!(path.vertex_list().len(), 48);

    let euclidean = |source: &Node, target: &Node| {
        let x = source.0.abs_diff(target.0) as f64;
        let y = source.1.abs_diff(target.1) as f64;
        (x * x + y * y).sqrt()
    };
    let path = AStarShortestPath::new(&graph, euclidean)
        .path(&source, &target)
        .unwrap()
        .unwrap();
    assert_eq!(path.weight(), 47.0);
    assert_eq!(path.edge_list().len(), 47);
}

#[test]
fn test_labyrinth_2() {
    let (graph, source, target) = labyrinth(&LABYRINTH_2);
    assert!(
        AStarShortestPath::new(&graph, manhattan)
            .path(&source, &target)
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_multigraph() {
    let mut graph = UGraph::<Node>::multi().weighted();
    let n1 = (0, 0);
    let n2 = (1, 0);
    let n3 = (2, 0);
    graph.add_vertices([n1, n2, n3]);
    for (source, target, weight) in [
        (n1, n2, 5.0),
        (n1, n2, 4.0),
        (n1, n2, 8.0),
        (n2, n3, 7.0),
        (n2, n3, 9.0),
        (n2, n3, 2.0),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }

    let path = AStarShortestPath::new(&graph, manhattan)
        .path(&n1, &n3)
        .unwrap()
        .unwrap();
    assert_eq!(path.weight(), 6.0);
    assert_eq!(path.edge_list().len(), 2);
}

#[test]
fn test_inconsistent_heuristic() {
    let mut graph = DiGraph::<i32>::pseudo().weighted();
    graph.add_vertices(0..=3);
    for (source, target, weight) in [
        (0, 1, 0.5822723681370429),
        (0, 3, 0.8512429683406786),
        (3, 0, 0.22867383417976428),
        (1, 2, 0.1531858692059932),
        (3, 1, 0.9639222864568235),
        (2, 2, 0.23262564370920258),
        (2, 2, 0.6166416559599189),
        (3, 3, 0.6088954021459719),
        (3, 3, 0.2476189990121238),
    ] {
        graph.add_weighted_edge(&source, &target, weight).unwrap();
    }
    let heuristic = |source: &i32, target: &i32| match (*source, *target) {
        (0, 1) => 0.5822723681370429,
        (3, 1) => 0.8109462023168071,
        (3, 2) => 0.9639222864568235,
        (0, 2) => 0.7354582373430361,
        _ => 0.0,
    };

    let path = AStarShortestPath::new(&graph, heuristic)
        .path(&3, &2)
        .unwrap()
        .unwrap();
    assert!((path.weight() - 0.9641320715228003).abs() < 1e-9);
}
