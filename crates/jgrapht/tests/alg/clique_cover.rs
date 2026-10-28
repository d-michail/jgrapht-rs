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

use jgrapht::alg::clique::*;
use jgrapht::alg::cover::*;
use jgrapht::alg::matching::{HopcroftKarp, MatchingAlgorithm};
use jgrapht::alg::partition::bipartite_partition;
use jgrapht::generate::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

fn random(seed: u64, n: usize, p: f64) -> UGraph<usize> {
    let mut g = UGraph::<usize>::simple();
    GnpRandomGraphGenerator::new(n, p, StdRng::seed_from_u64(seed), false)
        .unwrap()
        .generate_indexed(&mut g)
        .unwrap();
    g
}

fn brute_min_cover(g: &UGraph<usize>) -> usize {
    let n = g.vertex_count();
    (0u32..1 << n)
        .filter(|mask| {
            g.edges().all(|e| {
                mask >> g.edge_source(e).unwrap() & 1 == 1
                    || mask >> g.edge_target(e).unwrap() & 1 == 1
            })
        })
        .map(|m| m.count_ones() as usize)
        .min()
        .unwrap()
}

#[test]
fn cliques_and_independent_sets() {
    let mut k = UGraph::<usize>::simple();
    Complete(5).generate_indexed(&mut k).unwrap();
    assert_eq!(maximal_cliques(&k).len(), 1);
    assert_eq!(maximum_clique(&k).len(), 5);
    let mut rng = StdRng::seed_from_u64(1);
    for _ in 0..20 {
        let g = random(rng.random(), 12, 0.4);
        let cliques = maximal_cliques(&g);
        assert!(cliques.iter().all(|c| is_clique(&g, c)));
        // maximality: no outside vertex extends the clique
        for c in &cliques {
            assert!(
                g.vertices()
                    .filter(|v| !c.contains(v))
                    .all(|v| !c.iter().all(|u| g.contains_edge_between(u, v)))
            );
        }
        let mis = maximum_independent_set(&g);
        assert!(is_independent_set(&g, &mis));
        assert!(greedy_independent_set(&g).len() <= mis.len());
        assert!(is_independent_set(&g, &greedy_independent_set(&g)));
        // alpha + tau = n
        assert_eq!(mis.len() + brute_min_cover(&g), 12);
    }
}

#[test]
fn vertex_covers() {
    let mut rng = StdRng::seed_from_u64(2);
    for _ in 0..20 {
        let g = random(rng.random(), 12, 0.3);
        let opt = brute_min_cover(&g);
        let greedy = greedy_vertex_cover(&g);
        assert!(is_vertex_cover(&g, &greedy.vertices));
        let two = two_approximation_vertex_cover(&g);
        assert!(is_vertex_cover(&g, &two.vertices) && two.vertices.len() <= 2 * opt);
        let by = bar_yehuda_even_vertex_cover(&g, |_| 1.0);
        assert!(is_vertex_cover(&g, &by.vertices) && by.weight <= 2.0 * opt as f64);
        let cl = clarkson_vertex_cover(&g, |_| 1.0);
        assert!(is_vertex_cover(&g, &cl.vertices) && cl.weight <= 2.0 * opt as f64);
    }
}

#[test]
fn koenig_cover_equals_matching() {
    let mut rng = StdRng::seed_from_u64(3);
    for _ in 0..20 {
        let mut g = random(rng.random(), 10, 0.5);
        for e in g.edges_cloned().collect::<Vec<_>>() {
            if (g.edge_source(&e).unwrap() + g.edge_target(&e).unwrap()) % 2 == 0 {
                g.remove_edge(&e);
            }
        }
        let c = bipartite_vertex_cover(&g).unwrap();
        assert!(is_vertex_cover(&g, &c.vertices));
        let (partition1, partition2) = bipartite_partition(&g).unwrap();
        let matching = HopcroftKarp::new(&g, partition1, partition2)
            .unwrap()
            .matching()
            .unwrap();
        assert_eq!(c.vertices.len(), matching.len());
    }
}
