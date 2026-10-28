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

use jgrapht::alg::connectivity::block_cut_tree;
use jgrapht::alg::matching::*;
use jgrapht::alg::shortestpath::{
    AllDirectedPaths, BellmanFord, BhandariKDisjointShortestPaths, SuurballeKDisjointShortestPaths,
};
use jgrapht::alg::tour::*;
use jgrapht::generate::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::borrow::Borrow;

fn dg(edges: &[(u32, u32, f64)]) -> DiGraph<u32> {
    let mut g = DiGraph::<u32>::simple().weighted();
    for &(a, b, w) in edges {
        g.add_vertex(a);
        g.add_vertex(b);
        g.add_weighted_edge(&a, &b, w).unwrap();
    }
    g
}

fn matching_is_valid<G: Graph>(g: &G, matching: &Matching<G::Vertex, G::Edge>) -> bool {
    let mut used = std::collections::HashSet::new();
    matching.edges().iter().all(|edge| {
        let edge = edge.borrow();
        match g.edge_endpoints(edge) {
            Some((source, target)) => {
                source != target && used.insert(source) && used.insert(target)
            }
            None => false,
        }
    })
}

fn exact_matching(edges: &[(usize, usize, f64)], used: u16, index: usize) -> (f64, usize) {
    if index == edges.len() {
        return (0.0, 0);
    }
    let skip = exact_matching(edges, used, index + 1);
    let (source, target, weight) = edges[index];
    if used & (1 << source) != 0 || used & (1 << target) != 0 {
        return skip;
    }
    let (rest_weight, rest_len) =
        exact_matching(edges, used | (1 << source) | (1 << target), index + 1);
    let take = (weight + rest_weight, rest_len + 1);
    if take.0 > skip.0 { take } else { skip }
}

