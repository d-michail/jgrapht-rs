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

//! Kolmogorov's Blossom V weighted matching algorithm.

mod dual;
mod initializer;
mod matching;
mod options;
mod perfect;
mod primal;
mod state;
#[allow(dead_code)]
mod types;

pub use matching::KolmogorovWeightedMatching;
pub use options::{BlossomVOptions, DualUpdateStrategy, InitializationType, ObjectiveSense};
pub use perfect::{
    BLOSSOM_V_EPSILON, BLOSSOM_V_INFINITY, BlossomVDualSolution, KolmogorovWeightedPerfectMatching,
};
pub use state::BlossomVStatistics;
