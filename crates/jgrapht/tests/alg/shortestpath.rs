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

use jgrapht::alg::shortestpath::*;
use jgrapht::generate::{GnmRandomGraphGenerator, GraphGenerator};
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};
use rheaps::tree::FibonacciHeap;

#[path = "shortestpath/advanced_speedups.rs"]
mod advanced_speedups;
#[path = "shortestpath/all_directed_paths.rs"]
mod all_directed_paths;
#[path = "shortestpath/astar.rs"]
mod astar;
#[path = "shortestpath/basic_classes.rs"]
mod basic_classes;
#[path = "shortestpath/bellman_ford.rs"]
mod bellman_ford;
#[path = "shortestpath/bidirectional_astar.rs"]
mod bidirectional_astar;
#[path = "shortestpath/bidirectional_dijkstra.rs"]
mod bidirectional_dijkstra;
#[path = "shortestpath/bounded_pruned_yen.rs"]
mod bounded_pruned_yen;
#[path = "shortestpath/contraction_hierarchy.rs"]
mod contraction_hierarchy;
#[path = "shortestpath/dag_all_paths_counter.rs"]
mod dag_all_paths_counter;
#[path = "shortestpath/dijkstra.rs"]
mod dijkstra;
#[path = "shortestpath/disjoint_paths.rs"]
mod disjoint_paths;
#[path = "shortestpath/eppstein.rs"]
mod eppstein;
#[path = "shortestpath/floyd_warshall.rs"]
mod floyd_warshall;
#[path = "shortestpath/foundational_classes.rs"]
mod foundational_classes;
#[path = "shortestpath/incidence_errors.rs"]
mod incidence_errors;
#[path = "shortestpath/int_vertex_dijkstra.rs"]
mod int_vertex_dijkstra;
#[path = "shortestpath/johnson.rs"]
mod johnson;
#[path = "shortestpath/many_to_many.rs"]
mod many_to_many;
#[path = "shortestpath/multi_objective.rs"]
mod multi_objective;
#[path = "shortestpath/path_results.rs"]
mod path_results;
#[path = "shortestpath/ported_classes.rs"]
mod ported_classes;
#[path = "shortestpath/transit_node_routing.rs"]
mod transit_node_routing;
#[path = "shortestpath/yen.rs"]
mod yen;

fn weighted_di(edges: &[(u32, u32, f64)]) -> DiGraph<u32> {
    let mut g = DiGraph::<u32>::simple().weighted();
    for &(a, b, w) in edges {
        g.add_weighted_edge_with_vertices(a, b, w);
    }
    g
}

trait AddW {
    fn add_weighted_edge_with_vertices(&mut self, a: u32, b: u32, w: f64);
}
impl<D: jgrapht::graph::Direction> AddW for BaseGraph<u32, DefaultEdge, D> {
    fn add_weighted_edge_with_vertices(&mut self, a: u32, b: u32, w: f64) {
        self.add_vertex(a);
        self.add_vertex(b);
        self.add_weighted_edge(&a, &b, w).unwrap();
    }
}

fn sample() -> DiGraph<u32> {
    weighted_di(&[
        (1, 2, 7.0),
        (1, 3, 9.0),
        (1, 6, 14.0),
        (2, 3, 10.0),
        (2, 4, 15.0),
        (3, 4, 11.0),
        (3, 6, 2.0),
        (4, 5, 6.0),
        (6, 5, 9.0),
    ])
}

#[test]
fn dijkstra_classic() {
    let g = sample();
    let p = Dijkstra::new(&g).path(&1, &5).unwrap().unwrap();
    assert_eq!(p.vertices, [1, 3, 6, 5]);
    assert_eq!(p.weight, 20.0);
    assert!(p.is_valid(&g));
    assert_eq!(Dijkstra::new(&g).path(&5, &1).unwrap(), None);
    assert_eq!(
        Dijkstra::new(&g).path_weight(&5, &1).unwrap(),
        f64::INFINITY
    );
    let ss = Dijkstra::<_, FibonacciHeap<_, _>>::with_heap(&g)
        .single_source(&1)
        .unwrap();
    assert_eq!(ss.weight(&4), 20.0);
    assert!(
        Dijkstra::new(&g)
            .radius(10.0)
            .single_source(&1)
            .unwrap()
            .is_reachable(&3)
    );
    assert!(
        !Dijkstra::new(&g)
            .radius(10.0)
            .single_source(&1)
            .unwrap()
            .is_reachable(&5)
    );
}

#[test]
fn negative_weights() {
    let g = weighted_di(&[(1, 2, 4.0), (1, 3, 5.0), (3, 2, -3.0)]);
    assert!(Dijkstra::new(&g).single_source(&1).is_err());
    assert_eq!(BellmanFord::new(&g).path_weight(&1, &2).unwrap(), 2.0);
    let cyc = weighted_di(&[(1, 2, 1.0), (2, 3, -2.0), (3, 1, 0.5)]);
    assert_eq!(
        BellmanFord::new(&cyc).single_source(&1).err(),
        Some(GraphError::NegativeCycle)
    );
    assert_eq!(floyd_warshall(&cyc).err(), Some(GraphError::NegativeCycle));
    assert_eq!(johnson(&cyc).err(), Some(GraphError::NegativeCycle));
    let ap = johnson(&g).unwrap();
    assert_eq!(ap.weight(&1, &2), 2.0);
    assert_eq!(ap.path(&1, &2).unwrap().vertices, [1, 3, 2]);
}

