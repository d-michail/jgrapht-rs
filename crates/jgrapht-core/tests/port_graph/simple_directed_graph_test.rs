//! Port of `org.jgrapht.graph.SimpleDirectedGraphTest`.
//!
//! Skipped assertions: `NullPointerException` checks on `null` arguments (no nulls in Rust) and
//! `IntrusiveEdgeException` for an edge object that belongs to *another* graph (edges are plain
//! values here, so they can be shared freely).

use jgrapht::*;
use std::collections::HashSet;

type G = DiGraph<&'static str>;

const V1: &str = "v1";
const V2: &str = "v2";
const V3: &str = "v3";
const V4: &str = "v4";

fn uuid_like(i: u64) -> &'static str {
    Box::leak(format!("uuid-{i}").into_boxed_str())
}

fn new_graph() -> G {
    G::simple().with_vertex_factory(uuid_like)
}

struct Fixture {
    g1: G,
    g2: G,
    g3: G,
    g4: G,
    e_loop: DefaultEdge,
    e12_1: DefaultEdge,
    e21_1: DefaultEdge,
    e12_2: DefaultEdge,
    e21_2: DefaultEdge,
    e23_1: DefaultEdge,
    e32_1: DefaultEdge,
    e31_1: DefaultEdge,
    e13_1: DefaultEdge,
    e12_3: DefaultEdge,
    e23_2: DefaultEdge,
    e34_1: DefaultEdge,
    e41_1: DefaultEdge,
}

fn setup() -> Fixture {
    let (mut g1, mut g2, mut g3, mut g4) = (new_graph(), new_graph(), new_graph(), new_graph());
    let e_loop = g1.new_edge().unwrap();
    g1.add_vertex(V1);
    g2.add_vertices([V1, V2]);
    let e12_1 = g2.add_edge(&V1, &V2).unwrap().unwrap();
    let e21_1 = g2.add_edge(&V2, &V1).unwrap().unwrap();
    g3.add_vertices([V1, V2, V3]);
    let e12_2 = g3.add_edge(&V1, &V2).unwrap().unwrap();
    let e21_2 = g3.add_edge(&V2, &V1).unwrap().unwrap();
    let e23_1 = g3.add_edge(&V2, &V3).unwrap().unwrap();
    let e32_1 = g3.add_edge(&V3, &V2).unwrap().unwrap();
    let e31_1 = g3.add_edge(&V3, &V1).unwrap().unwrap();
    let e13_1 = g3.add_edge(&V1, &V3).unwrap().unwrap();
    g4.add_vertices([V1, V2, V3, V4]);
    let e12_3 = g4.add_edge(&V1, &V2).unwrap().unwrap();
    let e23_2 = g4.add_edge(&V2, &V3).unwrap().unwrap();
    let e34_1 = g4.add_edge(&V3, &V4).unwrap().unwrap();
    let e41_1 = g4.add_edge(&V4, &V1).unwrap().unwrap();
    Fixture {
        g1,
        g2,
        g3,
        g4,
        e_loop,
        e12_1,
        e21_1,
        e12_2,
        e21_2,
        e23_1,
        e32_1,
        e31_1,
        e13_1,
        e12_3,
        e23_2,
        e34_1,
        e41_1,
    }
}

fn set<T: Eq + std::hash::Hash>(it: impl IntoIterator<Item = T>) -> HashSet<T> {
    it.into_iter().collect()
}

#[test]
fn test_add_edge_edge() {
    let mut f = setup();
    assert_eq!(
        f.g1.add_edge_with(&V1, &V1, f.e_loop),
        Err(GraphError::LoopsNotAllowed)
    );
    let e = f.g1.new_edge().unwrap();
    assert_eq!(
        f.g1.add_edge_with(&"ya", &"ya", e),
        Err(GraphError::VertexNotFound)
    );
    assert_eq!(f.g2.add_edge_with(&V2, &V1, e), Ok(false));
    assert_eq!(f.g3.add_edge_with(&V2, &V1, e), Ok(false));
    assert_eq!(f.g4.add_edge_with(&V2, &V1, e), Ok(true));
}

#[test]
fn test_add_edge_object_object() {
    let mut f = setup();
    assert_eq!(f.g1.add_edge(&V1, &V1), Err(GraphError::LoopsNotAllowed));
    assert_eq!(f.g1.add_edge(&V2, &V1), Err(GraphError::VertexNotFound));
    assert_eq!(f.g2.add_edge(&V2, &V1), Ok(None));
    assert_eq!(f.g3.add_edge(&V2, &V1), Ok(None));
    assert!(f.g4.add_edge(&V2, &V1).unwrap().is_some());
}

