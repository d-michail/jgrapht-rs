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

//! Port of `org.jgrapht.graph.GraphWalkTest`.
//!
//! Java constructs a (possibly inconsistent) walk and calls `verify()` afterwards; here
//! `GraphWalk::try_new` and friends validate on construction, so the "invalid" tests assert that
//! construction fails.

use jgrapht::generate::{Complete, GraphGenerator};
use jgrapht::*;

type W = GraphWalk<i32, DefaultEdge>;

#[test]
fn test_invalid_path_1() {
    let mut graph = UGraph::<i32>::pseudo();
    graph.add_vertex(0);
    graph.add_edge(&0, &0).unwrap();
    let r = W::try_new(
        &graph,
        Some(0),
        Some(0),
        Some(vec![0, 0]),
        Some(vec![]),
        0.0,
    );
    assert!(matches!(r, Err(GraphError::InvalidGraphWalk(_))));
}

#[test]
fn test_invalid_path_2() {
    let mut graph = UGraph::<i32>::pseudo();
    graph.add_vertex(0);
    let r = W::try_new(&graph, Some(0), Some(0), Some(vec![]), Some(vec![]), 0.0);
    assert!(matches!(r, Err(GraphError::InvalidGraphWalk(_))));
}

#[test]
fn test_invalid_path_3() {
    let mut graph = UGraph::<i32>::simple();
    graph.add_vertex(0);
    let r = W::try_new(&graph, Some(0), Some(0), Some(vec![0, 0]), None, 0.0);
    assert!(matches!(r, Err(GraphError::InvalidGraphWalk(_))));
}

#[test]
fn test_invalid_path_4() {
    let mut graph = UGraph::<i32>::simple();
    graph.add_vertices(0..4);
    for (a, b) in [(0, 1), (1, 2), (2, 3)] {
        graph.add_edge(&a, &b).unwrap();
    }
    let r = W::try_new(&graph, Some(0), Some(2), Some(vec![0, 1, 3, 2]), None, 0.0);
    assert!(matches!(r, Err(GraphError::InvalidGraphWalk(_))));
}

#[test]
fn test_invalid_path_5() {
    let mut graph = UGraph::<i32>::simple();
    graph.add_vertices(0..4);
    let e1 = graph.add_edge(&0, &1).unwrap().unwrap();
    graph.add_edge(&1, &2).unwrap();
    let e3 = graph.add_edge(&2, &3).unwrap().unwrap();
    let r = W::try_new(&graph, Some(0), Some(2), None, Some(vec![e1, e3]), 0.0);
    assert!(matches!(r, Err(GraphError::InvalidGraphWalk(_))));
}

#[test]
fn test_valid_paths() {
    let mut graph = UGraph::<i32>::simple();
    graph.add_vertex(0);
    assert!(W::try_new(&graph, None, None, Some(vec![]), Some(vec![]), 0.0).is_ok());
    assert!(W::try_new(&graph, None, None, None, Some(vec![]), 0.0).is_ok());
    assert!(W::try_new(&graph, None, None, Some(vec![]), None, 0.0).is_ok());
    let gw4 = W::try_new(&graph, Some(0), Some(0), Some(vec![0]), Some(vec![]), 0.0).unwrap();
    assert!(gw4.verify(&graph).is_ok());
    let gw5 = W::from_vertex_list(&graph, vec![0], 0.0).unwrap();
    assert!(gw5.verify(&graph).is_ok());
    // neither list given
    assert!(W::try_new(&graph, Some(0), Some(0), None, None, 0.0).is_err());
    // only one of start/end given
    assert!(W::try_new(&graph, Some(0), None, Some(vec![0]), None, 0.0).is_err());
}

#[test]
fn test_empty_path() {
    let graph = UGraph::<i32>::simple();
    let paths = [
        W::try_new(&graph, None, None, None, Some(vec![]), 0.0).unwrap(),
        W::from_vertex_list(&graph, vec![], 0.0).unwrap(),
    ];
    for path in paths {
        assert_eq!(path.length(), 0);
        assert!(path.vertex_list().is_empty());
        assert!(path.edge_list().is_empty());
        assert!(path.is_empty());
        assert_eq!(W::empty(), path);
    }
}

