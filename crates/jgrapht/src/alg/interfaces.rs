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

//! Traits and result types shared by several algorithms (JGraphT's `alg.interfaces`).
//! More are added as the algorithm families that need them are ported.

use super::color::Coloring;
use super::cover::VertexCover;
use crate::Result;
use crate::graph::Graph;
use crate::graph::OutgoingGraph;
use crate::graph::UndirectedGraph;
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
    /// Creates a new instance.
    pub fn new(partitions: Vec<IndexSet<V>>) -> Self {
        Partitioning { partitions }
    }
    /// Returns the number partitions.
    pub fn number_partitions(&self) -> usize {
        self.partitions.len()
    }
    /// Returns the partition.
    pub fn partition(&self, index: usize) -> &IndexSet<V> {
        &self.partitions[index]
    }
    /// Returns the partitions.
    pub fn partitions(&self) -> &[IndexSet<V>] {
        &self.partitions
    }
    /// Returns the iter.
    pub fn iter(&self) -> impl Iterator<Item = &IndexSet<V>> {
        self.partitions.iter()
    }
}

/// Algorithm to compute a vertex partitioning of a graph.
///
/// # Rust notes
///
/// An algorithm computing a partitioning of the vertices.
pub trait PartitioningAlgorithm<G: Graph>
where
    G::Vertex: Eq + Hash,
{
    /// The partitioning, or `None` if the algorithm finds none.
    fn partitioning(&mut self) -> Option<&Partitioning<G::Vertex>>;
    /// Returns whether valid partitioning.
    fn is_valid_partitioning(&self, partitioning: &Partitioning<G::Vertex>) -> bool;
}

/// An algorithm for solving the [vertex $k$-center problem](https://encyclopedia.pub/entry/52164).
///
/// Center location problems are a class of optimization problems in graph theory and operations
/// research that aims selecting optimal locations for facilities (centers) to minimize the distance
/// or cost for serving a set of demand points or clients (vertices) in a graph.
///
/// # Rust notes
///
/// Algorithms choosing `k` center vertices of a complete graph (`CentersLocationAlgorithm`).
pub trait CentersLocationAlgorithm<V: Eq + Hash + Clone> {
    /// Chooses `k` centers and returns them together with the assignment of every client to its
    /// closest center and the covering radius, as one [`CentersSolution`].
    fn centers<G: UndirectedGraph<Vertex = V> + OutgoingGraph>(
        &mut self,
        graph: &G,
        k: usize,
    ) -> Result<CentersSolution<V>>;
}

/// A solution of the vertex $k$-center problem: the centers, the assignment of the other vertices
/// to them and the covering radius.
#[derive(Debug, Clone)]
pub struct CentersSolution<V: Eq + Hash> {
    centers: IndexSet<V>,
    assignment: IndexMap<V, V>,
    covering_radius: f64,
}

impl<V: Eq + Hash> CentersSolution<V> {
    pub(crate) fn new(
        centers: IndexSet<V>,
        assignment: IndexMap<V, V>,
        covering_radius: f64,
    ) -> Self {
        CentersSolution {
            centers,
            assignment,
            covering_radius,
        }
    }

    /// The chosen centers.
    pub fn centers(&self) -> &IndexSet<V> {
        &self.centers
    }

    /// The mapping $f: V \setminus C \rightarrow C$ from every client to the center it is assigned
    /// to (its closest one).
    pub fn assignment(&self) -> &IndexMap<V, V> {
        &self.assignment
    }

    /// The largest distance from a client to its center.
    pub fn covering_radius(&self) -> f64 {
        self.covering_radius
    }
}

/// A decomposition of a tree into vertex-disjoint paths (`TreeToPathDecompositionAlgorithm.PathDecomposition`).
#[derive(Debug, Clone)]
pub struct PathDecomposition<V, E> {
    edges: HashSet<E>,
    paths: Vec<GraphWalk<V, E>>,
}

