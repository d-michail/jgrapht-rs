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

use jgrapht::alg::planar::BoyerMyrvoldPlanarityInspector;
use jgrapht::generate::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

fn is_planar<G: Graph>(g: &G) -> bool {
    BoyerMyrvoldPlanarityInspector::new(g).is_planar()
}

fn build<G: GraphMut<Vertex = usize>>(mut g: G, mut f: impl GraphGenerator) -> G {
    f.generate_indexed(&mut g).unwrap();
    g
}
fn ug(edges: &[(usize, usize)]) -> UGraph<usize> {
    let mut g = UGraph::<usize>::simple();
    for &(a, b) in edges {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    g
}

#[test]
fn known_graphs() {
    assert!(is_planar(&build(UGraph::<usize>::simple(), Complete(4))));
    assert!(!is_planar(&build(UGraph::<usize>::simple(), Complete(5))));
    assert!(!is_planar(&build(
        UGraph::<usize>::simple(),
        CompleteBipartite(3, 3)
    )));
    assert!(is_planar(&build(
        UGraph::<usize>::simple(),
        CompleteBipartite(2, 5)
    )));
    assert!(is_planar(&build(
        UGraph::<usize>::simple(),
        Grid { rows: 6, cols: 7 }
    )));
    assert!(is_planar(&build(UGraph::<usize>::simple(), Hypercube(3))));
    assert!(!is_planar(&build(UGraph::<usize>::simple(), Hypercube(4))));
    assert!(is_planar(&build(
        UGraph::<usize>::simple(),
        Wheel {
            n: 9,
            inward_spokes: false
        }
    )));
    // K5 minus an edge, K3,3 minus an edge
    let mut k5 = build(UGraph::<usize>::simple(), Complete(5));
    let e = *k5.get_edge(&0, &1).unwrap();
    k5.remove_edge(&e);
    assert!(is_planar(&k5));
    let mut k33 = build(UGraph::<usize>::simple(), CompleteBipartite(3, 3));
    let e = *k33.get_edge(&0, &3).unwrap();
    k33.remove_edge(&e);
    assert!(is_planar(&k33));
    // Petersen graph
    let petersen = ug(&[
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (4, 0),
        (0, 5),
        (1, 6),
        (2, 7),
        (3, 8),
        (4, 9),
        (5, 7),
        (7, 9),
        (9, 6),
        (6, 8),
        (8, 5),
    ]);
    assert!(!is_planar(&petersen));
    // subdivided K5 stays non-planar
    let sub = ug(&[
        (0, 5),
        (5, 1),
        (0, 2),
        (0, 3),
        (0, 4),
        (1, 2),
        (1, 3),
        (1, 4),
        (2, 3),
        (2, 4),
        (3, 4),
    ]);
    assert!(!is_planar(&sub));
    // disconnected: planar component + planar component
    assert!(is_planar(&ug(&[
        (0, 1),
        (1, 2),
        (2, 0),
        (3, 4),
        (4, 5),
        (5, 3)
    ])));
    assert!(is_planar(&UGraph::<usize>::simple()));
}

#[test]
fn random_planar_triangulations_are_maximal() {
    let mut rng = StdRng::seed_from_u64(6);
    for _ in 0..25 {
        // Apollonian network: repeatedly insert a vertex into a random triangular face
        let n = rng.random_range(5..14);
        let mut g = ug(&[(0, 1), (1, 2), (2, 0)]);
        let mut faces = vec![(0usize, 1usize, 2usize)];
        for v in 3..n {
            let (a, b, c) = faces.swap_remove(rng.random_range(0..faces.len()));
            for x in [a, b, c] {
                g.add_edge_with_vertices(v, x).unwrap();
            }
            faces.extend([(a, b, v), (b, c, v), (a, c, v)]);
        }
        assert!(is_planar(&g), "n={n}");
        assert_eq!(g.edge_count(), 3 * n - 6);
        // any extra edge on a maximal planar graph breaks planarity
        for a in 0..n {
            for b in a + 1..n {
                if !g.contains_edge_between(&a, &b) {
                    let mut h = g.clone();
                    h.add_edge(&a, &b).unwrap();
                    assert!(!is_planar(&h));
                }
            }
        }
    }
}

#[test]
fn sparse_random_graphs_vs_edge_bound() {
    let mut rng = StdRng::seed_from_u64(7);
    for _ in 0..30 {
        let g = {
            let mut g = UGraph::<usize>::simple();
            GnmRandomGraphGenerator::new(
                10,
                rng.random_range(5..30),
                StdRng::seed_from_u64(rng.random()),
                false,
                false,
            )
            .unwrap()
            .generate_indexed(&mut g)
            .unwrap();
            g
        };
        if g.edge_count() > 24 {
            assert!(!is_planar(&g));
        }
        // planarity is monotone under edge deletion
        if is_planar(&g) {
            let mut h = g.clone();
            let es: Vec<_> = h.edges().cloned().collect();
            for e in es.iter().step_by(2) {
                h.remove_edge(e);
            }
            assert!(is_planar(&h));
        }
    }
}
