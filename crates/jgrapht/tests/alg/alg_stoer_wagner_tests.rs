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

//! Port of `org.jgrapht.alg.StoerWagnerMinimumCutTest`.

use indexmap::IndexSet;
use jgrapht::alg::stoer_wagner::StoerWagnerMinimumCut;
use jgrapht::*;

type WG = UGraph<&'static str>;

fn weighted(vertices: &[&'static str], edges: &[(&'static str, &'static str, f64)]) -> WG {
    let mut g = WG::simple().weighted();
    for v in vertices {
        g.add_vertex(*v);
    }
    for &(a, b, w) in edges {
        g.add_weighted_edge(&a, &b, w).unwrap();
    }
    g
}

fn set(vs: &[&'static str]) -> IndexSet<&'static str> {
    vs.iter().copied().collect()
}

fn assert_one_of(cut: &IndexSet<&'static str>, a: &[&'static str], b: &[&'static str]) {
    let same = |x: &IndexSet<&str>, y: &IndexSet<&str>| {
        x.len() == y.len() && x.iter().all(|v| y.contains(v))
    };
    assert!(same(cut, &set(a)) || same(cut, &set(b)), "{cut:?}");
}

#[test]
fn test_min_cut14() {
    let g = weighted(
        &["v1", "v2", "v3", "v4"],
        &[
            ("v1", "v2", 3.0),
            ("v1", "v3", 2.0),
            ("v1", "v4", 4.0),
            ("v2", "v3", 1.0),
            ("v3", "v4", 1.0),
        ],
    );
    assert!((StoerWagnerMinimumCut::new(&g).unwrap().min_cut_weight() - 4.0).abs() < 1e-6);
}

#[test]
fn test_min_cut_disconnected() {
    let g = weighted(
        &["v1", "v2", "v3", "v4"],
        &[("v1", "v2", 3.0), ("v1", "v3", 2.0), ("v2", "v3", 1.0)],
    );
    assert!(
        StoerWagnerMinimumCut::new(&g)
            .unwrap()
            .min_cut_weight()
            .abs()
            < 1e-6
    );
}

#[test]
fn test_min_cut0_weight() {
    let g = weighted(
        &["v1", "v2", "v3", "v4", "v5", "v6", "v7", "v8"],
        &[
            ("v1", "v2", 1.0),
            ("v2", "v3", 2.0),
            ("v3", "v4", 0.0),
            ("v4", "v5", 1.0),
            ("v5", "v6", 2.0),
            ("v6", "v1", 0.0),
            ("v6", "v8", 1.0),
            ("v8", "v7", 0.0),
            ("v7", "v3", 2.0),
        ],
    );
    let mincut = StoerWagnerMinimumCut::new(&g).unwrap();
    assert!(mincut.min_cut_weight().abs() < 1e-6);
    assert_one_of(
        mincut.min_cut(),
        &["v4", "v5", "v6", "v8"],
        &["v1", "v2", "v3", "v7"],
    );
}

#[test]
fn test_min_cut_small_weight() {
    let g = weighted(
        &["v1", "v2", "v3", "v4"],
        &[
            ("v1", "v2", 0.5),
            ("v2", "v3", 1.0),
            ("v3", "v4", 0.5),
            ("v4", "v1", 1.0),
        ],
    );
    let mincut = StoerWagnerMinimumCut::new(&g).unwrap();
    assert!((mincut.min_cut_weight() - 1.0).abs() < 1e-6);
    assert_one_of(mincut.min_cut(), &["v1", "v4"], &["v2", "v3"]);
}

#[test]
fn test_min_cut_multigraph() {
    let mut g = WG::multi().weighted();
    for v in ["v1", "v2", "v3"] {
        g.add_vertex(v);
    }
    g.add_weighted_edge(&"v1", &"v2", 1.5).unwrap();
    g.add_weighted_edge(&"v1", &"v2", 1.5).unwrap();
    g.add_weighted_edge(&"v2", &"v3", 2.0).unwrap();
    let mincut = StoerWagnerMinimumCut::new(&g).unwrap();
    assert!((mincut.min_cut_weight() - 2.0).abs() < 1e-6);
    assert_one_of(mincut.min_cut(), &["v1", "v2"], &["v3"]);
}

#[test]
fn test_min_cut_unweighted() {
    let mut g = WG::simple();
    for v in ["v1", "v2", "v3", "v4", "v5", "v6"] {
        g.add_vertex(v);
    }
    for (a, b) in [
        ("v1", "v2"),
        ("v2", "v3"),
        ("v3", "v1"),
        ("v4", "v5"),
        ("v5", "v6"),
        ("v6", "v4"),
        ("v3", "v4"),
    ] {
        g.add_edge(&a, &b).unwrap();
    }
    let mincut = StoerWagnerMinimumCut::new(&g).unwrap();
    assert!((mincut.min_cut_weight() - 1.0).abs() < 1e-6);
    assert_one_of(mincut.min_cut(), &["v1", "v2", "v3"], &["v4", "v5", "v6"]);
}

#[test]
fn test_min_cut_empty() {
    let g = WG::simple();
    assert!(matches!(
        StoerWagnerMinimumCut::new(&g),
        Err(GraphError::InvalidGraph(
            jgrapht::GraphProperty::AtLeastTwoVertices
        ))
    ));
}

#[test]
fn test_min_cut_singleton() {
    let mut g = WG::simple();
    g.add_vertex("v1");
    assert!(matches!(
        StoerWagnerMinimumCut::new(&g),
        Err(GraphError::InvalidGraph(
            jgrapht::GraphProperty::AtLeastTwoVertices
        ))
    ));
}

#[test]
fn test_min_cut_doubleton() {
    let mut g = WG::simple();
    g.add_vertex("v1");
    g.add_vertex("v2");
    let mincut = StoerWagnerMinimumCut::new(&g).unwrap();
    assert!(mincut.min_cut_weight().abs() < 1e-6);
    assert_one_of(mincut.min_cut(), &["v1"], &["v2"]);
}

#[test]
fn test_min_cut_small() {
    let mut g = WG::simple();
    g.add_vertex("v1");
    g.add_vertex("v2");
    g.add_edge(&"v1", &"v2").unwrap();
    let mincut = StoerWagnerMinimumCut::new(&g).unwrap();
    assert!((mincut.min_cut_weight() - 1.0).abs() < 1e-6);
    assert_one_of(mincut.min_cut(), &["v1"], &["v2"]);
}

/// Not in Java: negative weights and loops are rejected (a directed graph does not compile).
#[test]
fn rejects_invalid_input() {
    let g = weighted(&["a", "b"], &[("a", "b", -1.0)]);
    assert!(StoerWagnerMinimumCut::new(&g).is_err());

    let mut l = UGraph::<u32>::with_loops();
    l.add_vertex(1);
    l.add_vertex(2);
    l.add_edge(&1, &1).unwrap();
    assert!(StoerWagnerMinimumCut::new(&l).is_err());
}
