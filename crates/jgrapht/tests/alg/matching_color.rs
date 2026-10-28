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

use jgrapht::alg::color::*;
use jgrapht::alg::matching::*;
use jgrapht::alg::partition::*;
use jgrapht::generate::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

fn brute_max_matching(edges: &[(usize, usize, f64)], used: u32, i: usize, weighted: bool) -> f64 {
    if i == edges.len() {
        return 0.0;
    }
    let (a, b, w) = edges[i];
    let skip = brute_max_matching(edges, used, i + 1, weighted);
    if used & (1 << a) == 0 && used & (1 << b) == 0 {
        let gain = if weighted { w } else { 1.0 };
        skip.max(gain + brute_max_matching(edges, used | 1 << a | 1 << b, i + 1, weighted))
    } else {
        skip
    }
}

fn weighted_edges<G: Graph<Vertex = usize>>(g: &G) -> Vec<(usize, usize, f64)> {
    g.edges()
        .map(|e| {
            (
                *g.edge_source(e).unwrap(),
                *g.edge_target(e).unwrap(),
                g.edge_weight(e).unwrap(),
            )
        })
        .collect()
}

fn matching_is_valid<G: Graph>(g: &G, matching: &Matching<G::Vertex, G::Edge>) -> bool {
    let mut used = std::collections::HashSet::new();
    matching
        .edges()
        .iter()
        .all(|edge| match g.edge_endpoints(edge) {
            Some((source, target)) => {
                source != target && used.insert(source) && used.insert(target)
            }
            None => false,
        })
}

#[test]
fn bipartite_partition_works() {
    let mut g = UGraph::<usize>::simple();
    CompleteBipartite(3, 2).generate_indexed(&mut g).unwrap();
    let (a, b) = bipartite_partition(&g).unwrap();
    assert_eq!((a.len(), b.len()), (3, 2));
    let mut tri = UGraph::<usize>::simple();
    Ring(5).generate_indexed(&mut tri).unwrap();
    assert!(!is_bipartite(&tri));
}

#[test]
fn matchings_against_brute_force() {
    let mut rng = StdRng::seed_from_u64(1);
    for _ in 0..40 {
        let mut g = UGraph::<usize>::simple().weighted();
        GnpRandomGraphGenerator::new(10, 0.3, StdRng::seed_from_u64(rng.random()), false)
            .unwrap()
            .generate_indexed(&mut g)
            .unwrap();
        for e in g.edges_cloned().collect::<Vec<_>>() {
            g.set_edge_weight(&e, rng.random_range(1..9) as f64)
                .unwrap();
        }
        let edges = weighted_edges(&g);
        let m = DenseEdmondsMaximumCardinalityMatching::new(&g)
            .unwrap()
            .get_matching()
            .unwrap();
        assert!(matching_is_valid(&g, &m));
        assert_eq!(m.len() as f64, brute_max_matching(&edges, 0, 0, false));
        let greedy = GreedyWeightedMatching::new(&g, false)
            .unwrap()
            .get_matching()
            .unwrap();
        assert!(matching_is_valid(&g, &greedy));
        assert!(2.0 * greedy.weight() >= brute_max_matching(&edges, 0, 0, true) - 1e-9);
    }
    // odd cycle forces a blossom
    let mut g = UGraph::<usize>::simple();
    Ring(7).generate_indexed(&mut g).unwrap();
    assert_eq!(
        DenseEdmondsMaximumCardinalityMatching::new(&g)
            .unwrap()
            .get_matching()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn bipartite_matchings() {
    let mut rng = StdRng::seed_from_u64(2);
    for _ in 0..40 {
        let mut g = UGraph::<usize>::simple().weighted();
        GnpRandomGraphGenerator::new(9, 0.5, StdRng::seed_from_u64(rng.random()), false)
            .unwrap()
            .generate_indexed(&mut g)
            .unwrap();
        // keep only edges between evens and odds so the graph is bipartite
        for e in g.edges_cloned().collect::<Vec<_>>() {
            let (a, b) = (*g.edge_source(&e).unwrap(), *g.edge_target(&e).unwrap());
            if (a + b) % 2 == 0 {
                g.remove_edge(&e);
            } else {
                g.set_edge_weight(&e, rng.random_range(1..9) as f64)
                    .unwrap();
            }
        }
        let edges = weighted_edges(&g);
        let (partition1, partition2) = bipartite_partition(&g).unwrap();
        let hk = HopcroftKarpMaximumCardinalityBipartiteMatching::new(
            &g,
            partition1.clone(),
            partition2.clone(),
        )
        .unwrap()
        .get_matching()
        .unwrap();
        assert!(matching_is_valid(&g, &hk));
        assert_eq!(hk.len() as f64, brute_max_matching(&edges, 0, 0, false));
        let hu = MaximumWeightBipartiteMatching::new(
            &g,
            partition1.into_iter().collect(),
            partition2.into_iter().collect(),
        )
        .unwrap()
        .get_matching()
        .unwrap();
        assert!(matching_is_valid(&g, &hu));
        assert_eq!(hu.weight(), brute_max_matching(&edges, 0, 0, true));
    }
    let mut tri = UGraph::<usize>::simple();
    Ring(3).generate_indexed(&mut tri).unwrap();
    assert!(bipartite_partition(&tri).is_none());
}

#[test]
fn colorings() {
    let mut rng = StdRng::seed_from_u64(4);
    for _ in 0..20 {
        let mut g = UGraph::<usize>::simple();
        GnpRandomGraphGenerator::new(30, 0.2, StdRng::seed_from_u64(rng.random()), false)
            .unwrap()
            .generate_indexed(&mut g)
            .unwrap();
        let max_deg = g.vertices().map(|v| g.degree_of(v)).max().unwrap();
        for c in [
            greedy_coloring(&g),
            largest_degree_first_coloring(&g),
            smallest_degree_last_coloring(&g),
            saturation_degree_coloring(&g),
        ] {
            assert!(is_proper_coloring(&g, &c.colors));
            assert!(c.count <= max_deg + 1);
        }
    }
    let mut ring = UGraph::<usize>::simple();
    Ring(6).generate_indexed(&mut ring).unwrap();
    assert_eq!(saturation_degree_coloring(&ring).count, 2);
    let mut k5 = UGraph::<usize>::simple();
    Complete(5).generate_indexed(&mut k5).unwrap();
    assert_eq!(ChordalGraphColoring::new(&k5).coloring().unwrap().count, 5);
    assert!(ChordalGraphColoring::new(&ring).coloring().is_none());
}

#[test]
fn refinement_separates_degrees() {
    let mut star = UGraph::<usize>::simple();
    Star(5).generate_indexed(&mut star).unwrap();
    let c = color_refinement(&star);
    assert_ne!(c[&0], c[&1]);
    assert_eq!(c[&1], c[&4]);
}
