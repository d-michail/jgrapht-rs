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

use jgrapht::alg::centers::*;
use jgrapht::alg::decomposition::*;
use jgrapht::alg::densest::*;
use jgrapht::alg::drawing::*;
use jgrapht::alg::shortestpath::{Dijkstra, FloydWarshallShortestPaths, ShortestPathAlgorithm};
use jgrapht::alg::spanner::*;
use jgrapht::alg::spanning::kruskal;
use jgrapht::alg::steiner::*;
use jgrapht::generate::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

fn gnp(seed: u64, n: usize, p: f64) -> UGraph<usize> {
    let mut g = UGraph::<usize>::simple().weighted();
    GnpRandomGraphGenerator::new(n, p, StdRng::seed_from_u64(seed), false)
        .unwrap()
        .generate_indexed(&mut g)
        .unwrap();
    let mut rng = StdRng::seed_from_u64(seed ^ 0xabc);
    for e in g.edges_cloned().collect::<Vec<_>>() {
        g.set_edge_weight(&e, rng.random_range(1..10) as f64)
            .unwrap();
    }
    g
}

#[test]
fn densest_subgraph() {
    let mut rng = StdRng::seed_from_u64(1);
    for _ in 0..15 {
        let g = gnp(rng.random(), 9, 0.4);
        let mut best = 0.0f64;
        for mask in 1u32..1 << 9 {
            let m = g
                .edges()
                .filter(|e| {
                    mask >> g.edge_source(e).unwrap() & 1 == 1
                        && mask >> g.edge_target(e).unwrap() & 1 == 1
                })
                .count();
            best = best.max(m as f64 / mask.count_ones() as f64);
        }
        // the brute force counts edges, so give Goldberg an unweighted copy
        let mut plain = UGraph::<usize>::simple();
        plain.add_vertices(g.vertices_cloned());
        for e in g.edges() {
            let (u, v) = g.edge_endpoints(e).unwrap();
            plain.add_edge(u, v).unwrap();
        }
        let mut goldberg = GoldbergMaximumDensitySubgraph::new(&plain, 100, 101, 1e-9).unwrap();
        let set = goldberg.calculate_densest().unwrap();
        let d = goldberg.density().unwrap();
        assert!((d - best).abs() < 1e-6, "exact {d} vs brute {best}");
        assert!(set.vertex_count() > 0);
        let (_, approx) = greedy_densest_subgraph(&g);
        assert!(approx * 2.0 >= best - 1e-9 && approx <= best + 1e-9);
    }
}

#[test]
fn steiner_trees() {
    let mut rng = StdRng::seed_from_u64(2);
    for _ in 0..10 {
        let g = gnp(rng.random(), 9, 0.6);
        if !jgrapht::graph::props::is_connected(&g) {
            continue;
        }
        let terms = [0usize, 3, 5];
        // brute force: choose extra vertices, take MST of the induced subgraph
        let mut opt = f64::INFINITY;
        for mask in 0u32..1 << 9 {
            if terms.iter().any(|&t| mask >> t & 1 == 0) {
                continue;
            }
            let sub = AsSubgraph::induced(&g, (0..9).filter(|v| mask >> v & 1 == 1));
            if jgrapht::graph::props::is_connected(&sub) {
                opt = opt.min(kruskal(&sub).weight);
            }
        }
        assert_eq!(dreyfus_wagner_weight(&g, &terms).unwrap(), opt);
        let kmb = kmb_steiner_tree(&g, &terms).unwrap();
        assert!(kmb.weight >= opt && kmb.weight <= 2.0 * opt);
        // the KMB edges connect all terminals
        let sub = AsSubgraph::with_edges(
            &g,
            kmb.edges
                .iter()
                .flat_map(|e| [*g.edge_source(e).unwrap(), *g.edge_target(e).unwrap()]),
            kmb.edges.iter().cloned(),
        );
        assert!(
            jgrapht::graph::props::is_connected(&sub)
                && terms.iter().all(|t| sub.contains_vertex(t))
        );
    }
}