#[test]
fn test_add_vertex() {
    let mut f = setup();
    assert_eq!(
        [
            f.g1.vertex_count(),
            f.g2.vertex_count(),
            f.g3.vertex_count(),
            f.g4.vertex_count()
        ],
        [1, 2, 3, 4]
    );
    assert!(!f.g1.add_vertex(V1));
    assert!(f.g1.add_vertex(V2));
    assert_eq!(f.g1.vertex_count(), 2);
}

#[test]
fn test_contains_edge_edge() {
    let f = setup();
    assert!(f.g2.contains_edge(&f.e12_1) && f.g2.contains_edge(&f.e21_1));
    for e in [f.e12_2, f.e21_2, f.e23_1, f.e32_1, f.e31_1, f.e13_1] {
        assert!(f.g3.contains_edge(&e));
    }
    for e in [f.e12_3, f.e23_2, f.e34_1, f.e41_1] {
        assert!(f.g4.contains_edge(&e));
    }
}

#[test]
fn test_contains_edge_object_object() {
    let mut f = setup();
    assert!(!f.g1.contains_edge_between(&V1, &V2));
    assert!(!f.g1.contains_edge_between(&V1, &V1));
    assert!(f.g2.contains_edge_between(&V1, &V2) && f.g2.contains_edge_between(&V2, &V1));
    for (a, b) in [(V1, V2), (V2, V1), (V3, V2), (V2, V3), (V1, V3), (V3, V1)] {
        assert!(f.g3.contains_edge_between(&a, &b));
    }
    assert!(!f.g4.contains_edge_between(&V1, &V4));
    f.g4.add_edge(&V1, &V4).unwrap();
    assert!(f.g4.contains_edge_between(&V1, &V4));
    assert!(!f.g3.contains_edge_between(&V4, &V2));
}

#[test]
fn test_contains_vertex() {
    let f = setup();
    assert!(f.g1.contains_vertex(&V1) && !f.g1.contains_vertex(&V2));
    assert!(f.g2.contains_vertex(&V1) && f.g2.contains_vertex(&V2) && !f.g2.contains_vertex(&V3));
    assert!(f.g3.contains_vertex(&V3) && !f.g3.contains_vertex(&V4));
    assert!([V1, V2, V3, V4].iter().all(|v| f.g4.contains_vertex(v)));
}

#[test]
fn test_edge_set() {
    let f = setup();
    assert_eq!(f.g1.edge_count(), 0);
    assert_eq!(f.g2.edge_count(), 2);
    assert_eq!(f.g3.edge_count(), 6);
    assert_eq!(f.g4.edge_count(), 4);
    assert_eq!(set(f.g2.edges().copied()), set([f.e12_1, f.e21_1]));
    assert_eq!(
        set(f.g3.edges().copied()),
        set([f.e12_2, f.e21_2, f.e23_1, f.e32_1, f.e31_1, f.e13_1])
    );
    assert_eq!(
        set(f.g4.edges().copied()),
        set([f.e12_3, f.e23_2, f.e34_1, f.e41_1])
    );
}

#[test]
fn test_edges_of() {
    let f = setup();
    assert_eq!(f.g4.edges_of(&V1).count(), 2);
    assert_eq!(f.g3.edges_of(&V1).count(), 4);
}

#[test]
fn test_get_all_edges() {
    let f = setup();
    assert_eq!(
        f.g3.edges_between(&V1, &V2).copied().collect::<Vec<_>>(),
        [f.e12_2]
    );
    assert_eq!(
        f.g3.edges_between(&V2, &V1).copied().collect::<Vec<_>>(),
        [f.e21_2]
    );
}

