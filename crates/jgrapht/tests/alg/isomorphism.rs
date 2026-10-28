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

use jgrapht::alg::isomorphism::{VF2GraphIsomorphismInspector, VF2SubgraphIsomorphismInspector};
use jgrapht::generate::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

fn build<G: GraphMut<Vertex = usize> + IncidenceGraph>(mut g: G, mut f: impl GraphGenerator) -> G {
    f.generate_indexed(&mut g).unwrap();
    g
}
fn count_isomorphisms<G: IncidenceGraph>(g1: &G, g2: &G) -> usize {
    VF2GraphIsomorphismInspector::new(g1, g2)
        .unwrap()
        .iter()
        .count()
}
fn is_isomorphic<G: IncidenceGraph>(g1: &G, g2: &G) -> bool {
    VF2GraphIsomorphismInspector::new(g1, g2)
        .unwrap()
        .iter()
        .next()
        .is_some()
}
fn count_induced_subgraphs<G: IncidenceGraph>(pattern: &G, target: &G) -> usize {
    VF2SubgraphIsomorphismInspector::new(target, pattern)
        .unwrap()
        .iter()
        .count()
}
fn relabel(g: &UGraph<usize>, perm: &[usize]) -> UGraph<usize> {
    let mut h = UGraph::<usize>::simple();
    let c = |i: usize| perm[i];
    g.vertices().for_each(|&v| {
        h.add_vertex(c(v));
    });
    for e in g.edges() {
        h.add_edge(
            &c(*g.edge_source(e).unwrap()),
            &c(*g.edge_target(e).unwrap()),
        )
        .unwrap();
    }
    h
}

#[test]
fn isomorphic_after_relabeling() {
    let mut rng = StdRng::seed_from_u64(8);
    for _ in 0..20 {
        let g = {
            let mut g = UGraph::<usize>::simple();
            GnpRandomGraphGenerator::new(9, 0.35, StdRng::seed_from_u64(rng.random()), false)
                .unwrap()
                .generate_indexed(&mut g)
                .unwrap();
            g
        };
        let mut perm: Vec<usize> = (0..9).collect();
        for i in (1..9).rev() {
            perm.swap(i, rng.random_range(0..=i));
        }
        let h = relabel(&g, &perm);
        let inspector = VF2GraphIsomorphismInspector::new(&g, &h).unwrap();
        let m = inspector.iter().next().expect("isomorphic");
        assert!(m.is_valid_isomorphism());
        for e in g.edges() {
            let (s, t) = (g.edge_source(e).unwrap(), g.edge_target(e).unwrap());
            let (ms, mt) = (
                m.vertex_correspondence(s, true).unwrap(),
                m.vertex_correspondence(t, true).unwrap(),
            );
            assert!(h.contains_edge_between(ms, mt));
            assert!(m.edge_correspondence(e, true).is_some());
        }
        assert_eq!(m.forward_mapping().len(), 9);
        // removing an edge breaks it
        let mut h2 = h.clone();
        let first = h2.edges().next().cloned();
        if let Some(e) = first {
            h2.remove_edge(&e);
            assert!(!is_isomorphic(&g, &h2));
        }
    }
}

#[test]
fn automorphism_counts() {
    assert_eq!(
        count_isomorphisms(
            &build(UGraph::<usize>::simple(), Ring(5)),
            &build(UGraph::<usize>::simple(), Ring(5))
        ),
        10
    );
    let k4 = build(UGraph::<usize>::simple(), Complete(4));
    assert_eq!(count_isomorphisms(&k4, &k4), 24);
    let star = build(UGraph::<usize>::simple(), Star(5));
    let path = build(UGraph::<usize>::simple(), Linear(5));
    assert!(!is_isomorphic(&star, &path));
    // directed cycle vs directed path orientation
    let c3 = build(DiGraph::<usize>::simple(), Ring(3));
    assert_eq!(count_isomorphisms(&c3, &c3), 3);
    assert!(!is_isomorphic(
        &c3,
        &build(DiGraph::<usize>::simple(), Linear(3))
    ));
}

#[test]
fn induced_subgraphs() {
    let c5 = build(UGraph::<usize>::simple(), Ring(5));
    let p3 = build(UGraph::<usize>::simple(), Linear(3));
    let k4 = build(UGraph::<usize>::simple(), Complete(4));
    let tri = build(UGraph::<usize>::simple(), Complete(3));
    assert_eq!(count_induced_subgraphs(&p3, &c5), 10);
    assert_eq!(count_induced_subgraphs(&p3, &k4), 0);
    assert!(
        VF2SubgraphIsomorphismInspector::new(&k4, &tri)
            .unwrap()
            .iter()
            .next()
            .is_some()
    );
    assert!(
        VF2SubgraphIsomorphismInspector::new(&c5, &tri)
            .unwrap()
            .iter()
            .next()
            .is_none()
    );
}
