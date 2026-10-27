//! Port of the JGraphT (Java) JUnit test suite. See STATUS.md for the file-by-file state.
#![allow(
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::useless_vec
)]

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
mod alg_planar_tests;
mod alg_scoring_tests;
mod alg_small_tests;
mod alg_spanning_tests;
mod alg_tour_tests;
mod alg_util_tests;
mod alg_vertexcover_tests;
mod common;

mod brown_data;
mod clustering_data;
mod directed_acyclic_graph_test;
mod flow_data;
mod generate_tests;
mod graph_basic;
mod graph_builder;
mod graph_metrics_test;
mod graph_tests_test;
mod graph_views;
mod graph_walk_test;
mod graphs_test;
mod listenable_graph_test;
mod mincost_data;
mod netgen_tests;
mod planar_data;
mod simple_directed_graph_test;
mod traverse_tests;
mod util_tests;
mod vertex_cover_data;
