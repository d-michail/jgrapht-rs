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

//! Error behavior of the traversals (API roadmap R3): invalid start vertices and radii are rejected
//! when the iterator is built, and errors that can only arise while iterating are items of the
//! iterator, never a silent end of iteration or a panic.
use jgrapht::traverse::*;
use jgrapht::*;
use rand::{SeedableRng, rngs::StdRng};

fn dg(edges: &[(u32, u32)]) -> DiGraph<u32> {
    let mut g = DiGraph::<u32>::simple();
    for &(a, b) in edges {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    g
}

fn weighted_path(weights: &[f64]) -> DiGraph<u32> {
    let mut g = DiGraph::<u32>::simple().weighted();
    for (i, &w) in weights.iter().enumerate() {
        let (a, b) = (i as u32, i as u32 + 1);
        g.add_edge_with_vertices(a, b).unwrap();
        let e = *g.edge_between(&a, &b).unwrap();
        g.set_edge_weight(&e, w).unwrap();
    }
    g
}

const BAD: GraphError = GraphError::InvalidWeight(WeightProblem::NegativeOrNaN);

#[test]
fn start_vertex_must_be_a_vertex() {
    let g = dg(&[(1, 2)]);
    assert!(matches!(Bfs::new(&g, &9), Err(GraphError::VertexNotFound)));
    assert!(matches!(Dfs::new(&g, &9), Err(GraphError::VertexNotFound)));
    assert!(matches!(
        DfsPostOrder::new(&g, &9),
        Err(GraphError::VertexNotFound)
    ));
    assert!(matches!(
        ClosestFirst::new(&weighted_path(&[1.0]), &9),
        Err(GraphError::VertexNotFound)
    ));
    assert!(matches!(
        RandomWalk::new(&g, &9, 3, false, StdRng::seed_from_u64(1)),
        Err(GraphError::VertexNotFound)
    ));
    assert!(matches!(
        BreadthFirstIterator::from_start(&g, &9),
        Err(GraphError::VertexNotFound)
    ));
    assert!(matches!(
        RandomWalkVertexIterator::new(&g, &9, 3, false, StdRng::seed_from_u64(1)),
        Err(GraphError::VertexNotFound)
    ));
}

#[test]
fn radius_must_be_non_negative_and_not_nan() {
    let g = weighted_path(&[1.0, 1.0]);
    let invalid = |name| GraphError::must_be_non_negative(name);
    for radius in [-1.0, f64::NAN] {
        assert_eq!(
            ClosestFirst::with_radius(&g, &0, radius).err(),
            Some(invalid("radius"))
        );
        assert_eq!(
            ClosestFirstIterator::from_start_with_radius(&g, &0, radius).err(),
            Some(invalid("radius"))
        );
    }
    assert_eq!(
        ClosestFirstIterator::from_start_with_radius(&g, &0, 1.0)
            .unwrap()
            .count(),
        2
    );
}

#[test]
fn closest_first_reports_an_invalid_weight_once_and_ends() {
    for weight in [-1.0, f64::NAN] {
        let g = weighted_path(&[1.0, weight, 1.0]);

        let mut it = ClosestFirstIterator::from_start(&g, &0).unwrap();
        // the weight of the edge leaving vertex 1 is found while expanding it, so the error
        // takes the place of vertex 1
        assert_eq!(it.next(), Some(Ok(0)));
        assert_eq!(it.next(), Some(Err(BAD)));
        assert_eq!(it.next(), None);

        let mut simple = ClosestFirst::new(&g, &0).unwrap();
        assert!(matches!(simple.next(), Some(Ok((&0, _)))));
        assert_eq!(simple.next(), Some(Err(BAD)));
        assert_eq!(simple.next(), None);
    }
}

#[test]
fn closest_first_collects_into_a_result() {
    let good = weighted_path(&[1.0, 2.0]);
    let all: jgrapht::Result<Vec<_>> = ClosestFirstIterator::from_start(&good, &0)
        .unwrap()
        .collect();
    assert_eq!(all.unwrap(), [0, 1, 2]);

    let bad = weighted_path(&[1.0, -2.0]);
    let all: jgrapht::Result<Vec<_>> = ClosestFirstIterator::from_start(&bad, &0)
        .unwrap()
        .collect();
    assert_eq!(all, Err(BAD));
}

#[test]
fn weighted_random_walks_report_an_invalid_weight() {
    let g = weighted_path(&[1.0, -1.0]);

    let mut walk = RandomWalk::new(&g, &1, 5, true, StdRng::seed_from_u64(7)).unwrap();
    assert_eq!(walk.next(), Some(Err(BAD)));
    assert_eq!(walk.next(), None);

    let mut walk =
        RandomWalkVertexIterator::new(&g, &1, 5, true, StdRng::seed_from_u64(7)).unwrap();
    assert_eq!(walk.next(), Some(Err(BAD)));
    assert_eq!(walk.next(), None);

    // an unweighted walk ignores weights entirely
    let mut walk =
        RandomWalkVertexIterator::new(&g, &0, 5, false, StdRng::seed_from_u64(7)).unwrap();
    assert!(walk.all(|v| v.is_ok()));
}

#[test]
fn topological_order_reports_a_cycle_as_an_error() {
    let g = dg(&[(1, 2), (2, 3), (3, 2), (3, 4)]);
    let items: Vec<_> = TopologicalOrderIterator::new(&g).unwrap().collect();
    assert_eq!(
        items,
        [Ok(1), Err(GraphError::InvalidGraph(GraphProperty::Acyclic))]
    );
    let sorted: jgrapht::Result<Vec<_>> = TopologicalOrderIterator::new(&g).unwrap().collect();
    assert!(sorted.is_err());
}

#[test]
fn closing_a_cyclic_graph_as_a_dag_fails_instead_of_stopping_early() {
    let mut g = dg(&[(1, 2), (2, 3), (3, 2)]);
    let closure = jgrapht::alg::transitive::TransitiveClosure::INSTANCE;
    assert_eq!(
        closure.close_directed_acyclic_graph(&mut g),
        Err(GraphError::InvalidGraph(GraphProperty::Acyclic))
    );
}

#[test]
fn closest_first_distance_is_absent_for_unseen_vertices() {
    let g = weighted_path(&[1.0, 2.0, 4.0]);
    let mut it = ClosestFirstIterator::from_start(&g, &0).unwrap();
    assert_eq!(it.shortest_path_length(&2), None);
    it.by_ref().for_each(drop);
    assert_eq!(it.shortest_path_length(&0), Some(0.0));
    assert_eq!(it.shortest_path_length(&3), Some(7.0));
    assert_eq!(it.shortest_path_length(&99), None);
}
