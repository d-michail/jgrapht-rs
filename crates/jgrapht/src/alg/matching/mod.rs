//! Matching algorithms (JGraphT `org.jgrapht.alg.matching`).

mod base;
mod dense_edmonds;
mod greedy;
mod hopcroft_karp;
mod kuhn_munkres;
mod max_weight_bipartite;
mod path_growing;
mod sparse_edmonds;
pub(crate) mod legacy;

pub use base::{DEFAULT_EPSILON, Matching, MatchingAlgorithm};
pub use dense_edmonds::DenseEdmondsMaximumCardinalityMatching;
pub use greedy::{GreedyMaximumCardinalityMatching, GreedyWeightedMatching};
pub use hopcroft_karp::HopcroftKarpMaximumCardinalityBipartiteMatching;
pub use kuhn_munkres::KuhnMunkresMinimalWeightBipartitePerfectMatching;
pub use max_weight_bipartite::MaximumWeightBipartiteMatching;
pub use path_growing::{DEFAULT_USE_HEURISTICS, DynamicProgrammingPathSolver, PathGrowingWeightedMatching};
pub use sparse_edmonds::SparseEdmondsMaximumCardinalityMatching;
// old function-style helpers, removed class by class
pub use legacy::*;
