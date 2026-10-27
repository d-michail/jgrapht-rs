//! Ports of `org.jgrapht.alg.TransitiveClosureTest` and `org.jgrapht.alg.TransitiveReductionTest`.
//! `testReduceNull` (null argument) has no Rust counterpart.

use jgrapht::alg::transitive::{BitSet, TransitiveClosure, TransitiveReduction};
use jgrapht::generate::{CompleteBipartite, Empty, GraphGenerator, Linear, Ring};
use jgrapht::*;

type G = DiGraph<i64>;

fn integer_graph() -> G {
    G::simple().with_vertex_factory(|i| i as i64)
}

fn integer_dag() -> G {
    G::dag().with_vertex_factory(|i| i as i64)
}

#[test]
fn test_linear_graph() {
    let mut graph = integer_graph();
    let n = 10;
    Linear(n).generate_graph(&mut graph, None).unwrap();
    TransitiveClosure::INSTANCE
        .close_simple_directed_graph(&mut graph)
        .unwrap();
    assert_eq!(n * (n - 1) / 2, graph.edge_count());
    for i in 0..n as i64 {
        for j in i + 1..n as i64 {
            assert!(graph.get_edge(&i, &j).is_some());
        }
    }
}

#[test]
fn test_ring_graph() {
    let mut graph = integer_graph();
    let n = 10;
    Ring(n).generate_graph(&mut graph, None).unwrap();
    TransitiveClosure::INSTANCE
        .close_simple_directed_graph(&mut graph)
        .unwrap();
    assert_eq!(n * (n - 1), graph.edge_count());
    for i in 0..n as i64 {
        for j in 0..n as i64 {
            assert!(i == j || graph.get_edge(&i, &j).is_some());
        }
    }
}

#[test]
fn test_no_vertices_dag() {
    let mut graph = integer_dag();
    TransitiveClosure::INSTANCE
        .close_directed_acyclic_graph(&mut graph)
        .unwrap();
    assert_eq!(0, graph.edge_count());
}

#[test]
fn test_empty_dag() {
    let mut graph = integer_dag();
    Empty(10).generate_graph(&mut graph, None).unwrap();
    TransitiveClosure::INSTANCE
        .close_directed_acyclic_graph(&mut graph)
        .unwrap();
    assert_eq!(0, graph.edge_count());
}

#[test]
fn test_complete_bipartite_dag() {
    let mut graph = integer_dag();
    CompleteBipartite(5, 5)
        .generate_graph(&mut graph, None)
        .unwrap();
    TransitiveClosure::INSTANCE
        .close_directed_acyclic_graph(&mut graph)
        .unwrap();
    assert_eq!(25, graph.edge_count());
}

#[test]
fn test_linear_graph_for_dag() {
    let mut graph = integer_dag();
    let n = 10;
    Linear(n).generate_graph(&mut graph, None).unwrap();
    TransitiveClosure::INSTANCE
        .close_directed_acyclic_graph(&mut graph)
        .unwrap();
    assert_eq!(n * (n - 1) / 2, graph.edge_count());
    for i in 0..n as i64 {
        for j in i + 1..n as i64 {
            assert!(graph.get_edge(&i, &j).is_some());
        }
    }
}

/// Not in Java: the simple-graph closure rejects other graph types (Java's parameter type).
#[test]
fn closure_rejects_non_simple_graphs() {
    let mut g = DiGraph::<u32>::multi();
    g.add_edge_with_vertices(1, 2).unwrap();
    assert!(
        TransitiveClosure::INSTANCE
            .close_simple_directed_graph(&mut g)
            .is_err()
    );
}

// ---- TransitiveReductionTest ------------------------------------------------------------------

const MATRIX: [[u8; 5]; 5] = [
    [0, 1, 1, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 1, 1],
    [0, 0, 0, 0, 1],
    [0, 1, 0, 0, 0],
];

const EXPECTED_TRANSITIVELY_REDUCED_MATRIX: [[u8; 5]; 5] = [
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 1, 0],
    [0, 0, 0, 0, 1],
    [0, 1, 0, 0, 0],
];

fn as_bit_set_array(m: &[[u8; 5]; 5]) -> Vec<BitSet> {
    m.iter()
        .map(|row| {
            let mut b = BitSet::new(row.len());
            for (j, &x) in row.iter().enumerate() {
                b.set(j, x == 1);
            }
            b
        })
        .collect()
}

fn as_int_array(b: &[BitSet]) -> Vec<Vec<u8>> {
    (0..b.len())
        .map(|i| (0..b.len()).map(|j| u8::from(b[i].get(j))).collect())
        .collect()
}

fn rows(m: &[[u8; 5]; 5]) -> Vec<Vec<u8>> {
    m.iter().map(|r| r.to_vec()).collect()
}

#[test]
fn test_internals() {
    let expected_path_matrix: [[u8; 5]; 5] = [
        [0, 1, 1, 1, 1],
        [0, 0, 0, 0, 0],
        [0, 1, 0, 1, 1],
        [0, 1, 0, 0, 1],
        [0, 1, 0, 0, 0],
    ];
    let mut bits = as_bit_set_array(&MATRIX);
    TransitiveReduction::transform_to_path_matrix(&mut bits);
    assert_eq!(rows(&expected_path_matrix), as_int_array(&bits));

    TransitiveReduction::transitive_reduction(&mut bits);
    assert_eq!(
        rows(&EXPECTED_TRANSITIVELY_REDUCED_MATRIX),
        as_int_array(&bits)
    );
}

