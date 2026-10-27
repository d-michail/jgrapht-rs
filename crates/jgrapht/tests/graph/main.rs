//! Port of the JGraphT graph tests. See ../STATUS.md.
#![allow(clippy::bool_assert_comparison, clippy::approx_constant, clippy::useless_vec, dead_code)]
#[path = "../common/mod.rs"]
mod common;

mod directed_acyclic_graph_test;
mod graph_basic;
mod graph_builder;
mod graph_metrics_test;
mod graph_tests_test;
mod graph_views;
mod graph_walk_test;
mod graphs_test;
mod listenable_graph_test;
mod simple_directed_graph_test;
mod graph_basic_extra;
mod views;
