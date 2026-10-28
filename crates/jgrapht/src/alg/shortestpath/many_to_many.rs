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

use super::{
    BidirectionalDijkstraShortestPath, DijkstraClosestFirstIterator, ManyToManyPaths,
    ShortestPathAlgorithm, SingleSourcePaths,
};
use crate::graph::IncidenceGraph;
use crate::graph::IncomingGraph;
use crate::graph::OutgoingGraph;
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Naive algorithm for many-to-many shortest paths problem using.
///
/// Time complexity of the algorithm is $O(|S||T|C)$, where $S$ is the set of source vertices, $T$ is
/// the set of target vertices and $C$ is the complexity of the
/// `ShortestPathAlgorithm#getPath(Object, Object)` method of the provided implementation.
///
/// For every pair of `source` and `target` vertices computes a shortest path between
/// them using provided implementation of `ShortestPathAlgorithm`. By default this
/// implementation uses `BidirectionalDijkstraShortestPath`. If desired, a different
/// implementation can be provided via the `function` constructor parameter.
///
/// The computation complexity of the algorithm consists of two main components - the $|S||T|$
/// multiplier and the $C$ multiplier. This yields two bottlenecks for the algorithm. First of them
/// is the situation when the total number calls to
/// `ShortestPathAlgorithm#getPath(Object, Object)` is large. The second situation is when the
/// complexity of the individual call to `ShortestPathAlgorithm#getPath(Object, Object)` takes
/// a lot of time. Therefore the ideal use cases for this algorithm are small graphs or large graphs
/// with low total number of source and target vertices.
///
/// # Rust notes
///
/// Naive many-to-many implementation, running a point-to-point algorithm for every pair.
pub struct DefaultManyToManyShortestPaths<'g, G: Graph + 'g> {
    graph: &'g G,
    algorithm_factory: Box<dyn Fn(&'g G) -> Box<dyn ShortestPathAlgorithm<G> + 'g> + 'g>,
}

impl<'g, G: 'g + IncidenceGraph> DefaultManyToManyShortestPaths<'g, G> {
    /// Constructs a new instance of the algorithm for a given `graph`. The `function` is
    /// defaulted to returning `BidirectionalDijkstraShortestPath`.
    pub fn new(graph: &'g G) -> Self {
        Self {
            graph,
            algorithm_factory: Box::new(|graph| {
                Box::new(BidirectionalDijkstraShortestPath::new(graph))
            }),
        }
    }

    /// Supply the point-to-point algorithm used for every source/target pair.
    pub fn with_algorithm<A, F>(graph: &'g G, factory: F) -> Self
    where
        A: ShortestPathAlgorithm<G> + 'g,
        F: Fn(&'g G) -> A + 'g,
    {
        Self {
            graph,
            algorithm_factory: Box::new(move |graph| Box::new(factory(graph))),
        }
    }

    /// Computes shortest paths from all vertices in `sources` to all vertices in
    /// `targets`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn get_many_to_many_paths(
        &self,
        sources: impl IntoIterator<Item = G::Vertex>,
        targets: impl IntoIterator<Item = G::Vertex>,
    ) -> Result<ManyToManyPaths<G::Vertex, G::Edge>> {
        let sources: HashSet<_> = sources.into_iter().collect();
        let targets: HashSet<_> = targets.into_iter().collect();
        validate_sets(self.graph, &sources, &targets)?;
        let algorithm = (self.algorithm_factory)(self.graph);
        let mut paths = HashMap::new();
        for source in &sources {
            for target in &targets {
                if let Some(path) = algorithm.path(source, target)? {
                    paths.insert((source.clone(), target.clone()), path);
                }
            }
        }
        Ok(ManyToManyPaths::from_parts(sources, targets, paths))
    }
}

impl<G: Graph> ShortestPathAlgorithm<G> for DefaultManyToManyShortestPaths<'_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        if !self.graph.contains_vertex(source) {
            return Err(GraphError::VertexNotFound);
        }
        let algorithm = (self.algorithm_factory)(self.graph);
        let mut result = SingleSourcePaths {
            source: source.clone(),
            dist: HashMap::new(),
            pred: HashMap::new(),
        };
        for target in self.graph.vertices() {
            let Some(path) = algorithm.path(source, target)? else {
                continue;
            };
            result.dist.insert(target.clone(), path.weight);
            if let (Some(edge), Some(parent)) = (
                path.edges.last(),
                path.vertices.get(path.vertices.len().saturating_sub(2)),
            ) {
                result
                    .pred
                    .insert(target.clone(), (edge.clone(), parent.clone()));
            }
        }
        Ok(result)
    }

    /// Return the path from the `source` vertex to the `target` vertex. If no such
    /// path exists, `None` is returned.
    fn path(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Option<crate::walk::GraphWalk<G::Vertex, G::Edge>>> {
        (self.algorithm_factory)(self.graph).path(source, sink)
    }
}