#[test]
fn spanner_stretch() {
    let mut rng = StdRng::seed_from_u64(3);
    for k in 1..=3usize {
        let g = gnp(rng.random(), 14, 0.5);
        let kept = greedy_spanner(&g, k).unwrap();
        let sub = AsSubgraph::with_edges(&g, g.vertices_cloned(), kept.iter().cloned());
        let (full, sp) = (
            FloydWarshallShortestPaths::new(&g)
                .unwrap()
                .all_pairs()
                .clone(),
            FloydWarshallShortestPaths::new(&sub)
                .unwrap()
                .all_pairs()
                .clone(),
        );
        for a in 0..14 {
            for b in 0..14 {
                assert!(
                    sp.weight(&a, &b).unwrap_or(f64::INFINITY)
                        <= (2 * k - 1) as f64 * full.weight(&a, &b).unwrap_or(f64::INFINITY) + 1e-9
                );
            }
        }
    }
}

#[test]
fn tree_decompositions_are_valid() {
    let mut rng = StdRng::seed_from_u64(4);
    for h in [
        EliminationHeuristic::MinDegree,
        EliminationHeuristic::MinFill,
    ] {
        for _ in 0..10 {
            let g = gnp(rng.random(), 12, 0.25);
            let td = tree_decomposition(&g, h);
            assert_eq!(td.tree_edges.len() + 1, td.bags.len());
            for v in g.vertices() {
                let holding: Vec<usize> = (0..td.bags.len())
                    .filter(|&i| td.bags[i].contains(v))
                    .collect();
                assert!(!holding.is_empty());
                // connected inside the tree
                let sub = &td
                    .tree_edges
                    .iter()
                    .filter(|(a, b)| holding.contains(a) && holding.contains(b))
                    .count();
                assert_eq!(*sub + 1, holding.len());
            }
            for e in g.edges() {
                let (a, b) = (g.edge_source(e).unwrap(), g.edge_target(e).unwrap());
                assert!(td.bags.iter().any(|bag| bag.contains(a) && bag.contains(b)));
            }
        }
    }
    let mut tree = UGraph::<usize>::simple();
    Linear(8).generate_indexed(&mut tree).unwrap();
    assert_eq!(treewidth_upper_bound(&tree), 1);
    let mut k5 = UGraph::<usize>::simple();
    Complete(5).generate_indexed(&mut k5).unwrap();
    assert_eq!(treewidth_upper_bound(&k5), 4);
}

#[test]
fn k_center_is_two_approximation() {
    let mut rng = StdRng::seed_from_u64(5);
    for _ in 0..10 {
        let g = gnp(rng.random(), 10, 0.5);
        if !jgrapht::graph::props::is_connected(&g) {
            continue;
        }
        let k = 3;
        let (centers, r) = greedy_k_center(&g, k, true).unwrap();
        assert_eq!(centers.len(), k);
        let all = FloydWarshallShortestPaths::new(&g)
            .unwrap()
            .all_pairs()
            .clone();
        let radius = |cs: &[usize]| {
            (0..10)
                .map(|v| {
                    cs.iter()
                        .map(|&c| all.weight(&c, &v).unwrap_or(f64::INFINITY))
                        .fold(f64::INFINITY, f64::min)
                })
                .fold(0.0, f64::max)
        };
        assert!((radius(&centers) - r).abs() < 1e-9);
        let opt = (0u32..1 << 10)
            .filter(|m| m.count_ones() as usize == k)
            .map(|m| radius(&(0..10).filter(|v| m >> v & 1 == 1).collect::<Vec<_>>()))
            .fold(f64::INFINITY, f64::min);
        assert!(r <= 2.0 * opt + 1e-9);
    }
    assert!(greedy_k_center(&gnp(1, 5, 0.5), 0, true).is_err());
}

#[test]
fn layouts() {
    let g = gnp(9, 10, 0.4);
    let c = circular_layout(&g, 2.0);
    assert!(
        c.values()
            .all(|(x, y)| ((x * x + y * y).sqrt() - 2.0).abs() < 1e-9)
    );
    let mut rng = StdRng::seed_from_u64(1);
    let r = random_layout(&g, 5.0, 3.0, &mut rng);
    assert!(
        r.values()
            .all(|&(x, y)| (0.0..=5.0).contains(&x) && (0.0..=3.0).contains(&y))
    );
    let f = fruchterman_reingold(&g, 10.0, 10.0, 50, &mut rng);
    assert_eq!(f.len(), 10);
    assert!(
        f.values()
            .all(|&(x, y)| (0.0..=10.0).contains(&x) && (0.0..=10.0).contains(&y))
    );
    let _ = Dijkstra::new(&g).single_source(&0);
}
