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
    let same = |x: &IndexSet<&str>, y: &IndexSet<&str>| x.len() == y.len() && x.iter().all(|v| y.contains(v));
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
    assert!(StoerWagnerMinimumCut::new(&g).unwrap().min_cut_weight().abs() < 1e-6);
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
    assert_one_of(mincut.min_cut(), &["v4", "v5", "v6", "v8"], &["v1", "v2", "v3", "v7"]);
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
        Err(GraphError::InvalidArgument(_))
    ));
}

#[test]
fn test_min_cut_singleton() {
    let mut g = WG::simple();
    g.add_vertex("v1");
    assert!(matches!(
        StoerWagnerMinimumCut::new(&g),
        Err(GraphError::InvalidArgument(_))
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

/// Not in Java: directed graphs, negative weights and loops are rejected.
#[test]
fn rejects_invalid_input() {
    let mut d = DiGraph::<u32>::simple();
    d.add_edge_with_vertices(1, 2).unwrap();
    assert!(StoerWagnerMinimumCut::new(&d).is_err());

    let g = weighted(&["a", "b"], &[("a", "b", -1.0)]);
    assert!(StoerWagnerMinimumCut::new(&g).is_err());

    let mut l = UGraph::<u32>::with_loops();
    l.add_vertex(1);
    l.add_vertex(2);
    l.add_edge(&1, &1).unwrap();
    assert!(StoerWagnerMinimumCut::new(&l).is_err());
}
