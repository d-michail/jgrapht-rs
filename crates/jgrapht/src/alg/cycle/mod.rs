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

//! Cycles: detection, enumeration, bases, Eulerian cycles and chordality.
//!
//! * [`CycleDetector`] finds cycles in directed graphs; the simple-cycle enumerators (Tiernan, Tarjan,
//!   Johnson, Szwarcfiter-Lauer, Hawick-James) list every elementary cycle.
//! * Cycle bases: fundamental, Paton and others, as a [`CycleBasis`].
//! * [`HierholzerEulerianCycle`] tests for and finds Eulerian cycles; [`ChinesePostman`] solves the
//!   route-inspection problem.
//! * [`ChordalityInspector`], [`WeakChordalityInspector`] and [`BergeGraphInspector`] test chordality,
//!   weak chordality and the Berge property and return certificates (a hole, an antihole).
//! * [`HowardMinimumMeanCycle`] finds minimum mean cycles.

mod basis;
mod berge;
mod chinese_postman;
mod chordality;
mod detector;
mod eulerian;
mod howard;
mod separator;
mod simple_cycles;
mod weak_chordality;

pub use basis::{
    CycleBasis, CycleBasisAlgorithm, PatonCycleBase, QueueBFSFundamentalCycleBasis,
    StackBFSFundamentalCycleBasis,
};
pub use berge::{BergeCertificate, BergeGraphInspector};
pub use chinese_postman::ChinesePostman;
pub use chordality::{ChordalityInspector, IterationOrder};
pub use detector::{CycleDetector, simple_cycle_to_graph_path};
pub use eulerian::{EulerianCycleAlgorithm, HierholzerEulerianCycle};
pub use howard::{HowardMinimumMeanCycle, MinimumCycleMeanAlgorithm};
pub use separator::ChordalGraphMinimalVertexSeparatorFinder;
pub use simple_cycles::{
    DirectedSimpleCycles, HawickJamesSimpleCycles, JohnsonSimpleCycles,
    SzwarcfiterLauerSimpleCycles, TarjanSimpleCycles, TiernanSimpleCycles,
};
pub use weak_chordality::{WeakChordalityCertificate, WeakChordalityInspector};
