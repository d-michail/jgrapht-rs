//! Port of the JGraphT alg tests. See ../STATUS.md.
#![allow(clippy::bool_assert_comparison, clippy::approx_constant, clippy::useless_vec, dead_code)]
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
mod alg_stoer_wagner_tests;
mod alg_spanning_tests;
mod alg_tour_tests;
mod alg_transitive_tests;
mod alg_util_tests;
mod alg_vertexcover_tests;
mod brown_data;
mod matching_data;
mod clustering_data;
mod flow_data;
mod mincost_data;
mod planar_data;
mod vertex_cover_data;
mod clique_cover;
mod isomorphism;
mod matching_color;
mod misc_algs;
mod paths_misc;
mod planarity;
mod scoring;
mod shortestpath;
mod spanning_flow;
mod structure;
mod structure2;
