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
#![deny(missing_docs)]
//! A Rust port of the [JGraphT](https://jgrapht.org) core library.
//!
//! Unlike the Java original, graphs are expressed as *traits*
//! ([`Graph`], [`DirectedGraph`], [`UndirectedGraph`], [`GraphMut`]) so that
//! algorithms are written against capabilities and views (subgraphs, reversed
//! graphs, ...) are plain wrapper types. Vertices and edges are user supplied
//! values (`Clone + Eq + Hash`).
//!
//! New to the library? Start with the [`guide`] (concepts, graph types, views, traversals,
//! algorithms, generators, performance and a table for readers who know JGraphT).
//!
//! # Error handling at a glance
//!
//! Unreachable is `None`, a failure is `Err`, and algorithms say which graph capabilities they
//! need through trait bounds instead of failing at run time:
//!
//! ```
//! use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
//! use jgrapht::*;
//!
//! let mut g = DiGraph::<u32>::simple().weighted();
//! g.add_vertices(0..3);
//! let e = g.add_edge(&0, &1)?.unwrap();
//! g.set_edge_weight(&e, 2.5)?;
//!
//! // `Result<Option<_>>`: `Ok(None)` means "no path", `Err` means the call itself failed.
//! assert_eq!(Dijkstra::new(&g).path_weight(&0, &1)?, Some(2.5));
//! assert_eq!(Dijkstra::new(&g).path_weight(&0, &2)?, None);
//! # Ok::<(), GraphError>(())
//! ```
//!
//! Iterators that can fail yield `Result` items, so an error is never mistaken for the end:
//!
//! ```
//! use jgrapht::traverse::TopologicalOrderIterator;
//! use jgrapht::*;
//!
//! let mut g = DiGraph::<u32>::simple();
//! g.add_vertices(0..2);
//! g.add_edge(&0, &1)?;
//! g.add_edge(&1, &0)?;
//! let order: Result<Vec<_>> = TopologicalOrderIterator::new(&g)?.collect();
//! assert_eq!(order.unwrap_err(), GraphError::InvalidGraph(GraphProperty::Acyclic));
//! # Ok::<(), GraphError>(())
//! ```
//!
//! Capabilities are bounds. A function that walks incoming edges asks for [`IncomingGraph`], so a
//! graph without a reverse index is rejected at compile time:
//!
//! ```
//! use jgrapht::graph::sparse::{FullIncoming, SparseIntDirectedGraph};
//! use jgrapht::*;
//! use std::borrow::Borrow;
//!
//! fn in_degree_sum<G: IncomingGraph>(g: &G) -> usize {
//!     g.vertices().map(|v| g.in_degree_of(v.borrow())).sum()
//! }
//!
//! let g = SparseIntDirectedGraph::<FullIncoming>::from_edges(3, &[(0, 1), (1, 2)])?;
//! assert_eq!(in_degree_sum(&g), 2);
//! # Ok::<(), GraphError>(())
//! ```
//!
//! ```compile_fail
//! use jgrapht::graph::sparse::{NoIncoming, SparseIntDirectedGraph};
//! use jgrapht::*;
//! # fn in_degree_sum<G: IncomingGraph>(_: &G) {}
//! let g = SparseIntDirectedGraph::<NoIncoming>::from_edges(3, &[(0, 1)]).unwrap();
//! in_degree_sum(&g); // `NoIncoming` has no incoming-edge index
//! ```

#[cfg(feature = "serde")]
pub(crate) mod serde_support;

pub mod alg;
pub mod error;
pub mod event;
pub mod generate;
pub mod graph;
/// Capability descriptor of a graph (`GraphType`).
pub mod graph_type;
pub mod guide;
pub mod traverse;
pub mod util;
pub mod view;
pub mod walk;

pub use error::{
    FlowProblem, GraphError, GraphProperty, ParameterRequirement, Result, WeightProblem,
};
pub use graph::{
    BaseGraph, DefaultEdge, DiGraph, DirectedExt, DirectedGraph, FastutilMapGraph,
    FastutilMapIntVertexGraph, Graph, GraphExt, GraphMut, GraphMutExt, IncidenceGraph,
    IncomingGraph, OutgoingGraph, UGraph, UndirectedGraph, graph_equals, graph_hash,
    graph_to_string,
};
pub use graph_type::GraphType;
pub use view::{
    AsSubgraph, AsUndirected, AsUnweighted, AsWeighted, EdgeReversed, GraphUnion, MaskSubgraph,
    Unmodifiable, WeightCombiner,
};
pub use walk::{GraphMapping, GraphWalk};
