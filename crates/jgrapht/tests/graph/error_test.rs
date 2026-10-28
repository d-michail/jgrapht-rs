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

//! The typed `GraphError` variants (API roadmap R8): callers branch on variants and payloads, never
//! on message text. The `Display` strings are pinned only so that a rename is a deliberate change.
use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
use jgrapht::graph::props::{require_directed, require_undirected, require_weighted};
use jgrapht::{
    DiGraph, GraphError, GraphMutExt, GraphProperty, ParameterRequirement, UGraph, WeightProblem,
};

fn weighted_edge(weight: f64) -> DiGraph<i32> {
    let mut g = DiGraph::<i32>::simple().weighted();
    g.add_vertices([1, 2]);
    g.add_weighted_edge(&1, &2, weight).unwrap();
    g
}

#[test]
fn unusable_weights_are_reported_by_kind() {
    for weight in [-1.0, f64::NAN] {
        let g = weighted_edge(weight);
        assert_eq!(
            Dijkstra::new(&g).path_weight(&1, &2),
            Err(GraphError::InvalidWeight(WeightProblem::NegativeOrNaN))
        );
    }
}

#[test]
fn graph_property_requirements_name_the_property() {
    let directed = DiGraph::<i32>::simple();
    let undirected = UGraph::<i32>::simple();
    assert_eq!(
        require_directed(&undirected).err(),
        Some(GraphError::InvalidGraph(GraphProperty::Directed))
    );
    assert_eq!(
        require_undirected(&directed).err(),
        Some(GraphError::InvalidGraph(GraphProperty::Undirected))
    );
    assert_eq!(
        require_weighted(&directed).err(),
        Some(GraphError::InvalidGraph(GraphProperty::Weighted))
    );
}

#[test]
fn display_messages() {
    let cases: [(GraphError, &str); 7] = [
        (
            GraphError::InvalidWeight(WeightProblem::Negative),
            "invalid edge weight: negative",
        ),
        (
            GraphError::InvalidGraph(GraphProperty::Simple),
            "graph must be simple",
        ),
        (
            GraphError::must_be_positive("tolerance"),
            "parameter `tolerance` must be positive",
        ),
        (
            GraphError::InvalidParameter {
                name: "k",
                requirement: ParameterRequirement::AtLeast(2),
            },
            "parameter `k` must be at least 2",
        ),
        (GraphError::SourceEqualsSink, "source and sink must differ"),
        (GraphError::Infeasible, "problem is infeasible"),
        (GraphError::Unbounded, "problem is unbounded"),
    ];
    for (error, text) in cases {
        assert_eq!(error.to_string(), text);
    }
}
