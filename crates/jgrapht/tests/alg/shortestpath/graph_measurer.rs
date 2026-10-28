use jgrapht::GraphExt;
use jgrapht::util::java::RandomSource;
use rand::{SeedableRng, rngs::StdRng};
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

use jgrapht::alg::metrics::GraphMeasurer;
use jgrapht::alg::shortestpath::{FloydWarshallShortestPaths, JohnsonShortestPaths};
use jgrapht::{DiGraph, Graph, GraphMut, GraphMutExt, UGraph};
use std::collections::BTreeSet;

const EPSILON: f64 = 0.000000001;

fn graph(edges: &[(i32, i32)]) -> UGraph<i32> {
    let mut g = UGraph::<i32>::simple();
    g.add_vertices(0..7);
    for (u, v) in edges {
        g.add_edge(u, v).unwrap();
    }
    g
}

fn graph1() -> UGraph<i32> {
    graph(&[(0, 1), (1, 2), (1, 3), (1, 5), (3, 4), (5, 6)])
}

fn graph2() -> UGraph<i32> {
    graph(&[(0, 1), (1, 2), (1, 3), (1, 4), (1, 5), (5, 6)])
}

fn graph3() -> UGraph<i32> {
    let mut g = UGraph::<i32>::simple().weighted();
    let mut random = StdRng::seed_from_u64(12345678);
    let n = 100;
    let m = 100000;
    g.add_vertices(0..n);
    for i in 0..n - 1 {
        g.add_edge(&i, &(i + 1)).unwrap();
        let e = g.edge_between(&i, &(i + 1)).cloned().unwrap();
        g.set_edge_weight(&e, (50 + random.next_int_bound(50)) as f64)
            .unwrap();
    }
    for _ in n - 1..m {
        let u = random.next_int_bound(n as usize) as i32;
        let v = random.next_int_bound(n as usize) as i32;
        if u != v {
            // an existing edge is kept, but its weight is still overwritten, as in Java
            let _ = g.add_edge(&u, &v);
            let e = g.edge_between(&u, &v).cloned().unwrap();
            g.set_edge_weight(&e, (100 + random.next_int_bound(200)) as f64)
                .unwrap();
        }
    }
    g
}

fn set(v: Vec<i32>) -> BTreeSet<i32> {
    v.into_iter().collect()
}

fn check_eccentricity(g: &UGraph<i32>, expected: [f64; 7]) {
    let fw = FloydWarshallShortestPaths::new(g).unwrap();
    let johnson = JohnsonShortestPaths::new(g).unwrap();
    let measurers = [
        GraphMeasurer::with_algorithm(g, &fw).unwrap(),
        GraphMeasurer::with_algorithm(g, &johnson).unwrap(),
    ];
    for gdm in &measurers {
        let ecc = gdm.vertex_eccentricity_map();
        for (v, e) in expected.iter().enumerate() {
            let got = ecc.iter().find(|(u, _)| *u == v as i32).unwrap().1;
            assert!((got - e).abs() < EPSILON, "vertex {v}: {got} vs {e}");
        }
    }
}

#[test]
fn test_vertex_eccentricity_g1() {
    check_eccentricity(&graph1(), [3.0, 2.0, 3.0, 3.0, 4.0, 3.0, 4.0]);
}

#[test]
fn test_vertex_eccentricity_g2() {
    check_eccentricity(&graph2(), [3.0, 2.0, 3.0, 3.0, 3.0, 2.0, 3.0]);
}

#[test]
fn test_diameter_empty_graph() {
    let g = UGraph::<i32>::simple();
    assert_eq!(GraphMeasurer::new(&g).unwrap().diameter(), Some(0.0));
}

#[test]
fn test_diameter_disconnected_graph() {
    let mut g = UGraph::<i32>::simple();
    g.add_vertices([0, 1]);
    assert!(GraphMeasurer::new(&g).unwrap().diameter().is_none());
}

