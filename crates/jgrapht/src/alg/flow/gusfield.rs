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

use super::base::{MaximumFlowValueAlgorithm, MinimumCut, MinimumSTCutAlgorithm};
use super::{DEFAULT_EPSILON, PushRelabelMFImpl};
use crate::error::{GraphError, Result};
use crate::graph::IncidenceGraph;
use crate::graph::UndirectedGraph;
use crate::graph::{DefaultEdge, Graph, GraphExt, GraphMut, GraphMutExt, UGraph};
use indexmap::IndexSet;
use std::collections::{HashMap, VecDeque};

type CutAlgorithm<'g, G> =
    Box<dyn MinimumSTCutAlgorithm<<G as Graph>::Vertex, <G as Graph>::Edge> + 'g>;

/// Shared set-up of the two Gusfield algorithms: the vertex order and index map.
fn index_vertices<G: Graph>(network: &G) -> Result<(Vec<G::Vertex>, HashMap<G::Vertex, usize>)> {
    if network.vertex_count() < 2 {
        return Err(GraphError::InvalidGraph(
            crate::GraphProperty::AtLeastTwoVertices,
        ));
    }
    let vertex_list: Vec<G::Vertex> = network.vertices().cloned().collect();
    let index_map = vertex_list
        .iter()
        .enumerate()
        .map(|(i, v)| (v.clone(), i))
        .collect();
    Ok((vertex_list, index_map))
}

/// This class computes an Equivalent Flow Tree (EFT) using the algorithm proposed by Dan Gusfield.
/// EFTs can be used to efficiently calculate the maximum flow for all pairs of vertices. The
/// algorithm is described in: *Gusfield, D, Very simple methods for all pairs network flow
/// analysis. SIAM Journal on Computing, 19(1), p142-155, 1990*
/// In an undirected graph, there exist $frac{n(n-1)}{2}$ different vertex pairs. This class computes
/// the maximum flow between each of these pairs efficiently by performing exactly $(n-1)$ minimum
/// $s-t$ cut computations. If your application requires fewer than $(n-1)$ flow calculations,
/// consider computing the maximum flows manually through `MaximumFlowAlgorithm`.
///
/// The runtime complexity of this class is $O((V-1)Q)$, where $Q$ is the runtime complexity of the
/// algorithm used to compute $s-t$ cuts in the graph. By default, this class uses the
/// `PushRelabelMFImpl` implementation to calculate minimum $s-t$ cuts. This class has a
/// runtime complexity of $O(V^3)$, resulting in a $O(V^4)$ runtime complexity for the overal
/// algorithm.
///
/// Note: this class performs calculations in a lazy manner. The EFT is not calculated until the
/// first invocation of `GusfieldEquivalentFlowTree#getMaximumFlowValue(Object, Object)` or
/// `GusfieldEquivalentFlowTree#getEquivalentFlowTree()`. Moreover, this class *only*
/// calculates the value of the maximum flow between a source-destination pair; it does not calculate
/// the corresponding flow per edge. If you need to know the exact flow through an edge, use one of
/// the alternative `MaximumFlowAlgorithm` implementations.
///
/// Warning: EFTs do not allow you to calculate minimum cuts for all pairs of vertex! For that,
/// Gomory-Hu cut trees are required! Use the `GusfieldGomoryHuCutTree` implementation instead.
///
/// This class does not support changes to the underlying graph. The behavior of this class is
/// undefined when the graph is modified after instantiating this class.
///
/// # Rust notes
///
/// `GusfieldEquivalentFlowTree`: all-pairs maximum flow values from `n - 1` minimum cut
/// computations, kept in an equivalent flow tree.
///
/// It implements [`MaximumFlowValueAlgorithm`] only: an equivalent flow tree knows the *value* of
/// the maximum flow between any two vertices but not the flow on each edge, so it cannot be used
/// where [`MaximumFlowAlgorithm`](super::MaximumFlowAlgorithm) is required:
///
/// ```compile_fail,E0277
/// use jgrapht::alg::flow::{GusfieldEquivalentFlowTree, MaximumFlowAlgorithm};
/// use jgrapht::{GraphMutExt, UGraph};
///
/// fn needs_edge_flows<A: MaximumFlowAlgorithm<u32, jgrapht::DefaultEdge>>(_: &A) {}
/// let mut g = UGraph::<u32>::simple().weighted();
/// g.add_edge_with_vertices(0, 1).unwrap();
/// let eft = GusfieldEquivalentFlowTree::new(&g).unwrap();
/// needs_edge_flows(&eft);
/// ```
pub struct GusfieldEquivalentFlowTree<'g, G: Graph> {
    n: usize,
    minimum_st_cut_algorithm: CutAlgorithm<'g, G>,
    vertex_list: Vec<G::Vertex>,
    index_map: HashMap<G::Vertex, usize>,
    p: Option<Vec<usize>>,
    neighbors: Vec<usize>,
    flow_matrix: Vec<Vec<f64>>,
}

