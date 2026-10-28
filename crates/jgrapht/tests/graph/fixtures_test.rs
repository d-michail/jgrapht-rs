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

//! Sanity checks for the adversarial fixtures in `common::fixtures` (API roadmap R0).
use crate::common::fixtures::{FixtureGraph, FullFixture, OutgoingFixture, weight_failure_cases};
use jgrapht::graph::{Graph, GraphExt, IncidenceGraph, IncomingGraph, OutgoingGraph};

#[test]
fn full_fixture_supports_every_query() {
    let g = FullFixture::new(
        &[0, 1, 2, 3],
        &[(0, 1, 1.0), (1, 2, 2.0), (2, 3, 3.0), (0, 2, 10.0)],
    );
    assert_eq!(IncidenceGraph::edges_of(&g, &2).count(), 3);
    assert_eq!(IncomingGraph::in_edges_of(&g, &2).count(), 2);
    assert_eq!(OutgoingGraph::out_edges_of(&g, &2).count(), 1);
    assert_eq!(IncidenceGraph::degree_of(&g, &0), 2);
}

#[test]
fn outgoing_fixture_enumerates_outgoing_edges() {
    let g = OutgoingFixture::new(&[0, 1, 2], &[(0, 1, 1.0), (0, 2, 1.0)]);
    assert_eq!(OutgoingGraph::out_edges_of(&g, &0).count(), 2);
}

#[test]
fn inconsistent_graph_breaks_each_integrity_rule() {
    let g = FixtureGraph::inconsistent();
    assert!(!g.contains_vertex(g.edge_target(&0).unwrap()));
    assert!(g.edge_source(&1).is_none());
    assert!(g.edge_weight(&2).is_none());
    assert!(g.edge_endpoints(&1).is_none());
}

#[test]
fn weight_failure_cases_are_unusable_weights() {
    for (label, w) in weight_failure_cases() {
        let g = FixtureGraph::with_weight(w);
        let stored = g.edge_weight(&0).unwrap();
        assert!(stored.is_nan() || stored < 0.0, "{label}");
    }
}
