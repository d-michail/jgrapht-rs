//! Graph traversals as lazy iterators.
//!
//! The JGraphT-faithful iterators — [`BreadthFirstIterator`], [`DepthFirstIterator`],
//! [`ClosestFirstIterator`], [`TopologicalOrderIterator`], [`LexBreadthFirstIterator`],
//! [`MaximumCardinalityIterator`], [`DegeneracyOrderingIterator`] and
//! [`RandomWalkVertexIterator`] — report traversal events to [`TraversalListener`]s and visit
//! vertices in exactly JGraphT's order. The lightweight iterators (`Bfs`, `Dfs`, ...) are used
//! internally by algorithms.

mod cross;
mod events;
mod ordered;
mod random_walk;
mod simple;
mod simple_ordering;
mod simple_weighted;

pub use cross::{
    BfsData, BreadthFirstIterator, ClosestFirstIterator, Core, CrossComponentIterator,
    DepthFirstIterator, DfsStrategy, Strategy,
};
pub use events::{TraversalListener, TraversalListeners};
pub use ordered::{
    DegeneracyOrderingIterator, LexBreadthFirstIterator, MaximumCardinalityIterator,
    TopologicalOrderIterator,
};
pub use random_walk::RandomWalkVertexIterator;
pub use simple::*;
pub use simple_ordering::{DegeneracyOrdering, LexBfs, MaximumCardinality};
pub use simple_weighted::{ClosestFirst, RandomWalk};