impl<'g, G: UndirectedGraph + IncidenceGraph + 'g + IncidenceGraph>
    GusfieldEquivalentFlowTree<'g, G>
{
    /// Constructs a new GusfieldEquivalentFlowTree instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(network: &'g G) -> Result<Self> {
        Self::with_epsilon(network, DEFAULT_EPSILON)
    }

    /// Creates an instance with the given epsilon.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn with_epsilon(network: &'g G, epsilon: f64) -> Result<Self> {
        Self::with_algorithm(
            network,
            Box::new(PushRelabelMFImpl::with_epsilon(network, epsilon)?),
        )
    }

    /// Uses `minimum_st_cut_algorithm` for the cut computations.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn with_algorithm(
        network: &'g G,
        minimum_st_cut_algorithm: CutAlgorithm<'g, G>,
    ) -> Result<Self> {
        let (vertex_list, index_map) = index_vertices(network)?;
        Ok(GusfieldEquivalentFlowTree {
            n: vertex_list.len(),
            minimum_st_cut_algorithm,
            vertex_list,
            index_map,
            p: None,
            neighbors: Vec::new(),
            flow_matrix: Vec::new(),
        })
    }

    /// Runs the algorithm
    fn calculate_equivalent_flow_tree(&mut self) -> Result<()> {
        let n = self.n;
        let mut flow_matrix = vec![vec![0.0; n]; n];
        let mut p = vec![0usize; n];
        let mut neighbors = vec![0usize; n];
        for s in 1..n {
            let t = p[s];
            neighbors[s] = t;
            let cut = self
                .minimum_st_cut_algorithm
                .minimum_cut(&self.vertex_list[s], &self.vertex_list[t])?;
            let flow_value = cut.capacity();
            let source_partition = cut.source_partition();
            for i in s..n {
                if source_partition.contains(&self.vertex_list[i]) && p[i] == t {
                    p[i] = s;
                }
            }
            flow_matrix[s][t] = flow_value;
            flow_matrix[t][s] = flow_value;
            for i in 0..s {
                if i != t {
                    let v = flow_matrix[s][t].min(flow_matrix[t][i]);
                    flow_matrix[s][i] = v;
                    flow_matrix[i][s] = v;
                }
            }
        }
        self.flow_matrix = flow_matrix;
        self.p = Some(p);
        self.neighbors = neighbors;
        Ok(())
    }

    /// The equivalent flow tree: an edge `(i, neighbor(i))` weighted with their maximum flow value.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn equivalent_flow_tree(&mut self) -> Result<UGraph<G::Vertex>> {
        if self.p.is_none() {
            self.calculate_equivalent_flow_tree()?;
        }
        let mut tree = UGraph::<G::Vertex>::simple().weighted();
        tree.add_vertices(self.vertex_list.iter().cloned());
        for i in 1..self.n {
            let j = self.neighbors[i];
            tree.add_weighted_edge(
                &self.vertex_list[i],
                &self.vertex_list[j],
                self.flow_matrix[i][j],
            )?;
        }
        Ok(tree)
    }
}

impl<'g, G: UndirectedGraph + IncidenceGraph + 'g> MaximumFlowValueAlgorithm<G::Vertex>
    for GusfieldEquivalentFlowTree<'g, G>
{
    fn maximum_flow_value(&mut self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        let (s, t) = match (self.index_map.get(source), self.index_map.get(sink)) {
            (Some(&s), Some(&t)) => (s, t),
            _ => return Err(GraphError::VertexNotFound),
        };
        if self.p.is_none() {
            self.calculate_equivalent_flow_tree()?;
        }
        Ok(self.flow_matrix[s][t])
    }
}

