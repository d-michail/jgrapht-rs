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

//! Port of `org.jgrapht.GraphTestsTest`.

use jgrapht::generate::named::{self, build};
use jgrapht::generate::{Complete, GraphGenerator, Star};
use jgrapht::graph::props::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

fn get_graph(edges: &[(i32, i32)]) -> UGraph<i32> {
    let mut g = UGraph::<i32>::with_loops();
    for &(a, b) in edges {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    g
}

/// Generates into `g`, drawing fresh integer vertices from `counter` (like an integer supplier).
fn generate_into<G: GraphMut<Vertex = i32> + IncidenceGraph>(
    g: &mut G,
    gen_: &mut impl GraphGenerator,
    counter: &mut i32,
) {
    gen_.generate(g, |_| {
        let v = *counter;
        *counter += 1;
        v
    })
    .unwrap();
}

#[test]
fn test_is_empty() {
    let mut g = DiGraph::<i32>::with_loops();
    assert!(is_empty(&g));
    g.add_vertex(1);
    assert!(is_empty(&g));
    g.add_vertex(2);
    assert!(is_empty(&g));
    let e = g.add_edge(&1, &2).unwrap().unwrap();
    assert!(!is_empty(&g));
    g.remove_edge(&e);
    assert!(is_empty(&g));
}

#[test]
fn test_is_simple() {
    let g1 = DiGraph::<i32>::with_loops();
    assert!(is_simple(&g1));
    let g2 = UGraph::<i32>::simple();
    assert!(is_simple(&g2));

    let mut g3 = DiGraph::<i32>::pseudo();
    assert!(is_simple(&g3));
    g3.add_vertices([1, 2]);
    g3.add_edge(&1, &2).unwrap();
    g3.add_edge(&2, &1).unwrap();
    assert!(is_simple(&g3));
    let g3e11 = g3.add_edge(&1, &1).unwrap().unwrap();
    assert!(!is_simple(&g3));
    g3.remove_edge(&g3e11);
    assert!(is_simple(&g3));
    g3.add_edge(&2, &1).unwrap();
    assert!(!is_simple(&g3));

    let mut g4 = UGraph::<i32>::pseudo();
    g4.add_vertices([1, 2]);
    assert!(is_simple(&g4));
    let g4e12 = g4.add_edge(&1, &2).unwrap().unwrap();
    g4.add_edge(&2, &1).unwrap();
    assert!(!is_simple(&g4));
    g4.remove_edge(&g4e12);
    assert!(is_simple(&g4));
    g4.add_edge(&1, &1).unwrap();
    assert!(!is_simple(&g4));
}

#[test]
fn test_has_self_loops() {
    assert!(!has_self_loops(&DiGraph::<i32>::with_loops()));
    assert!(!has_self_loops(&UGraph::<i32>::simple()));

    let mut g3 = DiGraph::<i32>::pseudo();
    assert!(!has_self_loops(&g3));
    g3.add_vertices([1, 2]);
    g3.add_edge(&1, &2).unwrap();
    g3.add_edge(&2, &1).unwrap();
    assert!(!has_self_loops(&g3));
    g3.add_edge(&2, &2).unwrap();
    assert!(has_self_loops(&g3));

    let mut g4 = UGraph::<i32>::pseudo();
    g4.add_vertices([1, 2]);
    g4.add_edge(&1, &2).unwrap();
    g4.add_edge(&2, &1).unwrap();
    assert!(!has_self_loops(&g4));
    g4.add_edge(&2, &2).unwrap();
    assert!(has_self_loops(&g4));
}

#[test]
fn test_has_multiple_edges() {
    assert!(!has_multiple_edges(&DiGraph::<i32>::with_loops()));
    assert!(!has_multiple_edges(&UGraph::<i32>::simple()));

    let mut g3 = DiGraph::<i32>::pseudo();
    assert!(!has_multiple_edges(&g3));
    g3.add_vertices([1, 2]);
    g3.add_edge(&1, &2).unwrap();
    g3.add_edge(&2, &1).unwrap();
    g3.add_edge(&1, &1).unwrap();
    assert!(!has_multiple_edges(&g3));
    g3.add_edge(&2, &2).unwrap();
    assert!(!has_multiple_edges(&g3));
    g3.add_edge(&2, &1).unwrap();
    assert!(has_multiple_edges(&g3));

    let mut g4 = UGraph::<i32>::pseudo();
    g4.add_vertices([1, 2]);
    g4.add_edge(&1, &2).unwrap();
    g4.add_edge(&1, &1).unwrap();
    assert!(!has_multiple_edges(&g4));
    g4.add_edge(&2, &1).unwrap();
    assert!(has_multiple_edges(&g4));

    let mut g5 = UGraph::<i32>::pseudo();
    g5.add_vertices([1, 2]);
    g5.add_edge(&1, &2).unwrap();
    g5.add_edge(&1, &1).unwrap();
    assert!(!has_multiple_edges(&g5));
    g5.add_edge(&1, &1).unwrap();
    assert!(has_multiple_edges(&g5));
}

#[test]
fn test_is_complete_directed() {
    let mut g = DiGraph::<i32>::with_loops();
    assert!(is_complete(&g));
    g.add_vertex(1);
    assert!(is_complete(&g));
    g.add_vertex(2);
    assert!(!is_complete(&g));
    g.add_edge(&1, &2).unwrap();
    assert!(!is_complete(&g));
    g.add_edge(&2, &1).unwrap();
    assert!(is_complete(&g));
    g.add_vertex(3);
    assert!(!is_complete(&g));
    g.add_edge(&1, &3).unwrap();
    assert!(!is_complete(&g));
    g.add_edge(&3, &1).unwrap();
    assert!(!is_complete(&g));
    g.add_edge(&2, &3).unwrap();
    assert!(!is_complete(&g));
    g.add_edge(&3, &2).unwrap();
    assert!(is_complete(&g));

    // check loops
    let mut g1 = DiGraph::<i32>::pseudo();
    assert!(is_complete(&g1));
    g1.add_vertex(1);
    assert!(is_complete(&g1));
    g1.add_vertex(2);
    assert!(!is_complete(&g1));
    g1.add_edge(&1, &1).unwrap();
    g1.add_edge(&2, &2).unwrap();
    assert!(!is_complete(&g1));

    // check multiple edges
    let mut g2 = DiGraph::<i32>::pseudo();
    assert!(is_complete(&g2));
    g2.add_vertices([1, 2, 3]);
    assert!(!is_complete(&g2));
    g2.add_edge(&1, &2).unwrap();
    g2.add_edge(&1, &3).unwrap();
    g2.add_edge(&2, &3).unwrap();
    g2.add_edge(&1, &1).unwrap();
    g2.add_edge(&2, &2).unwrap();
    g2.add_edge(&3, &3).unwrap();
    assert!(!is_complete(&g2));
}

#[test]
fn test_is_complete_undirected() {
    let mut g = UGraph::<i32>::simple();
    assert!(is_complete(&g));
    g.add_vertex(1);
    assert!(is_complete(&g));
    g.add_vertex(2);
    assert!(!is_complete(&g));
    g.add_edge(&1, &2).unwrap();
    assert!(is_complete(&g));
    g.add_vertex(3);
    assert!(!is_complete(&g));
    g.add_edge(&1, &3).unwrap();
    assert!(!is_complete(&g));
    g.add_edge(&2, &3).unwrap();
    assert!(is_complete(&g));

    let mut g1 = UGraph::<i32>::pseudo();
    assert!(is_complete(&g1));
    g1.add_vertex(1);
    assert!(is_complete(&g1));
    g1.add_vertex(2);
    assert!(!is_complete(&g1));
    g1.add_edge(&1, &1).unwrap();
    assert!(!is_complete(&g1));

    let mut g2 = UGraph::<i32>::pseudo();
    assert!(is_complete(&g2));
    g2.add_vertex(1);
    assert!(is_complete(&g2));
    g2.add_vertex(2);
    assert!(!is_complete(&g2));
    g2.add_edge(&1, &2).unwrap();
    assert!(is_complete(&g2));
    g2.add_edge(&1, &2).unwrap();
    assert!(!is_complete(&g2));
    g2.add_vertex(3);
    g2.add_edge(&1, &3).unwrap();
    assert!(!is_complete(&g2));
}

#[test]
fn test_is_connected_undirected() {
    let mut g = UGraph::<i32>::simple();
    assert!(!is_connected(&g));
    g.add_vertex(1);
    assert!(is_connected(&g));
    g.add_vertex(2);
    assert!(!is_connected(&g));
    g.add_edge(&1, &2).unwrap();
    assert!(is_connected(&g));
    g.add_vertex(3);
    assert!(!is_connected(&g));
    g.add_edge(&1, &3).unwrap();
    assert!(is_connected(&g));
}

#[test]
fn test_is_connected_directed() {
    let mut g = DiGraph::<i32>::simple();
    assert!(!is_weakly_connected(&g));
    assert!(!is_strongly_connected(&g));
    g.add_vertex(1);
    assert!(is_weakly_connected(&g));
    assert!(is_strongly_connected(&g));
    g.add_vertex(2);
    assert!(!is_weakly_connected(&g));
    assert!(!is_strongly_connected(&g));
    g.add_edge(&1, &2).unwrap();
    assert!(is_weakly_connected(&g));
    assert!(!is_strongly_connected(&g));
    g.add_vertex(3);
    assert!(!is_weakly_connected(&g));
    assert!(!is_strongly_connected(&g));
    g.add_edge(&2, &3).unwrap();
    assert!(is_weakly_connected(&g));
    assert!(!is_strongly_connected(&g));
    g.add_edge(&3, &1).unwrap();
    assert!(is_weakly_connected(&g));
    assert!(is_strongly_connected(&g));
}

#[test]
fn test_is_tree() {
    let mut g = UGraph::<i32>::pseudo();
    assert!(!is_tree(&g));
    g.add_vertex(1);
    assert!(is_tree(&g));
    g.add_vertex(2);
    assert!(!is_tree(&g));
    g.add_edge(&1, &2).unwrap();
    assert!(is_tree(&g));
    g.add_vertex(3);
    assert!(!is_tree(&g));
    g.add_edge(&1, &3).unwrap();
    assert!(is_tree(&g));
    g.add_edge(&2, &3).unwrap();
    assert!(!is_tree(&g));

    // disconnected but with correct number of edges
    let mut g1 = UGraph::<i32>::pseudo();
    assert!(!is_tree(&g1));
    g1.add_vertex(1);
    g1.add_vertex(2);
    g.add_edge(&1, &1).unwrap();
    assert!(!is_tree(&g1));
}

#[test]
fn test_is_forest_1() {
    let mut g = UGraph::<i32>::pseudo();
    assert!(!is_forest(&g));
    g.add_vertex(1);
    assert!(is_forest(&g));
    g.add_vertex(2);
    assert!(is_forest(&g));
    g.add_edge(&1, &2).unwrap();
    assert!(is_forest(&g));
    g.add_edge(&1, &2).unwrap();
    assert!(!is_forest(&g));
}

#[test]
fn test_is_forest_2() {
    let mut g = UGraph::<i32>::pseudo();
    let mut counter = 0;
    let mut gen_ = Star(10);
    generate_into(&mut g, &mut gen_, &mut counter);
    generate_into(&mut g, &mut gen_, &mut counter);
    assert!(is_forest(&g));
}

#[test]
fn test_is_overfull() {
    assert!(!is_overfull(&build(named::claw())));
    assert!(is_overfull(&build(named::doyle())));

    let mut k6 = UGraph::<i32>::pseudo();
    generate_into(&mut k6, &mut Complete(6), &mut 0);
    assert!(!is_overfull(&k6));

    let mut k7 = UGraph::<i32>::pseudo();
    generate_into(&mut k7, &mut Complete(7), &mut 0);
    assert!(is_overfull(&k7));
}

#[test]
fn is_split_1() {
    assert!(!is_split(&build(named::petersen())));
    let mut g = UGraph::<i32>::pseudo();
    assert!(!is_split(&g));
    g.add_vertex(0);
    assert!(is_split(&g));
    g.add_vertices([1, 2, 3, 4]);
    // clique
    g.add_edge(&0, &1).unwrap();
    g.add_edge(&1, &2).unwrap();
    g.add_edge(&2, &0).unwrap();
    // independent set
    g.add_edge(&3, &1).unwrap();
    g.add_edge(&3, &2).unwrap();
    g.add_edge(&4, &1).unwrap();
    assert!(is_split(&g));
    g.add_edge(&3, &4).unwrap();
    assert!(is_split(&g));
}

#[test]
fn is_split_2() {
    // Create some random split graphs.
    let mut rng = StdRng::seed_from_u64(0);
    for _ in 0..5 {
        // 1. create a clique
        let mut g = UGraph::<i32>::simple();
        generate_into(&mut g, &mut Complete(6), &mut 0);
        // 2. add vertices (the independent set) and connect some of them to the clique
        for j in 6..12 {
            g.add_vertex(j);
            for i in 0..6 {
                if rng.random_bool(0.5) {
                    g.add_edge(&i, &j).unwrap();
                }
            }
        }
        assert!(is_split(&g));
    }
}

#[test]
fn test_is_cubic() {
    assert!(is_cubic(&build(named::petersen())));
    let mut triangle = UGraph::<i32>::simple();
    triangle.add_edge_with_vertices(1, 2).unwrap();
    triangle.add_edge_with_vertices(2, 3).unwrap();
    triangle.add_edge_with_vertices(3, 1).unwrap();
    assert!(!is_cubic(&triangle));
}

#[test]
fn test_is_chordal() {
    let mut graph = UGraph::<i32>::with_loops();
    for (a, b) in [(1, 2), (2, 3), (3, 4), (4, 5), (5, 1), (1, 3)] {
        graph.add_edge_with_vertices(a, b).unwrap();
    }
    assert!(!is_chordal(&graph));
    graph.add_edge_with_vertices(1, 4).unwrap();
    assert!(is_chordal(&graph));
}

#[test]
fn test_is_weakly_chordal() {
    let mut graph = UGraph::<i32>::with_loops();
    for (a, b) in [(1, 2), (2, 3), (3, 4), (4, 5), (5, 1)] {
        graph.add_edge_with_vertices(a, b).unwrap();
    }
    assert!(!is_weakly_chordal(&graph));
    graph.add_edge_with_vertices(1, 3).unwrap();
    assert!(is_weakly_chordal(&graph));
}

/// Full graph on 4 vertices (every graph on less than 5 is planar)
#[test]
fn test_is_planar_1() {
    let graph = get_graph(&[(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)]);
    assert!(is_planar(&graph));
}

/// K(3,3)
#[test]
fn test_is_planar_2() {
    let graph = get_graph(&[
        (1, 4),
        (1, 5),
        (1, 6),
        (2, 4),
        (2, 5),
        (2, 6),
        (3, 4),
        (3, 5),
        (3, 6),
    ]);
    assert!(!is_planar(&graph));
}

/// K5
#[test]
fn test_is_planar_3() {
    let graph = get_graph(&[
        (1, 2),
        (1, 3),
        (1, 4),
        (1, 5),
        (2, 3),
        (2, 4),
        (2, 5),
        (3, 4),
        (3, 5),
        (4, 5),
    ]);
    assert!(!is_planar(&graph));
}

#[test]
fn test_is_k33_subdivision_1() {
    let graph = get_graph(&[
        (1, 4),
        (1, 5),
        (1, 6),
        (2, 4),
        (2, 5),
        (2, 6),
        (3, 4),
        (3, 5),
        (3, 6),
    ]);
    assert!(is_kuratowski_subdivision(&graph));
    assert!(is_k33_subdivision(&graph));
}

#[test]
fn test_is_k33_subdivision_2() {
    let graph = get_graph(&[
        (1, 5),
        (1, 6),
        (2, 4),
        (2, 6),
        (3, 4),
        (3, 5),
        (1, 7),
        (7, 4),
        (2, 8),
        (8, 5),
        (3, 9),
        (9, 6),
    ]);
    assert!(is_kuratowski_subdivision(&graph));
    assert!(is_k33_subdivision(&graph));
}

#[test]
fn test_is_k5_subdivision_1() {
    let graph = get_graph(&[
        (1, 2),
        (1, 3),
        (1, 4),
        (1, 5),
        (2, 3),
        (2, 4),
        (2, 5),
        (3, 4),
        (3, 5),
        (4, 5),
    ]);
    assert!(is_kuratowski_subdivision(&graph));
    assert!(is_k5_subdivision(&graph));
}

#[test]
fn test_is_k5_subdivision_2() {
    let graph = get_graph(&[
        (1, 2),
        (2, 3),
        (3, 4),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 8),
        (8, 9),
        (9, 10),
        (10, 1),
        (1, 5),
        (1, 7),
        (3, 7),
        (3, 9),
        (5, 9),
    ]);
    assert!(is_kuratowski_subdivision(&graph));
    assert!(is_k5_subdivision(&graph));
}

#[test]
fn fail_require_is_weighted_on_unweighted_graph() {
    let graph = DiGraph::<String>::with_loops();
    assert_eq!(
        require_weighted(&graph).err(),
        Some(GraphError::InvalidGraph(jgrapht::GraphProperty::Weighted))
    );
}

#[test]
fn test_require_is_weighted() {
    let graph = UGraph::<i32>::with_loops().weighted();
    assert!(std::ptr::eq(require_weighted(&graph).unwrap(), &graph));
}