#[test]
fn test_get_edge() {
    let f = setup();
    assert_eq!(f.g2.get_edge(&V1, &V2), Some(&f.e12_1));
    assert_eq!(f.g2.get_edge(&V2, &V1), Some(&f.e21_1));
    assert_eq!(f.g3.get_edge(&V1, &V2), Some(&f.e12_2));
    assert_eq!(f.g3.get_edge(&V2, &V1), Some(&f.e21_2));
    assert_eq!(f.g3.get_edge(&V3, &V2), Some(&f.e32_1));
    assert_eq!(f.g3.get_edge(&V3, &V1), Some(&f.e31_1));
    assert_eq!(f.g3.get_edge(&V1, &V3), Some(&f.e13_1));
    assert_eq!(f.g4.get_edge(&V1, &V2), Some(&f.e12_3));
    assert_eq!(f.g4.get_edge(&V2, &V3), Some(&f.e23_2));
    assert_eq!(f.g4.get_edge(&V3, &V4), Some(&f.e34_1));
    assert_eq!(f.g4.get_edge(&V4, &V1), Some(&f.e41_1));
}

#[test]
fn test_get_edge_supplier() {
    let mut f = setup();
    let e = f.g1.new_edge().unwrap();
    assert_eq!(f.g1.edge_source(&e), None);
    assert_eq!(f.g1.edge_target(&e), None);
}

#[test]
fn test_get_vertex_supplier() {
    let mut f = setup();
    assert!(f.g1.new_vertex().is_ok());
    let v = f.g1.add_new_vertex().unwrap();
    assert!(f.g1.contains_vertex(&v));
}

#[test]
fn test_in_degree_of() {
    let f = setup();
    assert_eq!(f.g1.in_degree_of(&V1), 0);
    assert_eq!([f.g2.in_degree_of(&V1), f.g2.in_degree_of(&V2)], [1, 1]);
    assert_eq!([V1, V2, V3].map(|v| f.g3.in_degree_of(&v)), [2, 2, 2]);
    assert_eq!(
        [V1, V2, V3, V4].map(|v| f.g4.in_degree_of(&v)),
        [1, 1, 1, 1]
    );
    // unknown vertex: JGraphT throws IllegalArgumentException, here the degree is simply zero
    assert_eq!(f.g3.in_degree_of(&""), 0);
}

#[test]
fn test_incoming_outgoing_edges_of() {
    let f = setup();
    assert_eq!(
        set(f.g2.out_edges_of(&V1).copied()),
        set(f.g2.in_edges_of(&V2).copied())
    );
}

#[test]
fn test_out_degree_of() {
    let f = setup();
    assert_eq!([f.g2.out_degree_of(&V1), f.g2.out_degree_of(&V2)], [1, 1]);
    assert_eq!([V1, V2, V3].map(|v| f.g3.out_degree_of(&v)), [2, 2, 2]);
    assert_eq!(
        [V1, V2, V3, V4].map(|v| f.g4.out_degree_of(&v)),
        [1, 1, 1, 1]
    );
}

#[test]
fn test_outgoing_edges_of() {
    let f = setup();
    assert_eq!(f.g1.out_edges_of(&V1).count(), 0);
    assert_eq!(set(f.g2.out_edges_of(&V1).copied()), set([f.e12_1]));
    assert_eq!(set(f.g2.out_edges_of(&V2).copied()), set([f.e21_1]));
    assert_eq!(
        set(f.g3.out_edges_of(&V1).copied()),
        set([f.e12_2, f.e13_1])
    );
    assert_eq!(
        set(f.g3.out_edges_of(&V2).copied()),
        set([f.e23_1, f.e21_2])
    );
    assert_eq!(
        set(f.g3.out_edges_of(&V3).copied()),
        set([f.e31_1, f.e32_1])
    );
    assert_eq!(set(f.g4.out_edges_of(&V1).copied()), set([f.e12_3]));
    assert_eq!(set(f.g4.out_edges_of(&V2).copied()), set([f.e23_2]));
    assert_eq!(set(f.g4.out_edges_of(&V3).copied()), set([f.e34_1]));
    assert_eq!(set(f.g4.out_edges_of(&V4).copied()), set([f.e41_1]));
}

#[test]
fn test_remove_edge_edge_and_object_object() {
    // `testRemoveEdgeEdge` and `testRemoveEdgeObjectObject` are identical in the Java suite
    let mut f = setup();
    assert_eq!(f.g4.edge_count(), 4);
    f.g4.remove_edge_between(&V1, &V2);
    assert_eq!(f.g4.edge_count(), 3);
    assert!(!f.g4.remove_edge(&f.e_loop));
    let e23 = *f.g4.get_edge(&V2, &V3).unwrap();
    assert!(f.g4.remove_edge(&e23));
    assert_eq!(f.g4.edge_count(), 2);
}