/// This class computes a Gomory-Hu tree (GHT) using the algorithm proposed by Dan Gusfield. For a
/// definition of GHTs, refer to: *Gomory, R., Hu, T. Multi-terminal network flows. Journal of the
/// Socieity for Industrial and Applied mathematics, 9(4), p551-570, 1961.* GHTs can be used to
/// efficiently query the maximum flows and minimum cuts for all pairs of vertices. The algorithm is
/// described in: *Gusfield, D, Very simple methods for all pairs network flow analysis. SIAM
/// Journal on Computing, 19(1), p142-155, 1990*
/// In an undirected graph, there exist $\frac{n(n-1)}{2}$ different vertex pairs. This class
/// computes the maximum flow/minimum cut between each of these pairs efficiently by performing
/// exactly $(n-1)$ minimum $s-t$ cut computations. If your application needs fewer than $n-1$
/// flow/cut computations, consider computing the maximum flows/minimum cuts manually through
/// `MaximumFlowAlgorithm`/`MinimumSTCutAlgorithm`.
///
/// The runtime complexity of this class is $O((V-1)Q)$, where $Q$ is the runtime complexity of the
/// algorithm used to compute $s-t$ cuts in the graph. By default, this class uses the
/// `PushRelabelMFImpl` implementation to calculate minimum s-t cuts. This class has a runtime
/// complexity of $O(V^3)$, resulting in a $O(V^4)$ runtime complexity for the overall algorithm.
///
/// Note: this class performs calculations in a lazy manner. The GHT is not calculated until the
/// first invocation of `GusfieldGomoryHuCutTree#getMaximumFlowValue(Object, Object)` or
/// `GusfieldGomoryHuCutTree#getGomoryHuTree()`. Moreover, this class *only* calculates
/// the value of the maximum flow between a source-destination pair; it does not calculate the
/// corresponding flow per edge. If you need to know the exact flow through an edge, use one of the
/// alternative `MaximumFlowAlgorithm` implementations.
///
/// In contrast to an Equivalent Flow Tree (`GusfieldEquivalentFlowTree`), Gomory-Hu trees also
/// provide all minimum cuts for all pairs of vertices!
///
/// This class does not support changes to the underlying graph. The behavior of this class is
/// undefined when the graph is modified after instantiating this class.
///
/// # Rust notes
///
/// `GusfieldGomoryHuCutTree`: a Gomory–Hu cut tree from `n - 1` minimum cut computations. Answers
/// maximum flow values and minimum cuts between any two vertices.
pub struct GusfieldGomoryHuCutTree<'g, G: Graph> {
    network: &'g G,
    n: usize,
    minimum_st_cut_algorithm: CutAlgorithm<'g, G>,
    vertex_list: Vec<G::Vertex>,
    index_map: HashMap<G::Vertex, usize>,
    p: Option<Vec<usize>>,
    fl: Vec<f64>,
    flow_matrix: Vec<Vec<f64>>,
    gomory_hu_tree: Option<UGraph<G::Vertex>>,
}

