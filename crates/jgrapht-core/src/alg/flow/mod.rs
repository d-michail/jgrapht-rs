//! Maximum flow, minimum cuts, Gomory–Hu trees, minimum cost flow, Stoer–Wagner.
//!
//! The function-style helpers live in [`legacy`] and are re-exported; the JGraphT classes
//! (`EdmondsKarpMFImpl`, `DinicMFImpl`, `PushRelabelMFImpl`, `BoykovKolmogorovMFImpl`, ...) are in
//! the sibling modules and share the residual network of [`base`].

mod base;
mod boykov_kolmogorov;
mod dinic;
mod edmonds_karp;
mod gusfield;
mod legacy;
pub mod mincost;
mod padberg_rao;
mod push_relabel;

pub use base::{MaximumFlow, MaximumFlowAlgorithm, MinimumSTCutAlgorithm};
pub use boykov_kolmogorov::BoykovKolmogorovMFImpl;
pub use dinic::DinicMFImpl;
pub use edmonds_karp::EdmondsKarpMFImpl;
pub use gusfield::{GusfieldEquivalentFlowTree, GusfieldGomoryHuCutTree};
pub use legacy::*;
pub use padberg_rao::PadbergRaoOddMinimumCutset;
pub use push_relabel::PushRelabelMFImpl;

/// `MaximumFlowAlgorithmBase.DEFAULT_EPSILON`.
pub const DEFAULT_EPSILON: f64 = 1e-9;
