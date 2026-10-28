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

//! Graph algorithms, one module per area.
//!
//! Algorithms are written against the graph capability traits, so each one asks only for what it uses
//! (`Graph`, `OutgoingGraph`, `IncomingGraph`, `IncidenceGraph`, `DirectedGraph`, `UndirectedGraph`).
//! Functions return their result as a plain value; classes that keep configuration (a start heuristic,
//! a tolerance) are built with `new` or `with_*` constructors that validate their arguments and return
//! `Result`. Unreachable or absent answers are `Option`, and failures are typed [`GraphError`](crate::GraphError)s.
//!
//! | Area | Module |
//! |---|---|
//! | Shortest paths | [`shortestpath`] |
//! | Connectivity, components, block-cut trees | [`connectivity`] |
//! | Spanning trees and arborescences | [`spanning`], [`capacitated`], [`steiner`], [`spanner`] |
//! | Maximum flow, minimum cut, minimum-cost flow | [`flow`], [`stoer_wagner`] |
//! | Matching | [`matching`] |
//! | Vertex colouring | [`color`] |
//! | Cliques, independent sets, vertex covers | [`clique`], [`independent_set`], [`cover`] |
//! | Cycles, Eulerian cycles, chordality, holes | [`cycle`], [`holes`] |
//! | Tours and Hamiltonian paths | [`tour`] |
//! | Centrality and clustering coefficients | [`scoring`] |
//! | Planarity, isomorphism, lowest common ancestors | [`planar`], [`isomorphism`], [`lca`] |
//! | Tree decompositions and partitions | [`decomposition`], [`partition`] |
//! | Clustering, link prediction, similarity | [`clustering`], [`linkprediction`], [`similarity`] |
//! | Layouts | [`drawing`] |
//! | Transformations (complement, line graph, contraction) | [`transform`], [`transitive`] |
//! | Facility location and dense subgraphs | [`centers`], [`densest`] |
//!
//! Many algorithms come in two forms: a function for the common call (`greedy_coloring`,
//! `kruskal`, `maximal_cliques`) and a struct for configuration, repeated queries or use through a
//! trait (`GreedyColoring`, `BronKerboschCliqueFinder`). The function is a shortcut for the struct
//! with default settings; both give the same result.
//!
The [user guide](crate::guide#algorithms) shows examples.

pub mod capacitated;
pub mod centers;
pub mod clique;
pub mod clustering;
pub mod color;
pub mod connectivity;
pub mod cover;
pub mod cycle;
pub mod cyclic_exchange;
pub mod decomposition;
pub mod densest;
pub mod drawing;
pub mod flow;
pub mod holes;
pub mod independent_set;
pub mod interfaces;
pub mod isomorphism;
pub mod lca;
pub mod linkprediction;
pub mod matching;
pub mod metrics;
pub mod partition;
pub mod planar;
pub mod scoring;
pub mod shortestpath;
pub mod similarity;
pub mod spanner;
pub mod spanning;
pub mod steiner;
pub mod stoer_wagner;
pub mod tour;
pub mod transform;
pub mod transitive;
pub mod tree_connectivity;
pub mod util;
