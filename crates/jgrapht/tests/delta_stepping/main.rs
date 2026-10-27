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

//! Port of `org.jgrapht.alg.shortestpath.DeltaSteppingShortestPathTest` for
//! `ParallelDeltaSteppingShortestPath`. Java passes a `ThreadPoolExecutor`; here the number of
//! worker threads is the parallelism argument. The comparable / non-comparable vertex tests
//! only select a bucket set type in Java and use any `Eq + Hash` vertex here. The random
//! graphs come from a small LCG instead of `GnmRandomGraphGenerator`, since the test compares
//! with Dijkstra and not with fixed numbers.

use jgrapht::alg::shortestpath::{
    Dijkstra, ParallelDeltaSteppingShortestPath, ShortestPathAlgorithm,
};
use jgrapht::*;

fn threads() -> usize {
    std::thread::available_parallelism().map_or(2, usize::from)
}

const S: &str = "S";
const T: &str = "T";
const Y: &str = "Y";
const X: &str = "X";
const Z: &str = "Z";

fn simple_graph() -> DiGraph<&'static str> {
    let mut g = DiGraph::<&str>::pseudo().weighted();
    g.add_vertices([S, T, Y, X, Z]);
    for (u, v, w) in [
        (S, T, 10.0),
        (S, Y, 5.0),
        (T, Y, 2.0),
        (T, X, 1.0),
        (Y, T, 3.0),
        (Y, Z, 2.0),
        (Y, X, 9.0),
        (X, Z, 4.0),
        (Z, X, 6.0),
        (Z, S, 7.0),
    ] {
        g.add_weighted_edge(&u, &v, w).unwrap();
    }
    g
}

fn delta_stepping<'g, G: Graph>(g: &'g G) -> ParallelDeltaSteppingShortestPath<'g, G> {
    ParallelDeltaSteppingShortestPath::with_parallelism(g, threads())
}

#[test]
fn test_empty_graph() {
    let mut g = DiGraph::<&str>::pseudo().weighted();
    g.add_vertex(S);
    delta_stepping(&g).single_source(&S).unwrap();
}

#[test]
fn test_negative_weight_edge() {
    let mut g = DiGraph::<&str>::pseudo().weighted();
    g.add_vertices([S, T]);
    g.add_weighted_edge(&S, &T, -10.0).unwrap();
    assert!(matches!(
        delta_stepping(&g).single_source(&S),
        Err(GraphError::InvalidWeight(_))
    ));
}

#[test]
fn test_negative_delta() {
    let g = simple_graph();
    assert!(ParallelDeltaSteppingShortestPath::with_delta(&g, -1.0, 2).is_err());
}

#[test]
fn test_line_graph() {
    for n in 2..10usize {
        let mut g = DiGraph::<usize>::simple().weighted();
        g.add_vertices(0..n);
        let mut edges = Vec::new();
        for i in 0..n - 1 {
            edges.push(g.add_weighted_edge(&i, &(i + 1), 1.0).unwrap());
        }
        let path = delta_stepping(&g).path(&0, &(n - 1)).unwrap().unwrap();
        assert!((path.weight - (n - 1) as f64).abs() < 1e-9);
        assert_eq!(path.vertices, (0..n).collect::<Vec<_>>());
        assert_eq!(path.edges.len(), edges.len());
    }
}

#[test]
fn test_supply_comparator() {
    let mut g = DiGraph::<&str>::pseudo().weighted();
    g.add_vertices(["v0", "v1", "v2"]);
    g.add_weighted_edge(&"v0", &"v1", 1.0).unwrap();
    g.add_weighted_edge(&"v1", &"v2", 1.0).unwrap();
    let path = delta_stepping(&g).path(&"v0", &"v2").unwrap().unwrap();
    assert!((path.weight - 2.0).abs() < 1e-9);
    assert_eq!(path.vertices, ["v0", "v1", "v2"]);
}

/// Vertices whose ordering is constant (`ComparableVertex`) or absent (`NonComparableVertex`).
#[test]
fn test_comparable_and_non_comparable_vertices() {
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    struct V(u8);
    let mut g = DiGraph::<V>::pseudo().weighted();
    g.add_vertices([V(1), V(2)]);
    g.add_weighted_edge(&V(1), &V(2), 1.0).unwrap();
    let path = delta_stepping(&g).path(&V(1), &V(2)).unwrap().unwrap();
    assert!((path.weight - 1.0).abs() < 1e-9);
    assert_eq!(path.vertices, [V(1), V(2)]);
}

#[test]
fn test_get_path() {
    let g = simple_graph();
    let vertices = |t| delta_stepping(&g).path(&S, &t).unwrap().unwrap().vertices;
    assert_eq!(vertices(S), [S]);
    assert_eq!(vertices(T), [S, Y, T]);
    assert_eq!(vertices(X), [S, Y, T, X]);
    assert_eq!(vertices(Y), [S, Y]);
    assert_eq!(vertices(Z), [S, Y, Z]);
}

#[test]
fn test_get_paths_1() {
    let g = simple_graph();
    let variants = [
        ParallelDeltaSteppingShortestPath::with_delta(&g, 0.999, threads()).unwrap(),
        ParallelDeltaSteppingShortestPath::with_delta(&g, 5.0, threads()).unwrap(),
        ParallelDeltaSteppingShortestPath::with_delta(&g, 11.0, threads()).unwrap(),
        delta_stepping(&g),
    ];
    for algorithm in variants {
        let paths = algorithm.single_source(&S).unwrap();
        for (v, expected) in [(S, 0.0), (T, 8.0), (Y, 5.0), (X, 9.0), (Z, 7.0)] {
            assert!((paths.dist[&v] - expected).abs() < 1e-9, "{v}");
        }
    }
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % bound
    }
}

#[test]
fn test_get_paths_2() {
    let (n, degree, iterations) = (100usize, 50usize, 30);
    let mut rng = Lcg(17);
    for _ in 0..iterations {
        let mut g = DiGraph::<usize>::pseudo().weighted();
        g.add_vertices(0..n);
        for _ in 0..degree * n - n + 1 {
            let (u, v) = (rng.next(n as u64) as usize, rng.next(n as u64) as usize);
            g.add_weighted_edge(&u, &v, rng.next(1 << 20) as f64 / (1 << 20) as f64)
                .unwrap();
        }
        for i in 0..n - 1 {
            g.add_weighted_edge(&i, &(i + 1), rng.next(1 << 20) as f64 / (1 << 20) as f64)
                .unwrap();
        }
        let expected = Dijkstra::new(&g).single_source(&0).unwrap();
        let actual = delta_stepping(&g).single_source(&0).unwrap();
        for sink in 0..n {
            match (expected.path(&sink), actual.path(&sink)) {
                (None, None) => {}
                (Some(a), Some(b)) => assert!((a.weight - b.weight).abs() < 1e-9),
                (a, b) => panic!("reachability differs for {sink}: {a:?} vs {b:?}"),
            }
        }
    }
}

#[test]
fn test_single_thread_matches_many_threads() {
    let g = simple_graph();
    let one = ParallelDeltaSteppingShortestPath::with_parallelism(&g, 1)
        .single_source(&S)
        .unwrap();
    let many = ParallelDeltaSteppingShortestPath::with_parallelism(&g, 8)
        .single_source(&S)
        .unwrap();
    assert_eq!(one.dist, many.dist);
}
