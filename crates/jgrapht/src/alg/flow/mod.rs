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

//! Maximum flow, minimum cuts, Gomory–Hu trees and minimum cost flow.

mod base;
mod boykov_kolmogorov;
mod dinic;
mod edmonds_karp;
mod gusfield;
pub mod mincost;
mod padberg_rao;
mod push_relabel;

pub use base::{
    MaximumFlow, MaximumFlowAlgorithm, MaximumFlowValueAlgorithm, MinimumCut, MinimumSTCutAlgorithm,
};
pub use boykov_kolmogorov::BoykovKolmogorovMFImpl;
pub use dinic::DinicMFImpl;
pub use edmonds_karp::EdmondsKarpMFImpl;
pub use gusfield::{GusfieldEquivalentFlowTree, GusfieldGomoryHuCutTree};
pub use mincost::{
    CapacityScalingMinimumCostFlow, MinimumCostFlow, MinimumCostFlowAlgorithm,
    MinimumCostFlowProblem, MinimumCostFlowProblemImpl, MinimumCostSolution,
};
pub use padberg_rao::PadbergRaoOddMinimumCutset;
pub use push_relabel::PushRelabelMFImpl;

/// `MaximumFlowAlgorithmBase.DEFAULT_EPSILON`.
pub const DEFAULT_EPSILON: f64 = 1e-9;
