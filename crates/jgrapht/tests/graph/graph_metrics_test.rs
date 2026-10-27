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

//! Port of `org.jgrapht.GraphMetricsTest`.

use jgrapht::alg::cycle::{DirectedSimpleCycles, JohnsonSimpleCycles};
use jgrapht::alg::metrics::*;
use jgrapht::generate::named::{self, build};
use jgrapht::generate::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

const EPSILON: f64 = 0.000000001;

fn generate_ints<G: GraphMut<Vertex = i32> + IncidenceGraph>(g: &mut G, gen_: &mut impl GraphGenerator) {
    gen_.generate(g, |i| i as i32).unwrap();
}

#[test]
fn test_graph_diameter() {
    let mut g = DiGraph::<i32>::simple().weighted();
    g.add_vertices([0, 1]);
    g.add_weighted_edge(&0, &1, 10.0).unwrap();
    g.add_weighted_edge(&1, &0, 12.0).unwrap();
    assert!((GraphMeasurer::new(&g).unwrap().diameter().unwrap() - 12.0).abs() < EPSILON);
}

#[test]
fn test_graph_radius() {
    let g = UGraph::<i32>::simple();
    assert!((GraphMeasurer::new(&g).unwrap().radius().unwrap() - 0.0).abs() < EPSILON);
}

#[test]
fn test_graph_girth_acyclic() {
    let mut tree = UGraph::<i32>::simple();
    tree.add_vertices(0..6);
    for (a, b) in [(0, 1), (0, 4), (0, 5), (1, 2), (1, 3)] {
        tree.add_edge(&a, &b).unwrap();
    }
    assert_eq!(girth(&tree), None);
}

#[test]
fn test_graph_directed_acyclic() {
    let mut tree = DiGraph::<i32>::simple();
    tree.add_vertices(0..4);
    for (a, b) in [(0, 1), (0, 2), (1, 3), (2, 3)] {
        tree.add_edge(&a, &b).unwrap();
    }
    assert_eq!(girth(&tree), None);
}

#[test]
fn test_graph_directed_cyclic() {
    let mut tree = DiGraph::<i32>::simple();
    tree.add_vertices(0..4);
    for (a, b) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
        tree.add_edge(&a, &b).unwrap();
    }
    assert_eq!(girth(&tree), Some(4));
}

#[test]
fn test_graph_directed_cyclic_2() {
    let mut tree = DiGraph::<i32>::simple();
    tree.add_vertices(0..2);
    tree.add_edge(&0, &1).unwrap();
    tree.add_edge(&1, &0).unwrap();
    assert_eq!(girth(&tree), Some(2));
}

#[test]
fn test_graph_girth_grid_graph() {
    let mut grid = UGraph::<i32>::simple();
    generate_ints(&mut grid, &mut Grid { rows: 3, cols: 4 });
    assert_eq!(girth(&grid), Some(4));
}

#[test]
fn test_graph_girth_ring_graph_even() {
    let mut ring = UGraph::<i32>::simple();
    generate_ints(&mut ring, &mut Ring(10));
    assert_eq!(girth(&ring), Some(10));
}

#[test]
fn test_graph_girth_ring_graph_odd() {
    let mut ring = UGraph::<i32>::simple();
    generate_ints(&mut ring, &mut Ring(9));
    assert_eq!(girth(&ring), Some(9));
}

#[test]
fn test_graph_girth_wheel_graph() {
    let mut g = UGraph::<i32>::simple();
    generate_ints(
        &mut g,
        &mut Wheel {
            n: 5,
            inward_spokes: false,
        },
    );
    assert_eq!(girth(&g), Some(3));
}

#[test]
fn test_graph_directed_1() {
    let mut graph = DiGraph::<i32>::simple();
    graph.add_vertices(0..4);
    for (a, b) in [(1, 0), (3, 0), (1, 2), (2, 3), (3, 2)] {
        graph.add_edge(&a, &b).unwrap();
    }
    assert_eq!(girth(&graph), Some(2));
}

fn pseudo_edges<D: jgrapht::graph::Direction>(graph: &mut BaseGraph<i32, DefaultEdge, D>) {
    graph.add_vertices(0..4);
    for (a, b) in [(0, 1), (0, 1), (1, 2), (2, 2), (2, 3), (3, 0)] {
        graph.add_edge(&a, &b).unwrap();
    }
}

#[test]
fn test_pseudo_graph_undirected() {
    let mut graph = UGraph::<i32>::pseudo();
    pseudo_edges(&mut graph);
    assert_eq!(girth(&graph), Some(1));
}

#[test]
fn test_pseudo_graph_directed() {
    let mut graph = DiGraph::<i32>::pseudo();
    pseudo_edges(&mut graph);
    assert_eq!(girth(&graph), Some(1));
}

#[test]
fn test_multi_graph_undirected() {
    let mut graph = UGraph::<i32>::multi();
    graph.add_vertices(0..4);
    for (a, b) in [(0, 1), (0, 1), (1, 2), (2, 3), (3, 0)] {
        graph.add_edge(&a, &b).unwrap();
    }
    assert_eq!(girth(&graph), Some(2));
}

#[test]
fn test_multi_graph_directed() {
    let mut graph = DiGraph::<i32>::multi();
    graph.add_vertices(0..4);
    for (a, b) in [(0, 1), (0, 1), (1, 2), (2, 3), (3, 0)] {
        graph.add_edge(&a, &b).unwrap();
    }
    assert_eq!(girth(&graph), Some(4));
}

