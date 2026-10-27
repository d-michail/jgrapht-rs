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
pub struct FastutilMapSpecifics;

/// Storage marker for the integer-vertex map graph implementation.
///
/// Rust stores `usize` keys inline, so this has the same no-boxing property as fastutil's
/// `Int2ReferenceLinkedOpenHashMap`.
#[derive(Debug, Clone, Copy, Default)]
pub struct FastutilMapIntVertexSpecifics;

/// Map-backed graph with arbitrary hashable vertices.
///
/// The final type parameter selects direction and defaults to [`Undirected`]. Constructors accept
/// a `fast_lookups` flag; when enabled, endpoint pairs are indexed for expected O(1) edge lookup.
pub type FastutilMapGraph<V, E = DefaultEdge, D = Undirected> =
    BaseGraph<V, E, D, FastutilMapSpecifics>;

/// Map-backed graph specialized for integer vertices.
pub type FastutilMapIntVertexGraph<E = DefaultEdge, D = Undirected> =
    BaseGraph<usize, E, D, FastutilMapIntVertexSpecifics>;

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction>
    BaseGraph<V, E, D, FastutilMapSpecifics>
{
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

impl<E: Clone + Eq + Hash, D: Direction> BaseGraph<usize, E, D, FastutilMapIntVertexSpecifics> {
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

impl<V: Clone + Eq + Hash, D: Direction> BaseGraph<V, DefaultEdge, D, FastutilMapSpecifics> {
    /// Creates a map graph with [`DefaultEdge`]s and optional endpoint indexing.
    pub fn with_graph_type_and_lookup(ty: GraphType, fast_lookups: bool) -> Self {
        Self::with_edge_factory_and_lookup(ty, DefaultEdge::from_id, fast_lookups)
    }
}

impl<D: Direction> BaseGraph<usize, DefaultEdge, D, FastutilMapIntVertexSpecifics> {
    /// Creates an integer-vertex map graph with [`DefaultEdge`]s and optional endpoint indexing.
    pub fn with_graph_type_and_lookup(ty: GraphType, fast_lookups: bool) -> Self {
        Self::with_edge_factory_and_lookup(ty, DefaultEdge::from_id, fast_lookups)
    }
}
