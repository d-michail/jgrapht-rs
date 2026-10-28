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

//! Travelling salesman heuristics, exact solvers and Hamiltonian path search.
//!
//! The heuristics ([`NearestInsertionHeuristicTsp`], [`FarthestInsertionHeuristicTsp`],
//! [`ChristofidesThreeHalvesApproxMetricTsp`], [`TwoOptHeuristicTsp`], and the greedy, nearest-neighbour
//! and random tours in `simple`) return a tour as a [`GraphWalk`](crate::GraphWalk). The exact Held-Karp
//! solver is limited to small graphs and says so with
//! [`GraphError::TooManyVertices`](crate::GraphError::TooManyVertices). Hamiltonian cycle and path
//! searches (Palmer, backtracking, DAG, Held-Karp) are in `path`.

mod base;
mod christofides;
mod insertion;
mod path;
mod simple;
mod two_opt;

pub use base::{HamiltonianCycleAlgorithm, HamiltonianCycleImprovementAlgorithm};
pub use christofides::ChristofidesThreeHalvesApproxMetricTsp;
pub use insertion::{FarthestInsertionHeuristicTsp, NearestInsertionHeuristicTsp};
pub use path::{
    BacktrackingHamiltonianPath, DagHamiltonianPath, HamiltonianPathAlgorithm,
    HamiltonianPathSearchResult, HeldKarpHamiltonianPath, SearchStatus,
};
pub use simple::{
    GreedyHeuristicTsp, HeldKarpTsp, NearestNeighborHeuristicTsp, PalmerHamiltonianCycle,
    RandomTourTsp, TwoApproxMetricTsp,
};
pub use two_opt::TwoOptHeuristicTsp;