/// Naive algorithm for many-to-many shortest paths problem using
/// `DijkstraClosestFirstIterator`.
///
/// Complexity of the algorithm is $O(min(|S|,|T|)*(V\log V + E))$, where $S$ is the set of source
/// vertices, $T$ is the set of target vertices, $V$ is the set of graph vertices and $E$ is the set
/// of graph edges of the graph.
///
/// For each source vertex a single source shortest paths search is performed, which is stopped as
/// soon as all target vertices are reached. Shortest paths trees are constructed using
/// `DijkstraClosestFirstIterator`. In case $|T| > |S|$ the searches are performed on the
/// reversed graph using $|T|$ as source vertices and $|S|$ as target vertices. This allows to reduce
/// the total number of searches from $|S|$ to $min(|S|,|T|)$.
///
/// The main bottleneck of this algorithm is the memory usage to store individual shortest paths
/// trees for every source vertex, as they may take a lot of space. Considering this, the typical use
/// case of this algorithm are small graphs or large graphs with small total number of source and
/// target vertices.
///
/// # Rust notes
///
/// Many-to-many Dijkstra using `min(|S|, |T|)` truncated shortest-path searches.
pub struct DijkstraManyToManyShortestPaths<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: IncomingGraph + OutgoingGraph> DijkstraManyToManyShortestPaths<'g, G> {
    /// Constructs an instance of the algorithm for a given `graph`.
    pub fn new(graph: &'g G) -> Self {
        Self { graph }
    }

    /// Computes shortest paths from all vertices in `sources` to all vertices in
    /// `targets`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn get_many_to_many_paths(
        &self,
        sources: impl IntoIterator<Item = G::Vertex>,
        targets: impl IntoIterator<Item = G::Vertex>,
    ) -> Result<ManyToManyPaths<G::Vertex, G::Edge>> {
        let sources: HashSet<_> = sources.into_iter().collect();
        let targets: HashSet<_> = targets.into_iter().collect();
        validate_sets(self.graph, &sources, &targets)?;
        let mut paths = HashMap::new();
        if sources.len() <= targets.len() {
            for source in &sources {
                let tree = forward_tree(self.graph, source, &targets)?;
                for target in &targets {
                    if let Some(path) = tree.path(target) {
                        paths.insert((source.clone(), target.clone()), path);
                    }
                }
            }
        } else {
            for target in &targets {
                let tree = reverse_tree(self.graph, target, &sources)?;
                for source in &sources {
                    if let Some(mut path) = tree.path(source) {
                        path.vertices.reverse();
                        path.edges.reverse();
                        paths.insert((source.clone(), target.clone()), path);
                    }
                }
            }
        }
        Ok(ManyToManyPaths::from_parts(sources, targets, paths))
    }

    /// Performs `single source`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn single_source(
        &self,
        source: &G::Vertex,
    ) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        let targets: HashSet<_> = self.graph.vertices().cloned().collect();
        forward_tree(self.graph, source, &targets)
    }
}

impl<G: IncomingGraph + OutgoingGraph> ShortestPathAlgorithm<G>
    for DijkstraManyToManyShortestPaths<'_, G>
{
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        DijkstraManyToManyShortestPaths::single_source(self, source)
    }
}

fn validate_sets<G: Graph>(
    graph: &G,
    sources: &HashSet<G::Vertex>,
    targets: &HashSet<G::Vertex>,
) -> Result<()> {
    if sources
        .iter()
        .chain(targets)
        .any(|v| !graph.contains_vertex(v))
    {
        return Err(GraphError::VertexNotFound);
    }
    Ok(())
}

fn forward_tree<G: OutgoingGraph>(
    graph: &G,
    source: &G::Vertex,
    targets: &HashSet<G::Vertex>,
) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
    let mut iterator = DijkstraClosestFirstIterator::new(graph, source)?;
    let mut remaining = targets.clone();
    while !remaining.is_empty() {
        let Some(vertex) = iterator.next() else { break };
        remaining.remove(&vertex);
    }
    Ok(iterator.paths())
}

fn reverse_tree<G: IncomingGraph>(
    graph: &G,
    target: &G::Vertex,
    sources: &HashSet<G::Vertex>,
) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
    let target = graph
        .vertices()
        .find(|vertex| *vertex == target)
        .ok_or(GraphError::VertexNotFound)?;
    let vertices: Vec<_> = graph.vertices().collect();
    let index: HashMap<_, _> = vertices.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    let mut dist = vec![f64::INFINITY; vertices.len()];
    let mut predecessor: Vec<Option<(&G::Edge, usize)>> = vec![None; vertices.len()];
    let mut settled = vec![false; vertices.len()];
    let mut heap = BinaryHeap::new();
    let root = index[target];
    dist[root] = 0.0;
    heap.push(Reverse((OrderedF64(0.0), root)));
    let mut remaining: HashSet<_> = sources.iter().collect();
    while let Some(Reverse((OrderedF64(distance), vertex))) = heap.pop() {
        if settled[vertex] || distance > dist[vertex] {
            continue;
        }
        settled[vertex] = true;
        remaining.remove(vertices[vertex]);
        if remaining.is_empty() {
            break;
        }
        for (edge, previous) in graph.incoming_of(vertices[vertex]) {
            let weight = graph.weight_of(edge);
            if weight.is_nan() || weight < 0.0 {
                return Err(GraphError::InvalidWeight(
                    crate::WeightProblem::NegativeOrNaN,
                ));
            }
            let previous_index = index[previous];
            let candidate = distance + weight;
            if candidate < dist[previous_index] {
                dist[previous_index] = candidate;
                predecessor[previous_index] = Some((edge, vertex));
                heap.push(Reverse((OrderedF64(candidate), previous_index)));
            }
        }
    }
    let mut result = SingleSourcePaths {
        source: target.clone(),
        dist: HashMap::new(),
        pred: HashMap::new(),
    };
    for (i, vertex) in vertices.iter().enumerate() {
        if dist[i].is_finite() {
            result.dist.insert((*vertex).clone(), dist[i]);
            if let Some((edge, parent)) = predecessor[i] {
                result
                    .pred
                    .insert((*vertex).clone(), (edge.clone(), vertices[parent].clone()));
            }
        }
    }
    Ok(result)
}
