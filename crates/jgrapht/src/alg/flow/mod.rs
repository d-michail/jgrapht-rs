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

//! Maximum flow, minimum cut and minimum-cost flow.
//!
//! The maximum-flow classes ([`EdmondsKarp`], [`Dinic`], [`PushRelabel`],
//! [`BoykovKolmogorov`]) take the network (edge weights are capacities) and answer
//! `calculate_maximum_flow(source, sink)`; a computation that also needs the flow on each edge or the
//! cut returns a [`MaximumFlow`] result object. [`GusfieldEquivalentFlowTree`] and
//! [`GusfieldGomoryHuCutTree`] answer all-pairs minimum cuts, and [`PadbergRaoOddMinimumCutset`] finds
//! minimum odd cuts. [`CapacityScalingMinimumCostFlow`] solves minimum-cost flow problems described by
//! a [`MinimumCostFlowProblem`].
//!
//! Constructors reject negative and NaN capacities, and a source equal to the sink is
//! [`GraphError::SourceEqualsSink`](crate::GraphError::SourceEqualsSink).

mod base;
mod boykov_kolmogorov;
mod dinic;
mod edmonds_karp;
mod gusfield;
pub mod mincost;
mod padberg_rao;
mod push_relabel;

pub use base::{
    MaximumFlow, MaximumFlowAlgorithm, MaximumFlowValueAlgorithm, MinimumCut, MinimumStCutAlgorithm,
};
pub use boykov_kolmogorov::BoykovKolmogorov;
pub use dinic::Dinic;
pub use edmonds_karp::EdmondsKarp;
pub use gusfield::{GusfieldEquivalentFlowTree, GusfieldGomoryHuCutTree};
pub use mincost::{
    CapacityScalingMinimumCostFlow, MinimumCostFlow, MinimumCostFlowAlgorithm,
    MinimumCostFlowNetwork, MinimumCostFlowProblem, MinimumCostSolution,
};
pub use padberg_rao::PadbergRaoOddMinimumCutset;
pub use push_relabel::PushRelabel;

/// `MaximumFlowAlgorithmBase.DEFAULT_EPSILON`.
pub const DEFAULT_EPSILON: f64 = 1e-9;
