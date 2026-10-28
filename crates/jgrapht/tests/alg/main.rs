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

#![allow(clippy::needless_range_loop, clippy::multiple_bound_locations)]
//! Port of the JGraphT alg tests. See ../STATUS.md.
#![allow(
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::useless_vec,
    dead_code
)]
#[path = "../common/mod.rs"]
mod common;

mod alg_clique_tests;
mod alg_clustering_tests;
mod alg_color_tests;
mod alg_connectivity_tests;
mod alg_cycle_tests;
mod alg_decomposition_tests;
mod alg_drawing_tests;
mod alg_flow_tests;
mod alg_isomorphism_tests;
mod alg_lca_tests;
mod alg_linkprediction_tests;
mod alg_matching_tests;
mod alg_planar_tests;
mod alg_scoring_tests;
mod alg_small_tests;
mod alg_spanning_tests;
mod alg_stoer_wagner_tests;
mod alg_tour_tests;
mod alg_transitive_tests;
mod alg_util_tests;
mod alg_vertexcover_tests;
mod blossom_data;
mod brown_data;
mod capability_bounds_test;
mod clique_cover;
mod clustering_data;
mod flow_data;
mod flow_results_test;
mod isomorphism;
mod matching_color;
mod matching_data;
mod mincost_data;
mod misc_algs;
mod paths_misc;
mod planar_data;
mod planarity;
mod scoring;
mod shortestpath;
mod spanning_flow;
mod structure;
mod structure2;
mod vertex_cover_data;