impl<V, E> PathDecomposition<V, E> {
    /// Creates a new instance.
    pub fn new(edges: HashSet<E>, paths: Vec<GraphWalk<V, E>>) -> Self {
        PathDecomposition { edges, paths }
    }
    /// The edges lying on the paths.
    pub fn edges(&self) -> &HashSet<E> {
        &self.edges
    }
    /// Returns the paths.
    pub fn paths(&self) -> &[GraphWalk<V, E>] {
        &self.paths
    }
    /// Returns the number of paths.
    pub fn number_of_paths(&self) -> usize {
        self.paths.len()
    }
}

/// Algorithm to compute a [lowest common ancestor](https://en.wikipedia.org/wiki/Lowest_common_ancestor) in a tree, forest or DAG.
///
/// # Rust notes
///
/// Lowest common ancestor queries (`LowestCommonAncestorAlgorithm`).
pub trait LowestCommonAncestorAlgorithm<V: Eq + Hash + Clone> {
    /// The lowest common ancestor of `a` and `b`; `Ok(None)` when they have none (for example
    /// they lie in different trees). An error if a vertex is not in the graph.
    fn lca(&self, a: &V, b: &V) -> Result<Option<V>>;

    /// Returns the batch lca.
    fn batch_lca(&self, queries: &[(V, V)]) -> Result<Vec<Option<V>>> {
        queries.iter().map(|(a, b)| self.lca(a, b)).collect()
    }
}

/// A lowest-common-ancestor algorithm that can report *all* lowest common ancestors of two
/// vertices, which in a DAG can be more than one (in a tree there is at most one).
///
/// Only algorithms that work on DAGs implement it; the tree-only finders implement just
/// [`LowestCommonAncestorAlgorithm`]:
///
/// ```compile_fail,E0599
/// use jgrapht::alg::interfaces::AllLowestCommonAncestors;
/// use jgrapht::alg::lca::BinaryLiftingLcaFinder;
/// use jgrapht::{DiGraph, GraphMutExt};
///
/// let mut tree = DiGraph::<u32>::simple();
/// tree.add_edge_with_vertices(0, 1).unwrap();
/// let finder = BinaryLiftingLcaFinder::new(&tree, 0).unwrap();
/// let _ = finder.lca_set(&0, &1); // a tree finder has no `lca_set`
/// ```
pub trait AllLowestCommonAncestors<V: Eq + Hash + Clone>: LowestCommonAncestorAlgorithm<V> {
    /// All lowest common ancestors of `a` and `b`; empty when they have none.
    fn lca_set(&self, a: &V, b: &V) -> Result<HashSet<V>>;
}

/// A link prediction algorithm.
///
/// A link prediction algorithm provides a score $s_{uv}$ for any pair of vertices $u,v \in V$ in the
/// graph such that $e=(u,v) \notin E$. The nature, the magnitude and possible interpretation of such
/// a score depends solely on the actual algorithm, meaning that it might be a similarity score, a
/// distance metric, a probability, or even something completely unrelated.
///
/// Depending on the particular algorithm, a possible interpretation of the scores might be that they
/// measure similarity between vertices $u$ and $v$. Thus, given such scores one could sort the edges
/// in decreasing order and pick the top-k as links (edges) which are likely to exist.
///
/// # Rust notes
///
/// Scores for pairs of vertices that might become linked (`LinkPredictionAlgorithm`).
pub trait LinkPredictionAlgorithm<V: Clone> {
    /// The score of the pair; [`GraphError::IndexNotWellDefined`](crate::GraphError) when the index is
    /// undefined for it.
    fn predict(&self, u: &V, v: &V) -> Result<f64>;

    /// Returns the predict all.
    fn predict_all(&self, queries: &[(V, V)]) -> Result<Vec<(V, V, f64)>> {
        queries
            .iter()
            .map(|(u, v)| Ok((u.clone(), v.clone(), self.predict(u, v)?)))
            .collect()
    }
}