#[test]
fn test_remove_all_edges_object_object() {
    let mut f = setup();
    assert_eq!(f.g2.edge_count(), 2);
    assert!(f.g2.contains_edge_between(&V1, &V2));
    let edges: Vec<_> = f.g2.edges_between(&V1, &V2).copied().collect();
    assert_eq!(f.g2.remove_all_edges(&V1, &V2), Some(edges));
    assert_eq!(f.g2.edge_count(), 1);
    assert!(!f.g2.contains_edge_between(&V1, &V2));

    assert_eq!(f.g4.edge_count(), 4);
    let edges: Vec<_> = f.g4.edges_between(&V3, &V4).copied().collect();
    assert_eq!(f.g4.remove_all_edges(&V3, &V4), Some(edges));
    assert_eq!(f.g4.edge_count(), 3);
    assert!(!f.g4.contains_edge_between(&V3, &V4));
    assert_eq!(f.g4.remove_all_edges(&V3, &V2), Some(vec![]));
    assert_eq!(f.g4.edge_count(), 3);
    assert_eq!(f.g4.remove_all_edges(&V1, &"v5"), None);
}

#[test]
fn test_remove_vertex() {
    let mut f = setup();
    assert_eq!(f.g4.vertex_count(), 4);
    assert!(f.g4.remove_vertex(&V1));
    assert_eq!(f.g4.vertex_count(), 3);
    assert_eq!(f.g4.edge_count(), 2);
    assert!(!f.g4.remove_vertex(&V1));
    assert!(f.g4.remove_vertex(&V2));
    assert_eq!(f.g4.edge_count(), 1);
    assert!(f.g4.remove_vertex(&V3));
    assert_eq!((f.g4.edge_count(), f.g4.vertex_count()), (0, 1));
    assert!(f.g4.remove_vertex(&V4));
    assert_eq!(f.g4.vertex_count(), 0);
}

#[test]
fn test_vertex_set() {
    let f = setup();
    assert_eq!(f.g1.vertices().copied().collect::<Vec<_>>(), [V1]);
    assert_eq!(f.g2.vertices().copied().collect::<Vec<_>>(), [V1, V2]);
    assert_eq!(f.g3.vertices().copied().collect::<Vec<_>>(), [V1, V2, V3]);
    assert_eq!(
        f.g4.vertices().copied().collect::<Vec<_>>(),
        [V1, V2, V3, V4]
    );
}

#[test]
fn test_reversed_view() {
    let mut g = G::simple();
    g.add_vertices([V1, V2]);
    let e = g.add_edge(&V1, &V2).unwrap().unwrap();
    {
        let r = EdgeReversed(&g);
        assert!(r.contains_vertex(&V1) && r.contains_vertex(&V2));
        assert_eq!(set(g.vertices().copied()), set(r.vertices().copied()));
        assert_eq!(set(g.edges().copied()), set(r.edges().copied()));
        assert!(r.contains_edge_between(&V2, &V1));
        assert_eq!(r.get_edge(&V2, &V1), Some(&e));
        assert!(!r.contains_edge_between(&V1, &V2));
        assert_eq!(r.get_edge(&V1, &V2), None);
        assert_eq!(r.edges_between(&V1, &V2).count(), 0);
        assert_eq!(r.edges_between(&V2, &V1).copied().collect::<Vec<_>>(), [e]);
        assert_eq!(
            [
                r.in_degree_of(&V1),
                r.in_degree_of(&V2),
                r.out_degree_of(&V1),
                r.out_degree_of(&V2)
            ],
            [1, 0, 0, 1]
        );
        assert_eq!(set(r.in_edges_of(&V1).copied()), set(g.edges().copied()));
        assert_eq!(r.out_edges_of(&V1).count(), 0);
        assert_eq!(r.in_edges_of(&V2).count(), 0);
        assert_eq!(set(r.out_edges_of(&V2).copied()), set(g.edges().copied()));
        assert_eq!(r.edge_source(&e), Some(&V2));
        assert_eq!(r.edge_target(&e), Some(&V1));
    }
    g.remove_edge(&e);
    let r = EdgeReversed(&g);
    assert_eq!(r.edge_count(), 0);
    for v in [V1, V2] {
        assert_eq!((r.in_degree_of(&v), r.out_degree_of(&v)), (0, 0));
        assert_eq!(r.in_edges_of(&v).count() + r.out_edges_of(&v).count(), 0);
    }
}