#[test]
fn bfs_and_astar() {
    let g = sample();
    assert_eq!(BfsShortestPath(&g).path_weight(&1, &5).unwrap(), 2.0);
    let a = AStar::new(&g, |_, _| 0.0).path(&1, &5).unwrap().unwrap();
    assert_eq!(a.weight, 20.0);
    // grid with manhattan heuristic
    let mut grid = UGraph::<usize>::simple();
    jgrapht::generate::Grid { rows: 10, cols: 10 }
        .generate_indexed(&mut grid)
        .unwrap();
    let h = |a: &usize, b: &usize| ((a / 10).abs_diff(b / 10) + (a % 10).abs_diff(b % 10)) as f64;
    assert_eq!(
        AStar::new(&grid, h).path(&0, &99).unwrap().unwrap().weight,
        18.0
    );
}

#[test]
fn yen() {
    let g = weighted_di(&[
        (1, 2, 3.0),
        (1, 3, 2.0),
        (2, 4, 4.0),
        (3, 2, 1.0),
        (3, 4, 2.0),
        (3, 5, 3.0),
        (4, 5, 2.0),
        (4, 6, 1.0),
        (5, 6, 2.0),
    ]);
    let ps = yen_k_shortest_paths(&g, &1, &6, 3).unwrap();
    let ws: Vec<_> = ps.iter().map(|p| p.weight).collect();
    assert_eq!(ws, [5.0, 7.0, 8.0]);
    assert!(ps.iter().all(|p| p.is_valid(&g)));
    assert_eq!(yen_k_shortest_paths(&g, &1, &6, 100).unwrap().len(), 7);
}

#[test]
fn algorithms_agree_on_random_graphs() {
    let mut rng = StdRng::seed_from_u64(7);
    for _ in 0..10 {
        let mut g = DiGraph::<usize>::simple().weighted();
        GnmRandomGraphGenerator::new(25, 90, StdRng::seed_from_u64(rng.random()), false, false)
            .unwrap()
            .generate_indexed(&mut g)
            .unwrap();
        let es: Vec<_> = g.edges().cloned().collect();
        for e in es {
            g.set_edge_weight(&e, rng.random_range(0..20) as f64)
                .unwrap();
        }
        let fw = floyd_warshall(&g).unwrap();
        let jo = johnson(&g).unwrap();
        for s in 0..25 {
            let d = Dijkstra::new(&g).single_source(&s).unwrap();
            let b = BellmanFord::new(&g).single_source(&s).unwrap();
            for t in 0..25 {
                let w = d.weight(&t);
                assert_eq!(w, b.weight(&t));
                assert_eq!(w, fw.weight(&s, &t));
                assert_eq!(w, jo.weight(&s, &t));
                if let Some(p) = fw.path(&s, &t) {
                    assert!(p.is_valid(&g), "{p:?}");
                }
            }
        }
    }
}

#[test]
fn bidirectional_and_delta_stepping() {
    let g = sample();
    let p = BidirectionalDijkstra(&g).path(&1, &5).unwrap().unwrap();
    assert_eq!((p.weight, p.vertices.clone()), (20.0, vec![1, 3, 6, 5]));
    assert!(p.is_valid(&g));
    assert_eq!(BidirectionalDijkstra(&g).path(&5, &1).unwrap(), None);
    assert_eq!(
        BidirectionalDijkstra(&g)
            .path(&2, &2)
            .unwrap()
            .unwrap()
            .length(),
        0
    );

    let mut rng = StdRng::seed_from_u64(11);
    for round in 0..10 {
        let seed: u64 = rng.random();
        let mut weights = StdRng::seed_from_u64(seed ^ 1);
        macro_rules! check {
            ($g:expr) => {{
                let mut g = $g;
                GnmRandomGraphGenerator::new(30, 120, StdRng::seed_from_u64(seed), false, false)
                    .unwrap()
                    .generate_indexed(&mut g)
                    .unwrap();
                let es: Vec<_> = g.edges().cloned().collect();
                for e in es {
                    g.set_edge_weight(&e, weights.random_range(1..30) as f64)
                        .unwrap();
                }
                for s in [0usize, 7, 19] {
                    let reference = Dijkstra::new(&g).single_source(&s).unwrap();
                    for delta in [None, Some(3.0), Some(100.0)] {
                        let ds = match delta {
                            None => DeltaStepping::new(&g).single_source(&s),
                            Some(d) => DeltaStepping::with_delta(&g, d).single_source(&s),
                        }
                        .unwrap();
                        for t in 0..30 {
                            assert_eq!(
                                ds.weight(&t),
                                reference.weight(&t),
                                "round {round} delta {delta:?}"
                            );
                            if let Some(p) = ds.path(&t) {
                                assert!(p.is_valid(&g));
                            }
                        }
                    }
                    for t in 0..30 {
                        let bi = BidirectionalDijkstra(&g).path(&s, &t).unwrap();
                        assert_eq!(
                            bi.as_ref().map(|p| p.weight),
                            reference.path(&t).map(|p| p.weight)
                        );
                        if let Some(p) = bi {
                            assert!(p.is_valid(&g));
                        }
                    }
                }
            }};
        }
        check!(DiGraph::<usize>::simple().weighted());
        check!(UGraph::<usize>::simple().weighted());
    }
}