#[test]
fn test_diameter_directed_graph1() {
    let mut g = DiGraph::<i32>::simple().weighted();
    g.add_vertices([0, 1]);
    g.add_weighted_edge(&0, &1, 10.0).unwrap();
    assert!(GraphMeasurer::new(&g).unwrap().diameter().is_none());
}

#[test]
fn test_diameter_directed_graph2() {
    let mut g = DiGraph::<i32>::simple().weighted();
    g.add_vertices([0, 1]);
    g.add_weighted_edge(&0, &1, 10.0).unwrap();
    g.add_weighted_edge(&1, &0, 12.0).unwrap();
    assert!((GraphMeasurer::new(&g).unwrap().diameter().unwrap() - 12.0).abs() < EPSILON);
}

#[test]
fn test_radius_empty_graph() {
    let g = UGraph::<i32>::simple();
    assert_eq!(GraphMeasurer::new(&g).unwrap().radius(), Some(0.0));
}

#[test]
fn test_radius_disconnected_graph() {
    let mut g = UGraph::<i32>::simple();
    g.add_vertices([0, 1]);
    assert!(GraphMeasurer::new(&g).unwrap().radius().is_none());
}

#[test]
fn test_graph_center_g1() {
    let gdm = GraphMeasurer::new(&graph1()).unwrap();
    assert_eq!(set(gdm.graph_center()), set(vec![1]));
}

#[test]
fn test_graph_center_g2() {
    let gdm = GraphMeasurer::new(&graph2()).unwrap();
    assert_eq!(set(gdm.graph_center()), set(vec![1, 5]));
}

#[test]
fn test_graph_periphery_g1() {
    let gdm = GraphMeasurer::new(&graph1()).unwrap();
    assert_eq!(set(gdm.graph_periphery()), set(vec![4, 6]));
}

#[test]
fn test_graph_periphery_g2() {
    let gdm = GraphMeasurer::new(&graph2()).unwrap();
    assert_eq!(set(gdm.graph_periphery()), set(vec![0, 2, 3, 4, 6]));
}

#[test]
fn test_graph_pseudo_periphery_g1() {
    let gdm = GraphMeasurer::new(&graph1()).unwrap();
    assert_eq!(set(gdm.graph_pseudo_periphery()), set(vec![4, 6]));
}

#[test]
fn test_graph_pseudo_periphery_g2() {
    let gdm = GraphMeasurer::new(&graph2()).unwrap();
    assert_eq!(set(gdm.graph_pseudo_periphery()), set(vec![0, 2, 3, 4, 6]));
}

/// The graph is random, so the periphery is checked against an independent Floyd-Warshall
/// computation of the definition: `u` is in the pseudo-periphery when some `v` has
/// `d(u, v) = ecc(u) = ecc(v)`.
#[test]
fn test_graph_pseudo_periphery_g3() {
    let g = graph3();
    let vs: Vec<i32> = g.vertices_cloned().collect();
    let n = vs.len();
    let mut d = vec![vec![f64::INFINITY; n]; n];
    for (i, row) in d.iter_mut().enumerate() {
        row[i] = 0.0;
    }
    for e in g.edges() {
        let (a, b) = (*g.edge_source(e).unwrap(), *g.edge_target(e).unwrap());
        let w = g.edge_weight(e).unwrap();
        let (a, b) = (a as usize, b as usize);
        d[a][b] = d[a][b].min(w);
        d[b][a] = d[b][a].min(w);
    }
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                d[i][j] = d[i][j].min(d[i][k] + d[k][j]);
            }
        }
    }
    let ecc: Vec<f64> = d
        .iter()
        .map(|row| row.iter().copied().fold(0.0, f64::max))
        .collect();
    let want: BTreeSet<i32> = (0..n)
        .filter(|&u| (0..n).any(|v| d[u][v] == ecc[u] && ecc[u] == ecc[v]))
        .map(|u| vs[u])
        .collect();
    assert!(!want.is_empty());
    let gdm = GraphMeasurer::new(&g).unwrap();
    assert_eq!(set(gdm.graph_pseudo_periphery()), want);
}