#[test]
fn test_non_simple_path() {
    let mut complete = UGraph::<i32>::simple();
    Complete(5).generate(&mut complete, |i| i as i32).unwrap();
    let vertex_list = vec![0, 1, 2, 3, 2, 3, 4];
    let edge_list: Vec<DefaultEdge> = vertex_list
        .windows(2)
        .map(|w| *complete.edge_between(&w[0], &w[1]).unwrap())
        .collect();

    let p1 = W::from_edge_list(&complete, 0, 4, edge_list.clone(), 10.0).unwrap();
    assert_eq!((p1.start_vertex(), p1.end_vertex()), (Some(&0), Some(&4)));
    assert_eq!(p1.vertex_list(), &vertex_list[..]);
    assert_eq!(p1.length(), edge_list.len());
    assert!((p1.weight() - 10.0).abs() < 1e-10);

    let p2 = W::from_vertex_list(&complete, vertex_list.clone(), 10.0).unwrap();
    assert_eq!((p2.start_vertex(), p2.end_vertex()), (Some(&0), Some(&4)));
    assert_eq!(p2.edge_list(), &edge_list[..]);
    assert_eq!(p2.length(), edge_list.len());
    assert!((p2.weight() - 10.0).abs() < 1e-10);
}

fn path_graph_weighted(directed_extra: bool) -> (UGraph<i32>, [DefaultEdge; 3]) {
    let _ = directed_extra;
    let mut graph = UGraph::<i32>::simple().weighted();
    graph.add_vertices(0..4);
    let e1 = graph.add_weighted_edge(&0, &1, 2.0).unwrap().unwrap();
    let e2 = graph.add_weighted_edge(&1, &2, 3.0).unwrap().unwrap();
    let e3 = graph.add_weighted_edge(&2, &3, 4.0).unwrap().unwrap();
    (graph, [e1, e2, e3])
}

#[test]
fn test_reverse_path_undirected() {
    let (graph, [e1, e2, e3]) = path_graph_weighted(false);
    let gw1 = W::from_vertex_list(&graph, vec![0, 1, 2, 3], 9.0).unwrap();
    let gw2 = W::from_edge_list(&graph, 0, 3, vec![e1, e2, e3], 9.0).unwrap();
    let rev1 = gw1.reverse(&graph, Some(&|_| gw1.weight())).unwrap();
    rev1.verify(&graph).unwrap();
    let rev2 = gw2.reverse(&graph, Some(&|_| gw2.weight())).unwrap();
    rev2.verify(&graph).unwrap();
    let rev_path = W::from_edge_list(&graph, 3, 0, vec![e3, e2, e1], 9.0).unwrap();
    assert_eq!(rev_path, rev1);
    assert_eq!(rev_path, rev2);
    let rev1 = gw1.reverse(&graph, None).unwrap();
    assert!((rev1.weight() - 9.0).abs() < 1e-10);
    let rev2 = gw2.reverse(&graph, None).unwrap();
    assert!((rev2.weight() - 9.0).abs() < 1e-10);
}

#[test]
fn test_reverse_invalid_path_directed() {
    let mut graph = DiGraph::<i32>::simple();
    graph.add_vertices(0..4);
    for (a, b) in [(0, 1), (1, 2), (2, 3)] {
        graph.add_edge(&a, &b).unwrap();
    }
    let gw1 = W::from_vertex_list(&graph, vec![0, 1, 2, 3], 0.0).unwrap();
    assert!(matches!(
        gw1.reverse(&graph, None),
        Err(GraphError::InvalidGraphWalk(_))
    ));
}

#[test]
fn test_reverse_path_directed() {
    let mut graph = DiGraph::<i32>::simple().weighted();
    graph.add_vertices(0..4);
    graph.add_weighted_edge(&0, &1, 1.0).unwrap();
    graph.add_weighted_edge(&1, &2, 2.0).unwrap();
    graph.add_weighted_edge(&2, &3, 3.0).unwrap();
    let e1 = graph.add_weighted_edge(&3, &2, 4.0).unwrap().unwrap();
    let e2 = graph.add_weighted_edge(&2, &1, 5.0).unwrap().unwrap();
    let e3 = graph.add_weighted_edge(&1, &0, 6.0).unwrap().unwrap();
    let gw1 = W::from_vertex_list(&graph, vec![0, 1, 2, 3], 0.0).unwrap();
    let sum = |gw: &W| {
        gw.edge_list()
            .iter()
            .map(|e| graph.edge_weight(e).unwrap())
            .sum::<f64>()
    };
    let rev1 = gw1.reverse(&graph, Some(&sum)).unwrap();
    rev1.verify(&graph).unwrap();
    let rev_path = W::from_edge_list(&graph, 3, 0, vec![e1, e2, e3], 15.0).unwrap();
    assert_eq!(rev_path, rev1);
    assert!((rev1.weight() - 15.0).abs() < 1e-8);
    let rev2 = gw1.reverse(&graph, None).unwrap();
    assert!((rev2.weight() - 15.0).abs() < 1e-8);
}

