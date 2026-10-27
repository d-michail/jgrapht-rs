//! Traits and result types shared by several algorithms (JGraphT's `alg.interfaces`).
//! More are added as the algorithm families that need them are ported.

use crate::Result;
use crate::graph::Graph;
use crate::walk::GraphWalk;
use indexmap::{IndexMap, IndexSet};
use std::collections::HashSet;
use std::hash::Hash;

/// A partition of the vertices into disjoint sets (`PartitioningAlgorithm.Partitioning`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partitioning<V: Eq + Hash> {
    partitions: Vec<IndexSet<V>>,
}

impl<V: Eq + Hash> Partitioning<V> {
    pub fn new(partitions: Vec<IndexSet<V>>) -> Self {
        Partitioning { partitions }
    }
    pub fn number_partitions(&self) -> usize {
        self.partitions.len()
    }
    pub fn partition(&self, index: usize) -> &IndexSet<V> {
        &self.partitions[index]
    }
    pub fn partitions(&self) -> &[IndexSet<V>] {
        &self.partitions
    }
    pub fn iter(&self) -> impl Iterator<Item = &IndexSet<V>> {
        self.partitions.iter()
    }
}

/// An algorithm computing a partitioning of the vertices.
pub trait PartitioningAlgorithm<G: Graph>
where
    G::Vertex: Eq + Hash,
{
    /// The partitioning, or `None` if the algorithm finds none.
    fn partitioning(&mut self) -> Option<&Partitioning<G::Vertex>>;
    fn is_valid_partitioning(&self, partitioning: &Partitioning<G::Vertex>) -> bool;
}

/// Algorithms choosing `k` center vertices of a complete graph (`CentersLocationAlgorithm`).
pub trait CentersLocationAlgorithm<V: Eq + Hash + Clone> {
    fn centers<G: Graph<Vertex = V>>(&mut self, graph: &G, k: usize) -> Result<IndexSet<V>>;
    /// Client to its closest center; an error before [`centers`](Self::centers) ran.
    fn assignment(&self) -> Result<&IndexMap<V, V>>;
    fn covering_radius(&self) -> Result<f64>;
}

/// A decomposition of a tree into vertex-disjoint paths (`TreeToPathDecompositionAlgorithm.PathDecomposition`).
#[derive(Debug, Clone)]
pub struct PathDecomposition<V, E> {
    edges: HashSet<E>,
    paths: Vec<GraphWalk<V, E>>,
}

impl<V, E> PathDecomposition<V, E> {
    pub fn new(edges: HashSet<E>, paths: Vec<GraphWalk<V, E>>) -> Self {
        PathDecomposition { edges, paths }
    }
    /// The edges lying on the paths.
    pub fn edges(&self) -> &HashSet<E> {
        &self.edges
    }
    pub fn paths(&self) -> &[GraphWalk<V, E>] {
        &self.paths
    }
    pub fn number_of_paths(&self) -> usize {
        self.paths.len()
    }
}
