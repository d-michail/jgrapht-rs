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
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Naive many-to-many implementation, running a point-to-point algorithm for every pair.
pub struct DefaultManyToManyShortestPaths<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> DefaultManyToManyShortestPaths<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self { graph }
    }

    pub fn get_many_to_many_paths(
        &self,
        sources: impl IntoIterator<Item = G::Vertex>,
        targets: impl IntoIterator<Item = G::Vertex>,
    ) -> Result<ManyToManyPaths<G::Vertex, G::Edge>> {
        let sources: HashSet<_> = sources.into_iter().collect();
        let targets: HashSet<_> = targets.into_iter().collect();
        validate_sets(self.graph, &sources, &targets)?;
        let algorithm = BidirectionalDijkstraShortestPath::new(self.graph);
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

/// Many-to-many Dijkstra using `min(|S|, |T|)` truncated shortest-path searches.
pub struct DijkstraManyToManyShortestPaths<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> DijkstraManyToManyShortestPaths<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self { graph }
    }

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

    pub fn single_source(
        &self,
        source: &G::Vertex,
    ) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        let targets: HashSet<_> = self.graph.vertices().cloned().collect();
        forward_tree(self.graph, source, &targets)
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

fn forward_tree<G: Graph>(
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

fn reverse_tree<G: Graph>(
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
        for (edge, previous) in graph.try_incoming_of(vertices[vertex])? {
            let weight = graph.edge_weight(edge).unwrap_or(1.0);
            if weight.is_nan() || weight < 0.0 {
                return Err(GraphError::InvalidArgument(
                    "negative or NaN edge weight".into(),
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
