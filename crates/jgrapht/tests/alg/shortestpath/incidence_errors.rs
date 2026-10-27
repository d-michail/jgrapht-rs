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

use jgrapht::alg::shortestpath::{
    ALTAdmissibleHeuristic, BidirectionalAStarShortestPath, BidirectionalDijkstraShortestPath,
    BoundedPrunedYenKShortestPath, DijkstraManyToManyShortestPaths, EppsteinShortestPathIterator,
    ShortestPathAlgorithm,
};
use jgrapht::graph::sparse::{IncomingEdgesSupport, SparseIntDirectedGraph};
use jgrapht::graph::{DirectedGraph, Graph, GraphExt};
use jgrapht::{GraphError, GraphType, Result};

fn no_incoming_graph() -> SparseIntDirectedGraph {
    SparseIntDirectedGraph::with_incoming_support(
        4,
        &[(0, 1), (1, 2), (0, 2), (2, 3)],
        IncomingEdgesSupport::NoIncomingEdges,
    )
    .unwrap()
}

fn assert_unsupported<T>(result: Result<T>) {
    assert!(matches!(result, Err(GraphError::Unsupported(_))));
}

#[test]
fn reverse_searches_propagate_missing_incoming_index() {
    let graph = no_incoming_graph();

    assert_unsupported(graph.try_incoming_of(&2));
    assert_unsupported(ALTAdmissibleHeuristic::new(&graph, [0]));
    assert_unsupported(BidirectionalDijkstraShortestPath::new(&graph).path(&0, &3));
    assert_unsupported(
        BidirectionalAStarShortestPath::new(&graph, |_: &usize, _: &usize| 0.0).path(&0, &3),
    );
    assert_unsupported(
        DijkstraManyToManyShortestPaths::new(&graph).get_many_to_many_paths([0, 1], [3]),
    );
    assert_unsupported(BoundedPrunedYenKShortestPath::new(&graph).get_paths(&0, &3, 2));
}

/// Eppstein accepts only simple directed graphs, while the sparse backend deliberately reports
/// pseudograph capabilities. This wrapper narrows only the advertised type so the regression can
/// exercise Eppstein's reverse-tree construction against the same missing incoming index.
struct SimpleSparse(SparseIntDirectedGraph);

impl Graph for SimpleSparse {
    type Vertex = usize;
    type Edge = usize;

    fn graph_type(&self) -> GraphType {
        GraphType::simple().directed(true).as_unmodifiable()
    }

    fn vertices(&self) -> impl Iterator<Item = &Self::Vertex> {
        self.0.vertices()
    }

    fn edges(&self) -> impl Iterator<Item = &Self::Edge> {
        self.0.edges()
    }

    fn vertex_count(&self) -> usize {
        self.0.vertex_count()
    }

    fn edge_count(&self) -> usize {
        self.0.edge_count()
    }

    fn contains_vertex(&self, vertex: &Self::Vertex) -> bool {
        self.0.contains_vertex(vertex)
    }

    fn contains_edge(&self, edge: &Self::Edge) -> bool {
        self.0.contains_edge(edge)
    }

    fn edge_source(&self, edge: &Self::Edge) -> Option<&Self::Vertex> {
        self.0.edge_source(edge)
    }

    fn edge_target(&self, edge: &Self::Edge) -> Option<&Self::Vertex> {
        self.0.edge_target(edge)
    }

    fn edge_weight(&self, edge: &Self::Edge) -> Option<f64> {
        self.0.edge_weight(edge)
    }

    fn edges_of(&self, vertex: &Self::Vertex) -> Result<impl Iterator<Item = &Self::Edge>> {
        self.0.edges_of(vertex)
    }

    fn edges_between<'a>(
        &'a self,
        source: &'a Self::Vertex,
        target: &'a Self::Vertex,
    ) -> impl Iterator<Item = &'a Self::Edge> {
        self.0.edges_between(source, target)
    }

    fn in_edges_of(&self, vertex: &Self::Vertex) -> Result<impl Iterator<Item = &Self::Edge>> {
        self.0.in_edges_of(vertex)
    }

    fn out_edges_of(&self, vertex: &Self::Vertex) -> Result<impl Iterator<Item = &Self::Edge>> {
        self.0.out_edges_of(vertex)
    }
}

impl DirectedGraph for SimpleSparse {}

#[test]
fn eppstein_propagates_missing_incoming_index() {
    let graph = SimpleSparse(no_incoming_graph());
    assert_unsupported(EppsteinShortestPathIterator::new(&graph, &0, &3));
}