#[test]
fn test_illegal_concat_path_1() {
    let w0 = W::empty();
    let w1 = W::singleton(0, 10.0);
    assert!(matches!(
        w0.concat(&w1, &|_| 0.0),
        Err(GraphError::InvalidGraphWalk(_))
    ));
}

#[test]
fn test_illegal_concat_path_2() {
    let gw1 = W::singleton(0, 10.0);
    let gw2 = W::singleton(1, 12.0);
    assert!(matches!(
        gw1.concat(&gw2, &|_| gw1.weight() + gw2.weight()),
        Err(GraphError::InvalidGraphWalk(_))
    ));
}

#[test]
fn test_concat_path_1() {
    let mut graph = DiGraph::<i32>::simple().weighted();
    graph.add_vertices(0..4);
    graph.add_edge(&0, &1).unwrap();
    graph.add_edge(&1, &2).unwrap();
    let e3 = graph.add_edge(&2, &3).unwrap().unwrap();
    let e4 = graph.add_edge(&3, &1).unwrap().unwrap();
    let gw1 = W::from_vertex_list(&graph, vec![0, 1, 2], 5.0).unwrap();
    let gw2 = W::from_edge_list(&graph, 2, 1, vec![e3, e4], 7.0).unwrap();
    let gw3 = gw1.concat(&gw2, &|_| gw1.weight() + gw2.weight()).unwrap();
    gw3.verify(&graph).unwrap();
    let expected = W::from_vertex_list(&graph, vec![0, 1, 2, 3, 1], 12.0).unwrap();
    assert_eq!(expected, gw3);
    assert!((gw3.weight() - 12.0).abs() < 1e-8);
}

#[test]
fn test_concat_path_with_singleton() {
    let mut graph = DiGraph::<i32>::simple().weighted();
    graph.add_vertices(0..2);
    graph.add_edge(&0, &1).unwrap();
    let gw1 = W::from_vertex_list(&graph, vec![0, 1], 5.0).unwrap();
    let gw2 = W::singleton(1, 10.0);
    let gw3 = gw1.concat(&gw2, &|_| gw1.weight() + gw2.weight()).unwrap();
    gw3.verify(&graph).unwrap();
    assert_eq!(gw1, gw3);
}

#[test]
fn test_first_empty_walk_equality() {
    let gw1 = W::empty();
    let gw2 = W::singleton(0, 0.0);
    assert_ne!(gw1, gw2);
}

mod invariants {
    //! Rust-native: the walk's cardinality invariant is enforced by the safe constructor, and the
    //! fields cannot be set directly from outside the crate.
    use jgrapht::{DefaultEdge, GraphError, GraphWalk};

    fn e(i: u64) -> DefaultEdge {
        DefaultEdge(i)
    }

    #[test]
    fn from_parts_accepts_consistent_lists() {
        let empty = GraphWalk::<u32, DefaultEdge>::from_parts(vec![], vec![], 0.0).unwrap();
        assert!(empty.is_empty());
        let single = GraphWalk::<u32, DefaultEdge>::from_parts(vec![7], vec![], 0.0).unwrap();
        assert_eq!(single.vertex_list(), [7]);
        let walk = GraphWalk::from_parts(vec![1, 2, 3], vec![e(0), e(1)], 2.5).unwrap();
        assert_eq!(walk.length(), 2);
        assert_eq!(walk.weight(), 2.5);
        assert_eq!(walk.start_vertex(), Some(&1));
        assert_eq!(walk.end_vertex(), Some(&3));
    }

    #[test]
    fn from_parts_rejects_inconsistent_lists() {
        for (vertices, edges) in [
            (vec![1u32, 2], vec![]),
            (vec![1], vec![e(0)]),
            (vec![], vec![e(0)]),
            (vec![1, 2, 3, 4], vec![e(0)]),
        ] {
            assert!(matches!(
                GraphWalk::from_parts(vertices, edges, 0.0),
                Err(GraphError::InvalidGraphWalk(_))
            ));
        }
    }
}
