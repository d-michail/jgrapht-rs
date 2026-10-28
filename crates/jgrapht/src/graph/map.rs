// (C) Copyright 2003-2026, by John V Sichi and JGraphT Contributors.
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

//! Mutable map-backed graph storage strategies.

use super::{BaseGraph, DefaultEdge, Direction, Undirected};
use crate::GraphType;
use std::hash::Hash;

/// Storage marker for the object-keyed map graph implementation.
///
/// [`indexmap::IndexMap`] is a Rust-native open-addressed hash table with linked insertion order,
/// matching the relevant storage and iteration properties of fastutil's
/// `Object2ObjectLinkedOpenHashMap`.
#[derive(Debug, Clone, Copy, Default)]
pub struct MapSpecifics;

/// Storage marker for the integer-vertex map graph implementation.
///
/// Rust stores `usize` keys inline, so this has the same no-boxing property as fastutil's
/// `Int2ReferenceLinkedOpenHashMap`.
#[derive(Debug, Clone, Copy, Default)]
pub struct IntMapSpecifics;

/// Map-backed graph with arbitrary hashable vertices.
///
/// The final type parameter selects direction and defaults to [`Undirected`]. Constructors accept
/// a `fast_lookups` flag; when enabled, endpoint pairs are indexed for expected O(1) edge lookup.
pub type MapGraph<V, E = DefaultEdge, D = Undirected> = BaseGraph<V, E, D, MapSpecifics>;

/// Map-backed graph specialized for integer vertices.
pub type IntMapGraph<E = DefaultEdge, D = Undirected> = BaseGraph<usize, E, D, IntMapSpecifics>;

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction> BaseGraph<V, E, D, MapSpecifics> {
    /// Creates a map graph with an edge factory and optional endpoint indexing.
    pub fn with_edge_factory_and_lookup(
        ty: GraphType,
        factory: fn(u64) -> E,
        fast_lookups: bool,
    ) -> Self {
        Self::build(ty, Some(factory), fast_lookups)
    }

    /// Creates a map graph without an edge factory.
    pub fn without_edge_factory_and_lookup(ty: GraphType, fast_lookups: bool) -> Self {
        Self::build(ty, None, fast_lookups)
    }
}

impl<E: Clone + Eq + Hash, D: Direction> BaseGraph<usize, E, D, IntMapSpecifics> {
    /// Creates an integer-vertex map graph with an edge factory and optional endpoint indexing.
    pub fn with_edge_factory_and_lookup(
        ty: GraphType,
        factory: fn(u64) -> E,
        fast_lookups: bool,
    ) -> Self {
        Self::build(ty, Some(factory), fast_lookups)
    }

    /// Creates an integer-vertex map graph without an edge factory.
    pub fn without_edge_factory_and_lookup(ty: GraphType, fast_lookups: bool) -> Self {
        Self::build(ty, None, fast_lookups)
    }
}

impl<V: Clone + Eq + Hash, D: Direction> BaseGraph<V, DefaultEdge, D, MapSpecifics> {
    /// Creates a map graph with [`DefaultEdge`]s and optional endpoint indexing.
    pub fn with_graph_type_and_lookup(ty: GraphType, fast_lookups: bool) -> Self {
        Self::with_edge_factory_and_lookup(ty, DefaultEdge::from_id, fast_lookups)
    }
}

impl<D: Direction> BaseGraph<usize, DefaultEdge, D, IntMapSpecifics> {
    /// Creates an integer-vertex map graph with [`DefaultEdge`]s and optional endpoint indexing.
    pub fn with_graph_type_and_lookup(ty: GraphType, fast_lookups: bool) -> Self {
        Self::with_edge_factory_and_lookup(ty, DefaultEdge::from_id, fast_lookups)
    }
}
