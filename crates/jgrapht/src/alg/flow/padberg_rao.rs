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

use crate::graph::UndirectedGraph;
use crate::graph::IncidenceGraph;
use super::base::MinimumSTCutAlgorithm;
use super::{DEFAULT_EPSILON, GusfieldGomoryHuCutTree, PushRelabelMFImpl};
use crate::alg::connectivity::ConnectivityInspector;
use crate::error::{GraphError, Result};
use crate::graph::{DefaultEdge, Graph, GraphExt, GraphMut, UGraph};
use indexmap::IndexSet;
use std::collections::VecDeque;

/// Implementation of the algorithm by Padberg and Rao to compute Odd Minimum Cut-Sets. Let $G=(V,E)$
/// be an undirected, simple weighted graph, where all edge weights are positive. Let $T \subset V$
/// with $|T|$ even, be a set of vertices that are labelled *odd*. A cut-set $(U:V-U)$ is called
/// odd if $|T \cap U|$ is an odd number. Let $c(U:V-U)$ be the weight of the cut, that is, the sum
/// of weights of the edges which have exactly one endpoint in $U$ and one endpoint in $V-U$. The
/// problem of finding an odd minimum cut-set in $G$ is stated as follows: Find $W \subseteq V$ such
/// that $c(W:V-W)=min(c(U:V-U)|U \subseteq V, |T \cap U|$ is odd).
///
/// The algorithm has been published in: Padberg, M. Rao, M. Odd Minimum Cut-Sets and b-Matchings.
/// Mathematics of Operations Research, 7(1), p67-80, 1982. A more concise description is published
/// in: Letchford, A. Reinelt, G. Theis, D. Odd minimum cut-sets and b-matchings revisited. SIAM
/// Journal of Discrete Mathematics, 22(4), p1480-1487, 2008.
///
/// The runtime complexity of this algorithm is dominated by the runtime complexity of the algorithm
/// used to compute A Gomory-Hu tree on graph $G$. Consequently, the runtime complexity of this class
/// is $O(V^4)$.
///
/// This class does not support changes to the underlying graph. The behavior of this class is
/// undefined when the graph is modified after instantiating this class.
///
/// # Rust notes
///
/// `PadbergRaoOddMinimumCutset`: a minimum weight cut that separates the given "odd" vertices into
/// two odd-sized sets, read off a Gomory–Hu tree. Edge weights must be non-negative.
pub struct PadbergRaoOddMinimumCutset<'g, G: Graph> {
    network: &'g G,
    odd_vertices: IndexSet<G::Vertex>,
    gusfield: GusfieldGomoryHuCutTree<'g, G>,
    gomory_hu_tree: Option<UGraph<G::Vertex>>,
    minimum_cut_weight: f64,
    source_partition_minimum_cut: Option<IndexSet<G::Vertex>>,
}

