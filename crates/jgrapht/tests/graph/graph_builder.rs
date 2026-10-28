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

//! Ports of `graph/builder/GraphBuilderTest` and `graph/builder/GraphTypeBuilderTest`.

use jgrapht::graph::builder::{GraphBuilder, GraphTypeBuilder};
use jgrapht::graph::{Directed, Undirected};
use jgrapht::*;

const V1: &str = "v1";
const V2: &str = "v2";
const V3: &str = "v3";
const V4: &str = "v4";
const V5: &str = "v5";
const V6: &str = "v6";
const V7: &str = "v7";
const V8: &str = "v8";

#[test]
fn test_add_vertex() {
    let g = GraphBuilder::new(DiGraph::<&str>::with_loops())
        .add_vertex(V1)
        .add_vertices([V2, V3])
        .build()
        .unwrap();
    assert_eq!((g.vertex_count(), g.edge_count()), (3, 0));
    assert!([V1, V2, V3].iter().all(|v| g.contains_vertex(v)));
}

#[test]
fn test_add_edge() {
    let (e1, e2) = (DefaultEdge::new(), DefaultEdge::new());
    let g = GraphBuilder::new(DiGraph::<&str>::with_loops().weighted())
        .add_edge(V1, V2)
        .add_edge_chain([V3, V4, V5, V6])
        .add_weighted_edge(V7, V8, 10.0)
        .add_edge_with(V1, V7, e1)
        .add_weighted_edge_with(V1, V8, e2, 42.0)
        .build_as_unmodifiable()
        .unwrap();
    assert_eq!((g.vertex_count(), g.edge_count()), (8, 7));
    for (a, b) in [
        (V1, V2),
        (V3, V4),
        (V4, V5),
        (V5, V6),
        (V7, V8),
        (V1, V7),
        (V1, V8),
    ] {
        assert!(g.contains_edge_between(&a, &b));
    }
    assert_eq!(g.edge_between(&V1, &V7), Some(&e1));
    assert_eq!(g.edge_between(&V1, &V8), Some(&e2));
    assert_eq!(g.edge_weight(g.edge_between(&V7, &V8).unwrap()), Some(10.0));
    assert_eq!(g.edge_weight(g.edge_between(&V1, &V8).unwrap()), Some(42.0));
}

#[test]
fn test_add_graph() {
    let g1 = GraphBuilder::new(DiGraph::<&str>::with_loops())
        .add_vertex(V1)
        .add_edge(V2, V3)
        .build_as_unmodifiable()
        .unwrap();
    let g2 = GraphBuilder::new(DiGraph::<&str>::with_loops())
        .add_graph(&g1)
        .add_edge(V1, V4)
        .build()
        .unwrap();
    assert_eq!((g2.vertex_count(), g2.edge_count()), (4, 2));
    assert!([V1, V2, V3, V4].iter().all(|v| g2.contains_vertex(v)));
    assert!(g2.contains_edge_between(&V2, &V3) && g2.contains_edge_between(&V1, &V4));
}

#[test]
fn test_remove_vertex() {
    let g1 = GraphBuilder::new(DiGraph::<&str>::with_loops())
        .add_edge(V1, V3)
        .add_edge_chain([V2, V3, V4, V5])
        .build_as_unmodifiable()
        .unwrap();
    let g2 = GraphBuilder::new(DiGraph::<&str>::with_loops())
        .add_graph(&g1)
        .remove_vertex(&V2)
        .remove_vertices([&V4, &V5])
        .build()
        .unwrap();
    assert_eq!((g2.vertex_count(), g2.edge_count()), (2, 1));
    assert!(g2.contains_vertex(&V1) && g2.contains_vertex(&V3));
    assert!(g2.contains_edge_between(&V1, &V3));
}

#[test]
fn test_remove_edge() {
    let e = DefaultEdge::new();
    let g1 = GraphBuilder::new(DiGraph::<&str>::with_loops())
        .add_edge_chain([V1, V2, V3, V4])
        .add_edge_with(V1, V4, e)
        .build_as_unmodifiable()
        .unwrap();
    let g2 = GraphBuilder::new(DiGraph::<&str>::with_loops())
        .add_graph(&g1)
        .remove_edge_between(&V2, &V3)
        .remove_edge(&e)
        .build()
        .unwrap();
    assert_eq!((g2.vertex_count(), g2.edge_count()), (4, 2));
    assert!(g2.contains_edge_between(&V1, &V2) && g2.contains_edge_between(&V3, &V4));
}

#[test]
fn test_add_vertex_pseudograph() {
    let g = GraphBuilder::new(UGraph::<&str>::pseudo())
        .add_vertex(V1)
        .build()
        .unwrap();
    assert_eq!((g.vertex_count(), g.edge_count()), (1, 0));
}

#[test]
fn builder_reports_the_first_failure() {
    let r = GraphBuilder::new(UGraph::<&str>::simple())
        .add_edge(V1, V1)
        .add_edge(V1, V2)
        .build();
    assert_eq!(r.err(), Some(GraphError::LoopsNotAllowed));
}

#[test]
fn test_graph_type_builder() {
    let graph = GraphTypeBuilder::<i32, DefaultEdge, Directed>::directed()
        .allowing_multiple_edges(true)
        .allowing_self_loops(true)
        .default_edges()
        .build_graph();
    let t = graph.graph_type();
    assert!(t.directed && t.multiple_edges && t.self_loops);
    assert!(graph.edge_factory().is_some());
    assert!(graph.vertex_factory().is_none());
}

#[test]
fn test_graph_type_builder_with_edge_supplier() {
    let graph = GraphTypeBuilder::<i32, DefaultEdge, Directed>::directed()
        .allowing_multiple_edges(true)
        .allowing_self_loops(true)
        .edge_supplier(|_| DefaultEdge::new())
        .vertex_supplier(|i| i as i32)
        .build_graph();
    let t = graph.graph_type();
    assert!(t.directed && t.multiple_edges && t.self_loops);
    assert!(graph.edge_factory().is_some() && graph.vertex_factory().is_some());
}

#[test]
fn test_graph_type_builder_undirected() {
    let graph = GraphTypeBuilder::<i32, DefaultEdge, Undirected>::undirected()
        .allowing_multiple_edges(true)
        .allowing_self_loops(false)
        .default_edges()
        .build_graph();
    let t = graph.graph_type();
    assert!(t.is_undirected() && t.multiple_edges && !t.self_loops);
    assert!(graph.edge_factory().is_some());
}

#[test]
fn test_graph_type_builder_from_graph() {
    let graph = UGraph::<i32>::pseudo();
    let graph1 = GraphTypeBuilder::for_graph(&graph).build_graph();
    let t = graph1.graph_type();
    assert!(t.is_undirected() && t.multiple_edges && t.self_loops);
    assert!(graph1.edge_factory().is_some());
    assert_eq!(
        graph.edge_factory().map(|f| f as usize),
        graph1.edge_factory().map(|f| f as usize)
    );
}
