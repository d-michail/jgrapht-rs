use super::options::BlossomVOptions;
use super::types::BlossomArena;
use crate::graph::Graph;

/// Counters and timings collected by a Blossom V run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BlossomVStatistics {
    pub(crate) shrink_num: usize,
    pub(crate) expand_num: usize,
    pub(crate) grow_num: usize,
    pub(crate) augment_time_ns: u128,
    pub(crate) expand_time_ns: u128,
    pub(crate) shrink_time_ns: u128,
    pub(crate) grow_time_ns: u128,
    pub(crate) dual_updates_time_ns: u128,
}

impl BlossomVStatistics {
    /// Number of shrink operations.
    pub const fn shrink_count(self) -> usize {
        self.shrink_num
    }

    /// Number of expand operations.
    pub const fn expand_count(self) -> usize {
        self.expand_num
    }

    /// Number of grow operations.
    pub const fn grow_count(self) -> usize {
        self.grow_num
    }

    /// Nanoseconds spent augmenting.
    pub const fn augment_time_ns(self) -> u128 {
        self.augment_time_ns
    }

    /// Nanoseconds spent expanding blossoms.
    pub const fn expand_time_ns(self) -> u128 {
        self.expand_time_ns
    }

    /// Nanoseconds spent shrinking blossoms.
    pub const fn shrink_time_ns(self) -> u128 {
        self.shrink_time_ns
    }

    /// Nanoseconds spent growing alternating trees.
    pub const fn grow_time_ns(self) -> u128 {
        self.grow_time_ns
    }

    /// Nanoseconds spent updating dual variables.
    pub const fn dual_updates_time_ns(self) -> u128 {
        self.dual_updates_time_ns
    }
}

pub(crate) struct BlossomVState<'g, G: Graph> {
    pub(crate) graph: &'g G,
    pub(crate) arena: BlossomArena,
    pub(crate) node_num: usize,
    pub(crate) edge_num: usize,
    pub(crate) tree_num: usize,
    pub(crate) removed_num: usize,
    pub(crate) blossom_num: usize,
    pub(crate) statistics: BlossomVStatistics,
    pub(crate) options: BlossomVOptions,
    pub(crate) graph_vertices: Vec<G::Vertex>,
    pub(crate) graph_edges: Vec<G::Edge>,
    pub(crate) min_edge_weight: f64,
    pub(crate) root_sentinel: usize,
}
