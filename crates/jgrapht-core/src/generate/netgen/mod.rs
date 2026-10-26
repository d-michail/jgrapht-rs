//! Port of `org.jgrapht.generate.netgen`: a NETGEN-style generator of flow problems (minimum cost
//! flow, maximum flow and bipartite matching / assignment).
//!
//! The Java problem objects hold the graph; here the caller keeps the graph and the problem only
//! carries the per-vertex / per-arc data.

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