#[test]
fn simple_paths() {
    let g = dg(&[
        (1, 2, 1.0),
        (2, 4, 1.0),
        (1, 3, 1.0),
        (3, 4, 1.0),
        (2, 3, 1.0),
    ]);
    let algorithm = AllDirectedPaths::new(&g);
    let ps = algorithm.get_all_paths(&1, &4, true, None).unwrap();
    assert_eq!(ps.len(), 3);
    assert!(ps.iter().all(|p| p.is_valid(&g)));
    assert_eq!(
        algorithm
            .get_all_paths(&1, &4, true, Some(2))
            .unwrap()
            .len(),
        2
    );
    assert!(
        algorithm
            .get_all_paths(&4, &1, true, None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn negative_cycles() {
    let g = dg(&[(1, 2, 1.0), (2, 3, -2.0), (3, 1, 0.5), (3, 4, 1.0)]);
    let error = BellmanFord::new(&g).get_paths(&1).unwrap_err();
    let mut cycle = error.cycle().unwrap().vertex_list().to_vec();
    cycle.pop();
    cycle.sort();
    assert_eq!(cycle, [1, 2, 3]);
    assert!(
        BellmanFord::new(&dg(&[(1, 2, -1.0), (2, 3, 1.0)]))
            .get_paths(&1)
            .is_ok()
    );
}

#[test]
fn disjoint_paths() {
    // two routes from 0 to 5 sharing vertex 3 only in the edge-disjoint case
    let g = dg(&[
        (0, 1, 1.0),
        (1, 3, 1.0),
        (0, 2, 1.0),
        (2, 3, 1.0),
        (3, 4, 1.0),
        (3, 5, 1.0),
        (4, 5, 1.0),
        (0, 5, 10.0),
    ]);
    let ep = SuurballeKDisjointShortestPaths::new(&g)
        .get_paths(&0, &5, 3)
        .unwrap();
    assert_eq!(ep.len(), 3);
    assert!(ep.iter().all(|p| p.is_valid(&g)));
    let all_edges: Vec<_> = ep.iter().flat_map(|p| p.edge_list().to_vec()).collect();
    let mut dedup = all_edges.clone();
    dedup.sort();
    dedup.dedup();
    assert_eq!(all_edges.len(), dedup.len());
    let bp = BhandariKDisjointShortestPaths::new(&g)
        .get_paths(&0, &5, 3)
        .unwrap();
    assert_eq!(bp.len(), 3);
    assert!(bp.iter().all(|path| path.is_valid(&g)));
    assert!(
        SuurballeKDisjointShortestPaths::new(&g)
            .get_paths(&0, &0, 1)
            .is_err()
    );
}

#[test]
fn hamiltonian() {
    // Java has no exact Hamiltonian cycle search for general graphs (see `alg_tour_tests`), so
    // this only checks the path search.
    fn path<G: IncidenceGraph>(g: &G) -> Option<GraphWalk<G::Vertex, G::Edge>> {
        BacktrackingHamiltonianPath::new()
            .get_path(g)
            .unwrap()
            .path()
            .cloned()
    }
    let mut ring = UGraph::<usize>::simple();
    Ring(8).generate_indexed(&mut ring).unwrap();
    assert!(path(&ring).unwrap().is_valid(&ring));
    let mut star = UGraph::<usize>::simple();
    Star(5).generate_indexed(&mut star).unwrap();
    assert!(path(&star).is_none());
    let mut line = UGraph::<usize>::simple();
    Linear(6).generate_indexed(&mut line).unwrap();
    assert!(path(&line).unwrap().is_valid(&line));
    let mut k33 = UGraph::<usize>::simple();
    CompleteBipartite(3, 3).generate_indexed(&mut k33).unwrap();
    assert!(path(&k33).is_some());
    let mut k23 = UGraph::<usize>::simple();
    CompleteBipartite(2, 3).generate_indexed(&mut k23).unwrap();
    assert!(path(&k23).is_some());
    let mut k24 = UGraph::<usize>::simple();
    CompleteBipartite(2, 4).generate_indexed(&mut k24).unwrap();
    assert!(path(&k24).is_none());
}

#[test]
fn block_cut() {
    let mut g = UGraph::<u32>::simple();
    for (a, b) in [(1, 2), (2, 3), (3, 1), (3, 4), (4, 5), (5, 3), (5, 6)] {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    g.add_vertex(9);
    let t = block_cut_tree(&g);
    assert_eq!(t.blocks.len(), 4); // two triangles, the bridge, the isolated vertex
    let mut cuts = t.cut_vertices.clone();
    cuts.sort();
    assert_eq!(cuts, [3, 5]);
    assert_eq!(t.edges.len(), 4);
}

#[test]
fn approximate_and_exact_weighted_matching() {
    let mut rng = StdRng::seed_from_u64(12);
    for _ in 0..30 {
        let mut g = UGraph::<usize>::simple().weighted();
        GnpRandomGraphGenerator::new(10, 0.4, StdRng::seed_from_u64(rng.random()), false)
            .unwrap()
            .generate_indexed(&mut g)
            .unwrap();
        for e in g.edges_cloned().collect::<Vec<_>>() {
            g.set_edge_weight(&e, rng.random_range(1..10) as f64)
                .unwrap();
        }
        let edges: Vec<_> = g
            .edges()
            .map(|edge| {
                (
                    *g.edge_source(edge).unwrap(),
                    *g.edge_target(edge).unwrap(),
                    g.edge_weight(edge).unwrap(),
                )
            })
            .collect();
        let (exact_weight, exact_len) = exact_matching(&edges, 0, 0);
        let pg = PathGrowingWeightedMatching::new(&g)
            .unwrap()
            .get_matching()
            .unwrap();
        assert!(matching_is_valid(&g, &pg));
        assert!(2.0 * pg.weight() >= exact_weight - 1e-9 && pg.weight() <= exact_weight + 1e-9);
        let maximum_cardinality = DenseEdmondsMaximumCardinalityMatching::new(&g)
            .unwrap()
            .get_matching()
            .unwrap();
        assert!(exact_len <= maximum_cardinality.len());
    }
}

#[test]
fn christofides_is_within_bound() {
    let mut rng = StdRng::seed_from_u64(13);
    for _ in 0..10 {
        let pts: Vec<(f64, f64)> = (0..9)
            .map(|_| (rng.random::<f64>() * 10.0, rng.random::<f64>() * 10.0))
            .collect();
        let mut g = UGraph::<usize>::simple().weighted();
        g.add_vertices(0..9);
        for i in 0..9 {
            for j in i + 1..9 {
                let w = ((pts[i].0 - pts[j].0).powi(2) + (pts[i].1 - pts[j].1).powi(2)).sqrt();
                g.add_weighted_edge(&i, &j, w).unwrap();
            }
        }
        let c = ChristofidesThreeHalvesApproxMetricTSP::new()
            .get_tour(&g)
            .unwrap()
            .unwrap();
        assert!(c.is_valid(&g) && c.length() == 9);
        let opt = HeldKarpTSP::new().get_tour(&g).unwrap().unwrap();
        assert!(c.weight() >= opt.weight() - 1e-9 && c.weight() <= 1.5 * opt.weight() + 1e-9);
    }
}