impl<'g, G: UndirectedGraph + IncidenceGraph + 'g + IncidenceGraph> GusfieldGomoryHuCutTree<'g, G> {
    /// Constructs a new GusfieldEquivalentFlowTree instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(network: &'g G) -> Result<Self> {
        Self::with_epsilon(network, DEFAULT_EPSILON)
    }

    /// Creates an instance with the given epsilon.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn with_epsilon(network: &'g G, epsilon: f64) -> Result<Self> {
        Self::with_algorithm(
            network,
            Box::new(PushRelabelMFImpl::with_epsilon(network, epsilon)?),
        )
    }

    /// Uses `minimum_st_cut_algorithm` for the cut computations.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn with_algorithm(
        network: &'g G,
        minimum_st_cut_algorithm: CutAlgorithm<'g, G>,
    ) -> Result<Self> {
        let (vertex_list, index_map) = index_vertices(network)?;
        Ok(GusfieldGomoryHuCutTree {
            network,
            n: vertex_list.len(),
            minimum_st_cut_algorithm,
            vertex_list,
            index_map,
            p: None,
            fl: Vec::new(),
            flow_matrix: Vec::new(),
            gomory_hu_tree: None,
        })
    }

    /// Runs the algorithm
    fn calculate_gomory_hu_tree(&mut self) -> Result<()> {
        let n = self.n;
        let mut flow_matrix = vec![vec![0.0; n]; n];
        let mut p = vec![0usize; n];
        let mut fl = vec![0.0; n];
        for s in 1..n {
            let t = p[s];
            let cut = self
                .minimum_st_cut_algorithm
                .minimum_cut(&self.vertex_list[s], &self.vertex_list[t])?;
            let flow_value = cut.capacity();
            let source_partition = cut.source_partition();
            fl[s] = flow_value;
            for i in 0..n {
                if i != s && source_partition.contains(&self.vertex_list[i]) && p[i] == t {
                    p[i] = s;
                }
            }
            if source_partition.contains(&self.vertex_list[p[t]]) {
                p[s] = p[t];
                p[t] = s;
                fl[s] = fl[t];
                fl[t] = flow_value;
            }
            flow_matrix[s][t] = flow_value;
            flow_matrix[t][s] = flow_value;
            for i in 0..s {
                if i != t {
                    let v = flow_matrix[s][t].min(flow_matrix[t][i]);
                    flow_matrix[s][i] = v;
                    flow_matrix[i][s] = v;
                }
            }
        }
        self.flow_matrix = flow_matrix;
        self.p = Some(p);
        self.fl = fl;
        Ok(())
    }

    /// The Gomory–Hu tree: an edge `(i, p(i))` weighted with the cut value `fl(i)`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    ///
    /// # Panics
    ///
    /// Panics only if an internal invariant is violated, which indicates a bug in this library.
    pub fn gomory_hu_tree(&mut self) -> Result<UGraph<G::Vertex>> {
        if self.p.is_none() {
            self.calculate_gomory_hu_tree()?;
        }
        let p = self.p.as_ref().unwrap();
        let mut tree = UGraph::<G::Vertex>::simple().weighted();
        tree.add_vertices(self.vertex_list.iter().cloned());
        for i in 1..self.n {
            tree.add_weighted_edge(&self.vertex_list[i], &self.vertex_list[p[i]], self.fl[i])?;
        }
        Ok(tree)
    }

    /// A global minimum cut: the minimum $s$-$t$ cut for the endpoints of the cheapest tree edge.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if the graph lacks a property the algorithm requires; errors of the underlying graph operations are propagated.
    ///
    /// # Panics
    ///
    /// Panics only if the graph breaks the integrity contract of [`Graph`] (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
    pub fn global_minimum_cut(&mut self) -> Result<MinimumCut<G::Vertex, G::Edge>> {
        if self.gomory_hu_tree.is_none() {
            self.gomory_hu_tree = Some(self.gomory_hu_tree()?);
        }
        let tree = self.gomory_hu_tree.as_ref().unwrap();
        let mut cheapest: Option<(&DefaultEdge, f64)> = None;
        for e in tree.edges() {
            let w = tree.weight_of(e);
            if cheapest.is_none_or(|(_, best)| w < best) {
                cheapest = Some((e, w));
            }
        }
        let (edge, _) = cheapest.ok_or(GraphError::InvalidGraph(crate::GraphProperty::NonEmpty))?;
        let (u, v) = tree.edge_endpoints(edge).unwrap();
        let (u, v) = (u.clone(), v.clone());
        self.minimum_cut(&u, &v)
    }

    /// BFS method to find the edges in the shortest path from a source to a target vertex in a tree
    /// graph.
    fn find_path_between(
        &self,
        tree: &UGraph<G::Vertex>,
        source: &G::Vertex,
        target: &G::Vertex,
    ) -> Vec<DefaultEdge> {
        let mut visited = vec![false; self.vertex_list.len()];
        let mut predecessor: HashMap<G::Vertex, G::Vertex> = HashMap::new();
        let mut queue = VecDeque::from([source.clone()]);
        let mut found = false;
        while !found {
            let Some(next) = queue.pop_front() else { break };
            for v in tree.neighbors_of(&next) {
                if !visited[self.index_map[v]] {
                    predecessor.insert(v.clone(), next.clone());
                    queue.push_back(v.clone());
                }
                if v == target {
                    found = true;
                    break;
                }
            }
            visited[self.index_map[&next]] = true;
        }
        let mut edges = IndexSet::new();
        let mut v = target.clone();
        while v != *source {
            let pred = predecessor[&v].clone();
            edges.insert(*tree.get_edge(&v, &pred).unwrap());
            v = pred;
        }
        edges.into_iter().collect()
    }
}

