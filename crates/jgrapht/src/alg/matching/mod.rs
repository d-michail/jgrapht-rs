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

//! Matchings.
//!
//! A [`Matching`] holds the matched edges, their total weight and which vertices are matched.
//! [`MatchingAlgorithm`] is the common trait.
//!
//! | Algorithm | Graphs | Optimal for |
//! |---|---|---|
//! | [`GreedyMaximumCardinalityMatching`], [`GreedyWeightedMatching`] | any undirected | no (heuristic) |
//! | [`PathGrowingWeightedMatching`] | any undirected | no (1/2-approximation of maximum weight) |
//! | [`HopcroftKarp`] | bipartite, given the two sides | maximum cardinality |
//! | [`MaximumWeightBipartiteMatching`], [`KuhnMunkres`] | bipartite | maximum weight / minimum-weight perfect |
//! | [`DenseEdmonds`], [`SparseEdmonds`] | any undirected | maximum cardinality |
//! | the Blossom V module | any undirected | minimum-weight perfect matching |

mod base;
pub mod blossom;
mod dense_edmonds;
mod greedy;
mod hopcroft_karp;
mod kuhn_munkres;
mod max_weight_bipartite;
mod path_growing;
mod sparse_edmonds;

pub use base::{DEFAULT_EPSILON, Matching, MatchingAlgorithm};
pub use dense_edmonds::DenseEdmonds;
pub use greedy::{GreedyMaximumCardinalityMatching, GreedyWeightedMatching};
pub use hopcroft_karp::HopcroftKarp;
pub use kuhn_munkres::KuhnMunkres;
pub use max_weight_bipartite::MaximumWeightBipartiteMatching;
pub use path_growing::{
    DEFAULT_USE_HEURISTICS, DynamicProgrammingPathSolver, PathGrowingWeightedMatching,
};
pub use sparse_edmonds::SparseEdmonds;