/// A maximal clique enumeration algorithm.
///
/// # Rust notes
///
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

/// Computes a (weighted) [vertex cover](http://mathworld.wolfram.com/VertexCover.html) in
/// an undirected graph. A vertex cover of a graph is a set of vertices such that each edge of the
/// graph is incident to at least one vertex in the set. A minimum vertex cover is a vertex cover
/// having the smallest possible number of vertices for a given graph. The size of a minimum vertex
/// cover of a graph $G$ is known as the vertex cover number. A vertex cover of minimum weight is a
/// vertex cover where the sum of weights assigned to the individual vertices in the cover has been
/// minimized. The minimum vertex cover problem is a special case of the minimum weighted vertex
/// cover problem where all vertices have equal weight. Consequently, any algorithm designed for the
/// weighted version of the problem can also solve instances of the unweighted version.
///
/// # Rust notes
///
/// Computes a vertex cover (`VertexCoverAlgorithm`).
pub trait VertexCoverAlgorithm<V: Clone> {
    /// Returns the vertex cover.
    fn vertex_cover(&self) -> Result<VertexCover<V>>;
}

/// An algorithm which computes a graph vertex coloring.
///
/// # Rust notes
///
/// Computes a proper vertex coloring (`VertexColoringAlgorithm`).
pub trait VertexColoringAlgorithm<V: Clone + Eq + Hash> {
    /// Returns the coloring.
    fn coloring(&self) -> Result<Coloring<V>>;
}

/// A partition of the vertices into clusters (`ClusteringAlgorithm.Clustering`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clustering<V: Eq + Hash> {
    clusters: Vec<IndexSet<V>>,
}

impl<V: Eq + Hash> Clustering<V> {
    /// Creates a new instance.
    pub fn new(clusters: Vec<IndexSet<V>>) -> Self {
        Clustering { clusters }
    }
    /// Returns the number clusters.
    pub fn number_clusters(&self) -> usize {
        self.clusters.len()
    }
    /// Returns the clusters.
    pub fn clusters(&self) -> &[IndexSet<V>] {
        &self.clusters
    }
    /// Returns the iter.
    pub fn iter(&self) -> impl Iterator<Item = &IndexSet<V>> {
        self.clusters.iter()
    }
}

/// An algorithm which computes a graph vertex clustering.
///
/// # Rust notes
///
/// An algorithm that clusters the vertices of a graph (`ClusteringAlgorithm`).
pub trait ClusteringAlgorithm<V: Eq + Hash + Clone> {
    /// Returns the clustering.
    fn clustering(&self) -> Result<Clustering<V>>;
}

/// A strong connectivity inspector algorithm.
///
/// # Rust notes
///
/// Algorithms computing the strongly connected components of a directed graph
/// (`StrongConnectivityAlgorithm`). Results are computed when the algorithm is constructed.
pub trait StrongConnectivityAlgorithm {
    /// The g type.
    type G: crate::graph::DirectedGraph;

    /// Returns the graph.
    fn graph(&self) -> &Self::G;
    /// The vertex sets of the strongly connected components.
    fn strongly_connected_sets(&self) -> &[HashSet<<Self::G as Graph>::Vertex>];

    /// Returns whether strongly connected.
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
    /// Creates a new instance.
    pub fn new(map: IndexMap<V, Vec<E>>) -> Self {
        Embedding { map }
    }
    /// The edges around `v` in clockwise or counter-clockwise order.
    pub fn edges_around(&self, v: &V) -> Option<&[E]> {
        self.map.get(v).map(|l| l.as_slice())
    }
    /// Returns the iter.
    pub fn iter(&self) -> impl Iterator<Item = (&V, &[E])> {
        self.map.iter().map(|(v, l)| (v, l.as_slice()))
    }
}
