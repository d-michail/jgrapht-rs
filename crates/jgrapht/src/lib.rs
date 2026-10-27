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

#![allow(
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::type_complexity,
    clippy::manual_is_multiple_of,
    clippy::too_many_arguments,
    clippy::nonminimal_bool
)]
//! A Rust port of the [JGraphT](https://jgrapht.org) core library.
//!
//! Unlike the Java original, graphs are expressed as *traits*
//! ([`Graph`], [`DirectedGraph`], [`UndirectedGraph`], [`GraphMut`]) so that
//! algorithms are written against capabilities and views (subgraphs, reversed
//! graphs, ...) are plain wrapper types. Vertices and edges are user supplied
//! values (`Clone + Eq + Hash`).

pub mod alg;
pub mod error;
pub mod event;
pub mod generate;
pub mod graph;
pub mod graph_type;
pub mod traverse;
pub mod util;
pub mod view;
pub mod walk;

pub use error::{GraphError, Result};
pub use graph::{
    BaseGraph, DefaultEdge, DiGraph, DirectedGraph, FastutilMapGraph, FastutilMapIntVertexGraph,
    Graph, GraphExt, GraphMut, GraphMutExt, UGraph, UndirectedGraph, graph_equals, graph_hash,
    graph_to_string,
};
pub use graph_type::GraphType;
pub use view::{
    AsSubgraph, AsUndirected, AsUnweighted, AsWeighted, EdgeReversed, GraphUnion, MaskSubgraph,
    Unmodifiable, WeightCombiner,
};
pub use walk::{GraphMapping, GraphWalk};
