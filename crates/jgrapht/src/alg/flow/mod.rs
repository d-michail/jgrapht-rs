//! Maximum flow, minimum cuts, Gomory–Hu trees and minimum cost flow.

mod base;
mod boykov_kolmogorov;
mod dinic;
mod edmonds_karp;
mod gusfield;
pub mod mincost;
mod padberg_rao;
mod push_relabel;

pub use base::{MaximumFlow, MaximumFlowAlgorithm, MinimumSTCutAlgorithm};
pub use boykov_kolmogorov::BoykovKolmogorovMFImpl;
pub use dinic::DinicMFImpl;
pub use edmonds_karp::EdmondsKarpMFImpl;
pub use gusfield::{GusfieldEquivalentFlowTree, GusfieldGomoryHuCutTree};
pub use mincost::{
    CapacityScalingMinimumCostFlow, MinimumCostFlow, MinimumCostFlowAlgorithm,
    MinimumCostFlowProblem, MinimumCostFlowProblemImpl,
};
pub use padberg_rao::PadbergRaoOddMinimumCutset;
pub use push_relabel::PushRelabelMFImpl;

/// `MaximumFlowAlgorithmBase.DEFAULT_EPSILON`.
pub const DEFAULT_EPSILON: f64 = 1e-9;