impl<'g, G: UndirectedGraph + 'g + IncidenceGraph> PadbergRaoOddMinimumCutset<'g, G> {
    /// Creates a new instance of the PadbergRaoOddMinimumCutset algorithm.
    pub fn new(network: &'g G) -> Result<Self> {
        Self::with_epsilon(network, DEFAULT_EPSILON)
    }

    pub fn with_epsilon(network: &'g G, epsilon: f64) -> Result<Self> {
        Self::with_algorithm(
            network,
            Box::new(PushRelabelMFImpl::with_epsilon(network, epsilon)?),
        )
    }

    /// Uses `minimum_st_cut_algorithm` for the cut computations.
    pub fn with_algorithm(
        network: &'g G,
        minimum_st_cut_algorithm: Box<dyn MinimumSTCutAlgorithm<G::Vertex, G::Edge> + 'g>,
    ) -> Result<Self> {
        Ok(PadbergRaoOddMinimumCutset {
            network,
            odd_vertices: IndexSet::new(),
            gusfield: GusfieldGomoryHuCutTree::with_algorithm(network, minimum_st_cut_algorithm)?,
            gomory_hu_tree: None,
            minimum_cut_weight: f64::MAX,
            source_partition_minimum_cut: None,
        })
    }

    /// Computes the minimum odd cut-set for `odd_vertices` (an even number of vertices of the
    /// network); `use_tree_compression` splits the odd vertices into clusters instead of
    /// scanning every tree edge.
    pub fn calculate_min_cut(
        &mut self,
        odd_vertices: IndexSet<G::Vertex>,
        use_tree_compression: bool,
    ) -> Result<f64> {
        self.minimum_cut_weight = f64::MAX;
        if odd_vertices.len() % 2 == 1 {
            return Err(GraphError::InvalidArgument(
                "There needs to be an even number of odd vertices".into(),
            ));
        }
        debug_assert!(odd_vertices.iter().all(|v| self.network.contains_vertex(v)));
        self.odd_vertices = odd_vertices;
        self.gomory_hu_tree = Some(self.gusfield.gomory_hu_tree()?);
        if use_tree_compression {
            self.calculate_min_cut_with_tree_compression()
        } else {
            Ok(self.calculate_min_cut_without_tree_compression())
        }
    }

    /// Modified implementation of the algorithm proposed in Odd Minimum Cut-sets and b-matchings by
    /// Padberg and Rao. The optimal cut is directly computed on the Gomory-Hu tree computed for
    /// graph $G$. This approach iterates efficiently over all possible cuts of the graph (there are
    /// $|V|$ such cuts).
    fn calculate_min_cut_without_tree_compression(&mut self) -> f64 {
        let tree = self.gomory_hu_tree.as_mut().unwrap();
        let edges: Vec<DefaultEdge> = tree.edges().cloned().collect();
        for edge in edges {
            let (source, target) = {
                let (u, v) = tree.edge_endpoints(&edge).unwrap();
                (u.clone(), v.clone())
            };
            let edge_weight = tree.edge_weight(&edge).unwrap();
            if edge_weight >= self.minimum_cut_weight {
                continue;
            }
            tree.remove_edge(&edge); // temporarily
            let component = ConnectivityInspector::new()
                .connected_set_of(&*tree, &source)
                .expect("endpoint of a tree edge");
            let source_partition: IndexSet<G::Vertex> = tree
                .vertices()
                .filter(|v| component.contains(*v))
                .cloned()
                .collect();
            if Self::is_odd_vertex_set(&source_partition, &self.odd_vertices) {
                self.minimum_cut_weight = edge_weight;
                self.source_partition_minimum_cut = Some(source_partition);
            }
            tree.add_edge_with(&source, &target, edge)
                .expect("placing the edge back");
            tree.set_edge_weight(&edge, edge_weight)
                .expect("weighted tree");
        }
        self.minimum_cut_weight
    }

    /// Implementation of the algorithm proposed in Odd Minimum Cut-sets and b-matchings by Padberg
    /// and Rao. The algorithm evaluates at most $|T|$ cuts in the Gomory-Hu tree.
    fn calculate_min_cut_with_tree_compression(&mut self) -> Result<f64> {
        let mut queue: VecDeque<IndexSet<G::Vertex>> = VecDeque::new();
        queue.push_back(self.odd_vertices.clone());
        while let Some(next_cluster) = queue.pop_front() {
            self.split_cluster(&next_cluster, &mut queue)?;
        }
        Ok(self.minimum_cut_weight)
    }

    /// Takes a set of odd vertices with cardinality $2$ or more, and splits them into $2$ new
    /// non-empty sets.
    fn split_cluster(
        &mut self,
        cluster: &IndexSet<G::Vertex>,
        queue: &mut VecDeque<IndexSet<G::Vertex>>,
    ) -> Result<()> {
        debug_assert!(cluster.len() >= 2);
        let odd_node1 = &cluster[0];
        let odd_node2 = &cluster[1];
        let cut_weight = self.gusfield.calculate_min_cut(odd_node1, odd_node2)?;
        let mut source_partition: Option<IndexSet<G::Vertex>> = None;
        if cut_weight < self.minimum_cut_weight {
            let partition = self.gusfield.source_partition().clone();
            if Self::is_odd_vertex_set(&partition, &self.odd_vertices) {
                self.minimum_cut_weight = cut_weight;
                self.source_partition_minimum_cut = Some(partition.clone());
            }
            source_partition = Some(partition);
        }
        if cluster.len() == 2 {
            return Ok(());
        }
        let source_partition = match source_partition {
            Some(p) => p,
            None => self.gusfield.source_partition().clone(),
        };
        let split1 = Self::intersection(cluster, &source_partition);
        let split2: IndexSet<G::Vertex> = cluster
            .iter()
            .filter(|v| !split1.contains(*v))
            .cloned()
            .collect();
        if split1.len() > 1 {
            queue.push_back(split1);
        }
        if split2.len() > 1 {
            queue.push_back(split2);
        }
        Ok(())
    }

    /// Efficient way to compute the intersection between two sets
    fn intersection(set1: &IndexSet<G::Vertex>, set2: &IndexSet<G::Vertex>) -> IndexSet<G::Vertex> {
        let (a, b) = if set1.len() <= set2.len() {
            (set1, set2)
        } else {
            (set2, set1)
        };
        a.iter().filter(|v| b.contains(*v)).cloned().collect()
    }

    /// Whether `vertices` contains an odd number of the `odd_vertices`.
    pub fn is_odd_vertex_set(
        vertices: &IndexSet<G::Vertex>,
        odd_vertices: &IndexSet<G::Vertex>,
    ) -> bool {
        if vertices.len() < odd_vertices.len() {
            vertices
                .iter()
                .filter(|v| odd_vertices.contains(*v))
                .count()
                % 2
                == 1
        } else {
            odd_vertices
                .iter()
                .filter(|v| vertices.contains(*v))
                .count()
                % 2
                == 1
        }
    }

    /// The source side of the last minimum odd cut-set.
    pub fn source_partition(&self) -> Option<&IndexSet<G::Vertex>> {
        self.source_partition_minimum_cut.as_ref()
    }

    /// The sink side of the last minimum odd cut-set.
    pub fn sink_partition(&self) -> Option<IndexSet<G::Vertex>> {
        let source = self.source_partition_minimum_cut.as_ref()?;
        Some(
            self.network
                .vertices()
                .filter(|v| !source.contains(*v))
                .cloned()
                .collect(),
        )
    }

    /// The edges of the last minimum odd cut-set.
    pub fn cut_edges(&self) -> Option<IndexSet<G::Edge>> {
        let source = self.source_partition_minimum_cut.as_ref()?;
        let g = self.network;
        Some(
            g.edges()
                .filter(|e| {
                    let (u, v) = g.edge_endpoints(e).unwrap();
                    source.contains(u) ^ source.contains(v)
                })
                .cloned()
                .collect(),
        )
    }
}
