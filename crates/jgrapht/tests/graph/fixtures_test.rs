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

//! Sanity checks for the adversarial fixtures in `common::fixtures` (API roadmap R0).
use crate::common::fixtures::{FixtureGraph, Profile, weight_failure_cases};
use jgrapht::GraphError;
use jgrapht::graph::{Graph, GraphExt};

#[test]
fn full_incidence_supports_every_query() {
    let g = FixtureGraph::diamond(Profile::FullIncidence);
    assert_eq!(g.edges_of(&2).unwrap().count(), 3);
    assert_eq!(g.in_edges_of(&2).unwrap().count(), 2);
    assert_eq!(g.out_edges_of(&2).unwrap().count(), 1);
    assert_eq!(g.degree_of(&0).unwrap(), 2);
}

#[test]
fn outgoing_only_rejects_incoming_queries() {
    let g = FixtureGraph::diamond(Profile::OutgoingOnly);
    assert_eq!(g.out_edges_of(&0).unwrap().count(), 2);
    assert!(matches!(g.edges_of(&0), Err(GraphError::Unsupported(_))));
    assert!(matches!(g.in_edges_of(&0), Err(GraphError::Unsupported(_))));
    assert!(matches!(g.degree_of(&0), Err(GraphError::Unsupported(_))));
}

#[test]
fn inconsistent_graph_breaks_each_integrity_rule() {
    let g = FixtureGraph::inconsistent();
    assert_eq!(g.profile(), Profile::Inconsistent);
    assert!(!g.contains_vertex(g.edge_target(&0).unwrap()));
    assert!(g.edge_source(&1).is_none());
    assert!(g.edge_weight(&2).is_none());
    assert!(g.edge_endpoints(&1).is_none());
}

#[test]
fn weight_failure_cases_are_unusable_weights() {
    for (label, w) in weight_failure_cases() {
        let g = FixtureGraph::with_weight(Profile::FullIncidence, w);
        let stored = g.edge_weight(&0).unwrap();
        assert!(stored.is_nan() || stored < 0.0, "{label}");
    }
}
