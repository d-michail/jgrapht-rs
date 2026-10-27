//! Traits and result types shared by several algorithms (JGraphT's `alg.interfaces`).
//! More are added as the algorithm families that need them are ported.

use super::color::Coloring;
use super::cover::VertexCover;
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

/// Lowest common ancestor queries (`LowestCommonAncestorAlgorithm`).
pub trait LowestCommonAncestorAlgorithm<V: Eq + Hash + Clone> {
    /// The lowest common ancestor of `a` and `b`; `Ok(None)` when they have none (for example
    /// they lie in different trees). An error if a vertex is not in the graph.
    fn lca(&self, a: &V, b: &V) -> Result<Option<V>>;

    fn batch_lca(&self, queries: &[(V, V)]) -> Result<Vec<Option<V>>> {
        queries.iter().map(|(a, b)| self.lca(a, b)).collect()
    }

    /// All lowest common ancestors (only meaningful for DAGs; trees have at most one).
    fn lca_set(&self, _a: &V, _b: &V) -> Result<HashSet<V>> {
        Err(crate::GraphError::Unsupported(
            "lca_set is not supported by this algorithm".into(),
        ))
    }
}

/// Scores for pairs of vertices that might become linked (`LinkPredictionAlgorithm`).
pub trait LinkPredictionAlgorithm<V: Clone> {
    /// The score of the pair; [`GraphError::IndexNotWellDefined`](crate::GraphError) when the index is
    /// undefined for it.
    fn predict(&self, u: &V, v: &V) -> Result<f64>;

    fn predict_all(&self, queries: &[(V, V)]) -> Result<Vec<(V, V, f64)>> {
        queries
            .iter()
            .map(|(u, v)| Ok((u.clone(), v.clone(), self.predict(u, v)?)))
            .collect()
    }
}

/// Enumerates the maximal cliques of a graph (`MaximalCliqueEnumerationAlgorithm`).
pub trait MaximalCliqueEnumerationAlgorithm<V: Eq + Hash + Clone> {
    /// All maximal cliques (computed on first use); an error for graphs that are not simple.
    fn cliques(&mut self) -> Result<&[HashSet<V>]>;

    /// The cliques of maximum size.
    fn maximum_cliques(&mut self) -> Result<Vec<&HashSet<V>>> {
        let all = self.cliques()?;
        let max = all.iter().map(HashSet::len).max().unwrap_or(0);
        Ok(all.iter().filter(|c| c.len() == max).collect())
    }
}

/// Computes a vertex cover (`VertexCoverAlgorithm`).
pub trait VertexCoverAlgorithm<V: Clone> {
    fn vertex_cover(&self) -> Result<VertexCover<V>>;
}

/// Computes a proper vertex coloring (`VertexColoringAlgorithm`).
pub trait VertexColoringAlgorithm<V: Clone + Eq + Hash> {
    fn coloring(&self) -> Result<Coloring<V>>;
}

/// A partition of the vertices into clusters (`ClusteringAlgorithm.Clustering`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clustering<V: Eq + Hash> {
    clusters: Vec<IndexSet<V>>,
}

impl<V: Eq + Hash> Clustering<V> {
    pub fn new(clusters: Vec<IndexSet<V>>) -> Self {
        Clustering { clusters }
    }
    pub fn number_clusters(&self) -> usize {
        self.clusters.len()
    }
    pub fn clusters(&self) -> &[IndexSet<V>] {
        &self.clusters
    }
    pub fn iter(&self) -> impl Iterator<Item = &IndexSet<V>> {
        self.clusters.iter()
    }
}

/// An algorithm that clusters the vertices of a graph (`ClusteringAlgorithm`).
pub trait ClusteringAlgorithm<V: Eq + Hash + Clone> {
    fn clustering(&self) -> Result<Clustering<V>>;
}

/// Algorithms computing the strongly connected components of a directed graph
/// (`StrongConnectivityAlgorithm`). Results are computed when the algorithm is constructed.
pub trait StrongConnectivityAlgorithm {
    type G: crate::graph::DirectedGraph;

    fn graph(&self) -> &Self::G;
    /// The vertex sets of the strongly connected components.
    fn strongly_connected_sets(&self) -> &[HashSet<<Self::G as Graph>::Vertex>];

    fn is_strongly_connected(&self) -> bool {
        self.strongly_connected_sets().len() == 1
    }

    /// The components as induced subgraphs, in the order of
    /// [`strongly_connected_sets`](Self::strongly_connected_sets).
    fn strongly_connected_components(&self) -> Vec<crate::view::AsSubgraph<&Self::G>> {
        self.strongly_connected_sets()
            .iter()
            .map(|s| crate::view::AsSubgraph::induced(self.graph(), s.iter().cloned()))
            .collect()
    }

    /// The condensation: vertex `i` stands for component `i` of
    /// [`strongly_connected_sets`](Self::strongly_connected_sets).
    fn condensation(&self) -> crate::DiGraph<usize> {
        use crate::GraphMut;
        let sets = self.strongly_connected_sets();
        let g = self.graph();
        let mut comp_of = std::collections::HashMap::new();
        let mut c = crate::DiGraph::<usize>::simple();
        for (i, s) in sets.iter().enumerate() {
            c.add_vertex(i);
            for v in s {
                comp_of.insert(v, i);
            }
        }
        for e in g.edges() {
            let a = comp_of[g.edge_source(e).unwrap()];
            let b = comp_of[g.edge_target(e).unwrap()];
            if a != b {
                let _ = c.add_edge(&a, &b);
            }
        }
        c
    }
}

/// A planar embedding: for every vertex its incident edges in rotation order
/// (`PlanarityTestingAlgorithm.Embedding`).
#[derive(Debug, Clone)]
pub struct Embedding<V: Eq + Hash, E> {
    map: IndexMap<V, Vec<E>>,
}

impl<V: Eq + Hash, E> Embedding<V, E> {
    pub fn new(map: IndexMap<V, Vec<E>>) -> Self {
        Embedding { map }
    }
    /// The edges around `v` in clockwise or counter-clockwise order.
    pub fn edges_around(&self, v: &V) -> Option<&[E]> {
        self.map.get(v).map(|l| l.as_slice())
    }
    pub fn iter(&self) -> impl Iterator<Item = (&V, &[E])> {
        self.map.iter().map(|(v, l)| (v, l.as_slice()))
    }
}