impl<'g, G: UndirectedGraph + IncidenceGraph + 'g> GusfieldGomoryHuCutTree<'g, G> {
    /// The source side of a minimum cut between `source` and `target`, read off the tree: remove
    /// the cheapest edge of the tree path and take the component of `source`.
    fn source_side(
        &mut self,
        source: &G::Vertex,
        target: &G::Vertex,
    ) -> Result<IndexSet<G::Vertex>> {
        if self.gomory_hu_tree.is_none() {
            self.gomory_hu_tree = Some(self.gomory_hu_tree()?);
        }
        if self.gomory_hu_tree.is_none() {
            self.gomory_hu_tree = Some(self.gomory_hu_tree().expect("tree"));
        }
        let path_edges = {
            let tree = self.gomory_hu_tree.as_ref().unwrap();
            self.find_path_between(tree, source, target)
        };
        let tree = self.gomory_hu_tree.as_mut().unwrap();
        let mut cheapest: Option<(DefaultEdge, f64)> = None;
        for e in &path_edges {
            let w = tree.weight_of(e);
            if cheapest.is_none_or(|(_, best)| w < best) {
                cheapest = Some((*e, w));
            }
        }
        let (edge, weight) = cheapest.ok_or(GraphError::SourceEqualsSink)?;
        let (u, v) = {
            let (u, v) = tree.edge_endpoints(&edge).unwrap();
            (u.clone(), v.clone())
        };
        tree.remove_edge(&edge);
        let component = crate::alg::connectivity::ConnectivityInspector::new()
            .connected_set_of(&*tree, source)
            .expect("source is in the tree");
        let partition = self
            .vertex_list
            .iter()
            .filter(|x| component.contains(*x))
            .cloned()
            .collect();
        tree.add_edge_with(&u, &v, edge)
            .expect("re-adding the removed edge");
        tree.set_edge_weight(&edge, weight).expect("weighted tree");
        Ok(partition)
    }
}

impl<'g, G: UndirectedGraph + IncidenceGraph + 'g> MaximumFlowValueAlgorithm<G::Vertex>
    for GusfieldGomoryHuCutTree<'g, G>
{
    fn maximum_flow_value(&mut self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        let (s, t) = match (self.index_map.get(source), self.index_map.get(sink)) {
            (Some(&s), Some(&t)) => (s, t),
            _ => return Err(GraphError::VertexNotFound),
        };
        if self.p.is_none() {
            self.calculate_gomory_hu_tree()?;
        }
        Ok(self.flow_matrix[s][t])
    }
}

impl<'g, G: UndirectedGraph + IncidenceGraph + 'g> MinimumSTCutAlgorithm<G::Vertex, G::Edge>
    for GusfieldGomoryHuCutTree<'g, G>
{
    fn minimum_cut(
        &mut self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<MinimumCut<G::Vertex, G::Edge>> {
        let capacity = self.maximum_flow_value(source, sink)?;
        let source_partition = self.source_side(source, sink)?;
        let g = self.network;
        let sink_partition: IndexSet<G::Vertex> = g
            .vertices()
            .filter(|v| !source_partition.contains(*v))
            .cloned()
            .collect();
        let cut_edges: IndexSet<G::Edge> = g
            .edges()
            .filter(|e| {
                g.edge_endpoints(e).is_some_and(|(u, v)| {
                    source_partition.contains(u) ^ source_partition.contains(v)
                })
            })
            .cloned()
            .collect();
        Ok(MinimumCut::new(
            capacity,
            source_partition,
            sink_partition,
            cut_edges,
        ))
    }
}
