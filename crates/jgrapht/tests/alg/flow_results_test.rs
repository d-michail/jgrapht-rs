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

//! Rust-native tests of the flow result objects (API roadmap R5): a run returns everything it
//! produced, owned, and there is no state to query in the wrong order.
use jgrapht::alg::flow::{
    Dinic, EdmondsKarp, GusfieldEquivalentFlowTree, GusfieldGomoryHuCutTree, MaximumFlowAlgorithm,
    MaximumFlowValueAlgorithm, MinimumStCutAlgorithm, PushRelabel,
};
use jgrapht::{DiGraph, Graph, GraphError, GraphMut, GraphMutExt, UGraph, WeightProblem};

/// 0 -> 1 -> 3 and 0 -> 2 -> 3, with a cross edge 1 -> 2; the maximum flow from 0 to 3 is 5.
fn network() -> DiGraph<u32> {
    let mut g = DiGraph::<u32>::simple().weighted();
    for (a, b, c) in [
        (0, 1, 3.0),
        (0, 2, 2.0),
        (1, 3, 2.0),
        (2, 3, 3.0),
        (1, 2, 1.0),
    ] {
        g.add_edge_with_vertices(a, b).unwrap();
        let e = *g.edge_between(&a, &b).unwrap();
        g.set_edge_weight(&e, c).unwrap();
    }
    g
}

#[test]
fn a_result_outlives_the_next_run_of_the_same_algorithm() {
    let g = network();
    let mut dinic = Dinic::new(&g).unwrap();
    let first = dinic.maximum_flow(&0, &3).unwrap();
    let second = dinic.maximum_flow(&0, &1).unwrap();
    assert_eq!(first.value(), 5.0);
    assert_eq!(second.value(), 3.0);
    // the first result still describes the first run
    assert_eq!(first.flow_map().len(), 5);
    assert!(first.source_partition().contains(&0));
    assert!(first.sink_partition().contains(&3));
}

#[test]
fn the_cut_certifies_the_flow() {
    let g = network();
    for result in [
        Dinic::new(&g).unwrap().maximum_flow(&0, &3).unwrap(),
        EdmondsKarp::new(&g).unwrap().maximum_flow(&0, &3).unwrap(),
        PushRelabel::new(&g).unwrap().maximum_flow(&0, &3).unwrap(),
    ] {
        assert_eq!(result.value(), 5.0);
        assert_eq!(result.cut_capacity(), result.value());
        let crossing: f64 = result
            .cut_edges()
            .iter()
            .map(|e| g.edge_weight(e).unwrap())
            .sum();
        assert_eq!(crossing, result.value());
        assert!(
            result
                .source_partition()
                .is_disjoint(result.sink_partition())
        );
        assert_eq!(
            result.source_partition().len() + result.sink_partition().len(),
            4
        );
        for e in g.edges() {
            let flow = result.flow(e).unwrap();
            assert!((0.0..=g.edge_weight(e).unwrap()).contains(&flow));
            assert!(result.flow_direction(e).is_some());
        }
    }
}

#[test]
fn minimum_cut_is_the_cut_of_the_flow() {
    let g = network();
    let cut = Dinic::new(&g).unwrap().minimum_cut(&0, &3).unwrap();
    let flow = Dinic::new(&g).unwrap().maximum_flow(&0, &3).unwrap();
    assert_eq!(cut.capacity(), flow.value());
    assert_eq!(cut.source_partition(), flow.source_partition());
    assert_eq!(cut.cut_edges(), flow.cut_edges());
}

#[test]
fn bad_terminals_and_capacities_are_typed_errors() {
    let g = network();
    let mut dinic = Dinic::new(&g).unwrap();
    assert_eq!(
        dinic.maximum_flow(&0, &99).err(),
        Some(GraphError::VertexNotFound)
    );
    assert_eq!(
        dinic.maximum_flow(&99, &0).err(),
        Some(GraphError::VertexNotFound)
    );
    assert_eq!(
        dinic.maximum_flow_value(&2, &2).err(),
        Some(GraphError::SourceEqualsSink)
    );

    let mut negative = DiGraph::<u32>::simple().weighted();
    negative.add_edge_with_vertices(0, 1).unwrap();
    let e = *negative.edge_between(&0, &1).unwrap();
    negative.set_edge_weight(&e, -1.0).unwrap();
    assert_eq!(
        Dinic::new(&negative).err(),
        Some(GraphError::InvalidWeight(WeightProblem::Negative))
    );
}

#[test]
fn equivalent_flow_trees_answer_values_through_the_narrow_trait() {
    // a path 0 - 1 - 2 with capacities 4 and 2
    let mut g = UGraph::<u32>::simple().weighted();
    for (a, b, c) in [(0, 1, 4.0), (1, 2, 2.0)] {
        g.add_edge_with_vertices(a, b).unwrap();
        let e = *g.edge_between(&a, &b).unwrap();
        g.set_edge_weight(&e, c).unwrap();
    }
    fn value<A: MaximumFlowValueAlgorithm<u32>>(a: &mut A, s: u32, t: u32) -> f64 {
        a.maximum_flow_value(&s, &t).unwrap()
    }
    let mut eft = GusfieldEquivalentFlowTree::new(&g).unwrap();
    assert_eq!(value(&mut eft, 0, 2), 2.0);
    assert_eq!(value(&mut eft, 0, 1), 4.0);
    assert_eq!(
        eft.maximum_flow_value(&0, &99).err(),
        Some(GraphError::VertexNotFound)
    );

    let mut ght = GusfieldGomoryHuCutTree::new(&g).unwrap();
    assert_eq!(value(&mut ght, 1, 2), 2.0);
    let cut = ght.minimum_cut(&0, &2).unwrap();
    assert_eq!(cut.capacity(), 2.0);
    assert!(cut.source_partition().contains(&0));
    assert!(cut.sink_partition().contains(&2));
    assert_eq!(cut.cut_edges().len(), 1);
    let global = ght.global_minimum_cut().unwrap();
    assert_eq!(global.capacity(), 2.0);
}
