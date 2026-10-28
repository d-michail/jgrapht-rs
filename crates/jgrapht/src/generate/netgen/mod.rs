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

//! Port of `org.jgrapht.generate.netgen`: a NETGEN-style generator of flow problems (minimum cost
//! flow, maximum flow and bipartite matching / assignment).
//!
//! Generated problem objects borrow the caller-owned graph and own their per-vertex / per-arc data.

mod builder;
mod config;
mod distributor;
mod generator;
mod problems;

pub use builder::NetworkGeneratorConfigBuilder;
pub use config::NetworkGeneratorConfig;
pub use distributor::Distributor;
pub use generator::{CAPACITY_COST_BOUND, MAX_ARC_NUM, MAX_NODE_NUM, MAX_SUPPLY, NetworkGenerator};
pub use problems::{BipartiteMatchingProblem, CAPACITY_INF, MaximumFlowProblem, NetworkInfo};
