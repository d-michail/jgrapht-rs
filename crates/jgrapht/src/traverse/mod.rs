// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
// Rust port (C) Copyright 2026, by Dimitrios Michail and jgrapht-rs Contributors.
//
// See the CONTRIBUTORS.md file distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the GNU Lesser General Public
// License v2.1 or later which is available at
// http://www.gnu.org/licenses/old-licenses/lgpl-2.1-standalone.html.
//
// SPDX-License-Identifier: EPL-2.0 OR LGPL-2.1-or-later

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