type SG = DiGraph<&'static str>;

fn string_graph(vertices: &[&'static str], edges: &[(&'static str, &'static str)]) -> SG {
    let mut g = SG::simple();
    for v in vertices {
        g.add_vertex(*v);
    }
    for (a, b) in edges {
        g.add_edge(a, b).unwrap();
    }
    g
}

#[test]
fn test_reduce_no_vertex_no_edge() {
    let mut graph = string_graph(&[], &[]);
    TransitiveReduction::INSTANCE.reduce(&mut graph).unwrap();
    assert_eq!(0, graph.vertex_count());
    assert_eq!(0, graph.edge_count());
}

#[test]
fn test_reduce_some_vertices_no_edge() {
    let mut graph = string_graph(&["x", "y", "z"], &[]);
    TransitiveReduction::INSTANCE.reduce(&mut graph).unwrap();
    assert_eq!(3, graph.vertex_count());
    assert_eq!(0, graph.edge_count());
}

#[test]
fn test_reduce_already_reduced() {
    let mut graph = string_graph(&["x", "y", "z"], &[("x", "y"), ("y", "z")]);
    assert_eq!(3, graph.vertex_count());
    assert_eq!(2, graph.edge_count());
    TransitiveReduction::INSTANCE.reduce(&mut graph).unwrap();
    assert_eq!(3, graph.vertex_count());
    assert_eq!(2, graph.edge_count());
    assert!(graph.contains_edge_between(&"x", &"y"));
    assert!(graph.contains_edge_between(&"y", &"z"));
    assert!(!graph.contains_edge_between(&"x", &"z"));
}

#[test]
fn test_reduce_basic() {
    let mut graph = string_graph(&["x", "y", "z"], &[("x", "y"), ("y", "z"), ("x", "z")]);
    assert_eq!(3, graph.edge_count());
    TransitiveReduction::INSTANCE.reduce(&mut graph).unwrap();
    assert_eq!(3, graph.vertex_count());
    assert_eq!(2, graph.edge_count());
    assert!(graph.contains_edge_between(&"x", &"y"));
    assert!(graph.contains_edge_between(&"y", &"z"));
    assert!(!graph.contains_edge_between(&"x", &"z"));
}

#[test]
fn test_reduce_far_away() {
    let mut graph = string_graph(
        &["a", "b", "c", "x", "y", "z"],
        &[
            ("a", "b"),
            ("b", "c"),
            ("c", "x"),
            ("x", "y"),
            ("y", "z"),
            ("a", "z"),
        ],
    );
    assert_eq!(6, graph.vertex_count());
    assert_eq!(6, graph.edge_count());
    TransitiveReduction::INSTANCE.reduce(&mut graph).unwrap();
    assert_eq!(6, graph.vertex_count());
    assert_eq!(5, graph.edge_count());
    for (a, b) in [("a", "b"), ("b", "c"), ("c", "x"), ("x", "y"), ("y", "z")] {
        assert!(graph.contains_edge_between(&a, &b));
    }
    assert!(!graph.contains_edge_between(&"a", &"z"));
}

#[test]
fn test_reduce_canonical_graph() {
    let mut graph = DiGraph::<usize>::simple();
    for i in 0..MATRIX.len() {
        graph.add_vertex(i);
    }
    for (i, row) in MATRIX.iter().enumerate() {
        for (j, &x) in row.iter().enumerate() {
            if x == 1 {
                graph.add_edge(&i, &j).unwrap();
            }
        }
    }
    assert!(!graph.contains_edge_between(&0, &0));
    assert!(graph.contains_edge_between(&0, &1));
    assert!(graph.contains_edge_between(&2, &4));
    assert!(graph.contains_edge_between(&4, &1));
    assert_eq!(5, graph.vertex_count());
    assert_eq!(6, graph.edge_count());

    TransitiveReduction::INSTANCE.reduce(&mut graph).unwrap();
    assert_eq!(5, graph.vertex_count());
    assert_eq!(4, graph.edge_count());
    assert!(!graph.contains_edge_between(&0, &0));
    assert!(!graph.contains_edge_between(&0, &1));
    assert!(!graph.contains_edge_between(&2, &4));
    assert!(graph.contains_edge_between(&4, &1));

    let mut actual = vec![vec![0u8; 5]; 5];
    for e in graph.edges() {
        let (u, v) = graph.edge_endpoints(e).unwrap();
        actual[*u][*v] = 1;
    }
    assert_eq!(rows(&EXPECTED_TRANSITIVELY_REDUCED_MATRIX), actual);
}

/// Not in Java: undirected graphs are rejected (`requireDirected`).
#[test]
fn reduce_rejects_undirected_graphs() {
    let mut g = UGraph::<u32>::simple();
    g.add_edge_with_vertices(1, 2).unwrap();
    assert!(TransitiveReduction::INSTANCE.reduce(&mut g).is_err());
}
