//! Travelling salesman heuristics, exact solvers and Hamiltonian path search.
//!
//! The function-style helpers (`nearest_neighbor_tour`, `held_karp_tour`, ...) are kept in
//! [`legacy`] and re-exported; the JGraphT classes live in the sibling modules and share the
//! helpers of [`base`].

mod base;
mod insertion;
mod legacy;
mod path;
mod simple;
mod two_opt;

pub use base::{HamiltonianCycleAlgorithm, HamiltonianCycleImprovementAlgorithm};
pub use insertion::{FarthestInsertionHeuristicTSP, NearestInsertionHeuristicTSP};
pub use legacy::*;
pub use path::{
    BacktrackingHamiltonianPath, DagHamiltonianPath, HamiltonianPathAlgorithm,
    HamiltonianPathSearchResult, HeldKarpHamiltonianPath, SearchStatus,
};
pub use simple::{
    GreedyHeuristicTSP, HeldKarpTSP, NearestNeighborHeuristicTSP, PalmerHamiltonianCycle,
    RandomTourTSP, TwoApproxMetricTSP,
};
pub use two_opt::TwoOptHeuristicTSP;