#[test]
fn test_directed_graphs() {
    // reuse one generator (and its random stream) for ten graphs, like the Java test
    let mut gen_ = GnpRandomGraphGenerator::new(10, 0.55, StdRng::seed_from_u64(0), false).unwrap();
    for _ in 0..10 {
        let mut graph = DiGraph::<i32>::simple();
        generate_ints(&mut graph, &mut gen_);
        let min_cycle = JohnsonSimpleCycles::new(&graph)
            .unwrap()
            .find_simple_cycles()
            .unwrap()
            .iter()
            .map(Vec::len)
            .min();
        assert_eq!(min_cycle, girth(&graph));
    }
}

#[test]
fn test_count_triangles() {
    let mut random = StdRng::seed_from_u64(0x8888);
    for _ in 0..300 {
        let n = 20 + random.random_range(0..100);
        let mut graph = UGraph::<i32>::simple();
        let mut generator = BarabasiAlbertGraphGenerator::new(
            10 + random.random_range(0..10),
            1 + random.random_range(0..7),
            n,
            StdRng::seed_from_u64(random.random()),
        )
        .unwrap();
        generate_ints(&mut graph, &mut generator);
        let vs: Vec<i32> = graph.vertices().copied().collect();
        assert_eq!(
            naive_count_triangles(&graph, &vs),
            number_of_triangles(&graph)
        );
    }
}

#[test]
fn test_count_triangles_2() {
    let mut random = StdRng::seed_from_u64(0x8888);
    for _ in 0..100 {
        let n = 1 + random.random_range(0..100);
        let mut graph = UGraph::<i32>::simple();
        let mut generator =
            GnpRandomGraphGenerator::new(n, 0.55, StdRng::seed_from_u64(random.random()), false)
                .unwrap();
        generate_ints(&mut graph, &mut generator);
        let vs: Vec<i32> = graph.vertices().copied().collect();
        assert_eq!(
            naive_count_triangles(&graph, &vs),
            number_of_triangles(&graph)
        );
    }
}

fn clear<G: GraphMut>(g: &mut G) {
    let vs: Vec<_> = g.vertices().cloned().collect();
    for v in vs {
        g.remove_vertex(&v);
    }
}

#[test]
fn test_count_triangles_3() {
    let mut graph = UGraph::<i32>::simple();
    // Complete graph: expected (|V| choose 3)
    generate_ints(&mut graph, &mut Complete(50));
    let vs: Vec<i32> = graph.vertices().copied().collect();
    assert_eq!(number_of_triangles(&graph), 50 * 49 * 48 / 6);
    assert_eq!(naive_count_triangles(&graph, &vs), 50 * 49 * 48 / 6);

    // Wheel graph: expected |V|-1 triangles
    clear(&mut graph);
    generate_ints(
        &mut graph,
        &mut Wheel {
            n: 50,
            inward_spokes: false,
        },
    );
    let vs: Vec<i32> = graph.vertices().copied().collect();
    assert_eq!(number_of_triangles(&graph), 49);
    assert_eq!(naive_count_triangles(&graph, &vs), 49);

    // Named graphs
    for (structure, expected) in [
        (named::petersen(), 0),
        (named::diamond(), 2),
        (named::goldner_harary(), 25),
        (named::klein7_regular(), 56),
    ] {
        let graph = build(structure);
        let vs: Vec<usize> = graph.vertices().copied().collect();
        assert_eq!(number_of_triangles(&graph), expected);
        assert_eq!(naive_count_triangles(&graph, &vs), expected);
    }
}

#[test]
fn test_count_triangles_4() {
    let mut g = UGraph::<i32>::pseudo();
    g.add_vertices(0..25);
    let edges = [
        (0, 1),
        (1, 2),
        (0, 3),
        (1, 3),
        (2, 5),
        (3, 5),
        (4, 5),
        (1, 6),
        (2, 6),
        (1, 7),
        (2, 7),
        (3, 7),
        (4, 7),
        (1, 8),
        (2, 8),
        (2, 9),
        (1, 10),
        (7, 10),
        (1, 11),
        (2, 11),
        (2, 12),
        (3, 13),
        (4, 13),
        (1, 15),
        (6, 15),
        (9, 15),
        (1, 16),
        (4, 16),
        (11, 16),
        (1, 18),
        (2, 18),
        (1, 19),
        (3, 19),
        (6, 19),
        (1, 20),
        (2, 20),
        (2, 21),
        (3, 21),
        (3, 22),
        (5, 22),
        (10, 22),
        (3, 23),
        (19, 23),
        (1, 24),
        (2, 24),
    ];
    for (a, b) in edges {
        g.add_edge(&a, &b).unwrap();
    }
    let vs: Vec<i32> = g.vertices().copied().collect();
    assert_eq!(number_of_triangles(&g), naive_count_triangles(&g, &vs));
}

#[test]
fn test_multiple_edges() {
    let mut g = UGraph::<i32>::pseudo();
    for (a, b) in [(0, 1), (1, 2), (2, 0), (1, 3), (2, 3), (2, 1)] {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    assert_eq!(number_of_triangles(&g), 4);
}

#[test]
fn test_multiple_edges_2() {
    let mut g = UGraph::<i32>::pseudo();
    for (a, b) in [
        (0, 1),
        (1, 2),
        (2, 0),
        (1, 3),
        (2, 3),
        (2, 1),
        (0, 2),
        (0, 2),
    ] {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    assert_eq!(number_of_triangles(&g), 8);
}
