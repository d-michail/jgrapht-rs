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

//! Port of the JGraphT graph tests. See ../STATUS.md.
#![allow(
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::useless_vec,
    dead_code
)]
#[path = "../common/mod.rs"]
mod common;

mod directed_acyclic_graph_test;
mod fixtures_test;
mod graph_basic;
mod graph_basic_extra;
mod graph_builder;
mod graph_metrics_test;
mod graph_tests_test;
mod graph_views;
mod graph_walk_test;
mod graphs_test;
mod listenable_graph_test;
mod map_graph_test;
mod simple_directed_graph_test;
mod sparse_int_graph_test;
mod views;
