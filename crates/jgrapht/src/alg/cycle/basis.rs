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

use super::detector::simple_cycle_to_graph_path;
use crate::error::{GraphError, Result};
use crate::graph::props::{has_multiple_edges, require_undirected};
use crate::graph::{Graph, GraphExt};
use crate::walk::GraphWalk;
use indexmap::{IndexMap, IndexSet};
use std::collections::{HashMap, HashSet, VecDeque};

/// `CycleBasisAlgorithm.CycleBasis`: a set of cycles (as edge lists) with their total length and
/// weight.
#[derive(Debug, Clone)]
pub struct CycleBasis<E> {
    cycles: IndexSet<Vec<E>>,
    length: usize,
    weight: f64,
}

impl<E: Clone + Eq + std::hash::Hash> CycleBasis<E> {
    pub fn new(cycles: IndexSet<Vec<E>>, length: usize, weight: f64) -> Self {
        CycleBasis {
            cycles,
            length,
            weight,
        }
    }

    pub fn empty() -> Self {
        Self::new(IndexSet::new(), 0, 0.0)
    }

    pub fn cycles(&self) -> &IndexSet<Vec<E>> {
        &self.cycles
    }

    /// The total number of edges of all cycles.
    pub fn length(&self) -> usize {
        self.length
    }

    /// The total weight of all cycles.
    pub fn weight(&self) -> f64 {
        self.weight
    }

    /// The cycles as closed walks of `graph`.
    pub fn cycles_as_graph_paths<G: Graph<Edge = E>>(
        &self,
        graph: &G,
    ) -> Result<Vec<GraphWalk<G::Vertex, E>>> {
        let mut paths = Vec::new();
        for cycle in &self.cycles {
            if let Some(path) = simple_cycle_to_graph_path(graph, cycle)? {
                paths.push(path);
            }
        }
        Ok(paths)
    }
}

/// Allows to derive an undirected [cycle basis](https://en.wikipedia.org/wiki/Cycle_basis) of a given graph.
///
/// Note that undirected cycle bases are defined for both undirected and directed graphs. For a
/// discussion of different kinds of cycle bases in graphs see the following paper.
///
/// - Christian Liebchen, and Romeo Rizzi. Classes of Cycle Bases. Discrete Applied Mathematics,
/// 155(3), 337-355, 2007.
///
/// # Rust notes
///
/// `CycleBasisAlgorithm`.
pub trait CycleBasisAlgorithm<E> {
    fn get_cycle_basis(&mut self) -> Result<CycleBasis<E>>;
}

fn require_directed_or_undirected<G: Graph>(graph: &G) -> Result<()> {
    if graph.graph_type().mixed {
        return Err(GraphError::InvalidArgument(
            "Graph must be directed or undirected".into(),
        ));
    }
    Ok(())
}

/// `AbstractFundamentalCycleBasis`: a spanning forest (edge to the parent of every vertex) and the
/// fundamental cycle of every non-tree edge.
fn fundamental_cycle_basis<G: Graph>(
    graph: &G,
    spanning_forest: &HashMap<G::Vertex, Option<G::Edge>>,
) -> CycleBasis<G::Edge> {
    let tree_edges: HashSet<&G::Edge> = spanning_forest.values().flatten().collect();
    let mut cycles: IndexSet<Vec<G::Edge>> = IndexSet::new();
    let mut length = 0;
    let mut weight = 0.0;
    for e in graph.edges() {
        if !tree_edges.contains(e) {
            let (cycle, w) = build_fundamental_cycle(graph, e, spanning_forest);
            length += cycle.len();
            weight += w;
            cycles.insert(cycle);
        }
    }
    CycleBasis::new(cycles, length, weight)
}

fn build_fundamental_cycle<G: Graph>(
    graph: &G,
    e: &G::Edge,
    spanning_forest: &HashMap<G::Vertex, Option<G::Edge>>,
) -> (Vec<G::Edge>, f64) {
    let (source, target) = graph.edge_endpoints(e).expect("edge of the graph");
    let weight_of = |e: &G::Edge| graph.edge_weight(e).unwrap_or(1.0);
    if source == target {
        return (vec![e.clone()], weight_of(e));
    }
    // the path from the source up to the target (or to the root of its tree)
    let mut path1: IndexSet<G::Edge> = IndexSet::new();
    path1.insert(e.clone());
    let mut cur = source;
    while cur != target {
        let Some(Some(edge_to_parent)) = spanning_forest.get(cur) else {
            break;
        };
        let parent = graph
            .opposite_vertex(edge_to_parent, cur)
            .expect("tree edge");
        path1.insert(edge_to_parent.clone());
        cur = parent;
    }
    // the path from the target up to the common ancestor
    let mut path2_weight = 0.0;
    let mut path2: VecDeque<G::Edge> = VecDeque::new();
    if cur != target {
        cur = target;
        while let Some(Some(edge_to_parent)) = spanning_forest.get(cur) {
            let parent = graph
                .opposite_vertex(edge_to_parent, cur)
                .expect("tree edge");
            if path1.contains(edge_to_parent) {
                path1.shift_remove(edge_to_parent);
            } else {
                path2.push_back(edge_to_parent.clone());
                path2_weight += weight_of(edge_to_parent);
            }
            cur = parent;
        }
    }
    for a in path1 {
        path2_weight += weight_of(&a);
        path2.push_front(a);
    }
    (path2.into_iter().collect(), path2_weight)
}

macro_rules! fundamental_basis {
    ($(#[$doc:meta])* $name:ident, $push:ident, $pop:ident) => {
        $(#[$doc])*
        pub struct $name<'g, G: Graph> {
            graph: &'g G,
        }

        impl<'g, G: Graph> $name<'g, G> {
            /// Fails for mixed graphs.
            pub fn new(graph: &'g G) -> Result<Self> {
                require_directed_or_undirected(graph)?;
                Ok($name { graph })
            }

            fn compute_spanning_forest(&self) -> Result<HashMap<G::Vertex, Option<G::Edge>>> {
                let graph = self.graph;
                let mut pred: HashMap<G::Vertex, Option<G::Edge>> = HashMap::new();
                let mut queue: VecDeque<G::Vertex> = VecDeque::new();
                for s in graph.vertices() {
                    if pred.contains_key(s) {
                        continue;
                    }
                    pred.insert(s.clone(), None);
                    queue.$push(s.clone());
                    while let Some(v) = queue.$pop() {
                        for e in graph.edges_of(&v)? {
                            let u = graph.opposite_vertex(e, &v).expect("edge of the graph");
                            if !pred.contains_key(u) {
                                pred.insert(u.clone(), Some(e.clone()));
                                queue.$push(u.clone());
                            }
                        }
                    }
                }
                Ok(pred)
            }
        }

        impl<'g, G: Graph> CycleBasisAlgorithm<G::Edge> for $name<'g, G> {
            fn get_cycle_basis(&mut self) -> Result<CycleBasis<G::Edge>> {
                let forest = self.compute_spanning_forest()?;
                Ok(fundamental_cycle_basis(self.graph, &forest))
            }
        }
    };
}

fundamental_basis!(
    /// `QueueBFSFundamentalCycleBasis`: the fundamental cycles of a breadth-first spanning forest.
    QueueBFSFundamentalCycleBasis,
    push_back,
    pop_front
);
fundamental_basis!(
    /// `StackBFSFundamentalCycleBasis`: the fundamental cycles of the spanning forest of a
    /// stack-driven search.
    StackBFSFundamentalCycleBasis,
    push_back,
    pop_back
);

/// Find a cycle basis of an undirected graph using a variant of Paton's algorithm.
///
/// See:
/// K. Paton, An algorithm for finding a fundamental set of cycles for an undirected linear graph,
/// Comm. ACM 12 (1969), pp. 514-518.
///
/// Note that Paton's algorithm produces a fundamental cycle basis while this implementation produces
/// a [weakly fundamental cycle basis](https://en.wikipedia.org/wiki/Cycle_space#Fundamental_and_weakly_fundamental_bases). A cycle basis is called weakly fundamental if there exists a linear
/// ordering of the cycles in a cycle basis such that each cycle includes at least one edge that is
/// not part of any previous cycle. Every fundamental cycle basis is weakly fundamental (for all
/// linear orderings) but not necessarily vice versa.
///
/// # Rust notes
///
/// `PatonCycleBase`: Paton's 1969 algorithm for a cycle basis of an undirected graph without
/// parallel edges.
pub struct PatonCycleBase<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> PatonCycleBase<'g, G> {
    /// Fails for directed graphs.
    pub fn new(graph: &'g G) -> Result<Self> {
        require_undirected(graph)?;
        Ok(PatonCycleBase { graph })
    }
}

impl<'g, G: Graph> CycleBasisAlgorithm<G::Edge> for PatonCycleBase<'g, G> {
    fn get_cycle_basis(&mut self) -> Result<CycleBasis<G::Edge>> {
        let graph = self.graph;
        if has_multiple_edges(graph) {
            return Err(GraphError::InvalidArgument(
                "Graphs with multiple edges not supported".into(),
            ));
        }
        let weight_of = |e: &G::Edge| graph.edge_weight(e).unwrap_or(1.0);
        let mut used: HashMap<G::Vertex, IndexMap<G::Vertex, G::Edge>> = HashMap::new();
        let mut parent: HashMap<G::Vertex, Option<G::Edge>> = HashMap::new();
        let mut stack: Vec<G::Vertex> = Vec::new();
        let mut cycles: IndexSet<Vec<G::Edge>> = IndexSet::new();
        let mut total_length = 0;
        let mut total_weight = 0.0;
        for root in graph.vertices() {
            if parent.contains_key(root) {
                continue;
            }
            used.clear();
            parent.insert(root.clone(), None);
            used.insert(root.clone(), IndexMap::new());
            stack.push(root.clone());
            while let Some(current) = stack.pop() {
                for e in graph.edges_of(&current)? {
                    let neighbor = graph
                        .opposite_vertex(e, &current)
                        .expect("edge of the graph");
                    if !used.contains_key(neighbor) {
                        parent.insert(neighbor.clone(), Some(e.clone()));
                        let mut neighbour_used = IndexMap::new();
                        neighbour_used.insert(current.clone(), e.clone());
                        used.insert(neighbor.clone(), neighbour_used);
                        stack.push(neighbor.clone());
                    } else if *neighbor == current {
                        total_weight += weight_of(e);
                        total_length += 1;
                        cycles.insert(vec![e.clone()]);
                    } else if !used[&current].contains_key(neighbor) {
                        let mut weight = weight_of(e);
                        let mut cycle = vec![e.clone()];
                        let mut v = current.clone();
                        while !used[neighbor].contains_key(&v) {
                            let p = parent[&v].clone().expect("vertex below the root");
                            weight += weight_of(&p);
                            v = graph.opposite_vertex(&p, &v).expect("tree edge").clone();
                            cycle.push(p);
                        }
                        let a = used[neighbor][&v].clone();
                        weight += weight_of(&a);
                        cycle.push(a);
                        used.get_mut(neighbor)
                            .expect("seen vertex")
                            .insert(current.clone(), e.clone());
                        total_length += cycle.len();
                        total_weight += weight;
                        cycles.insert(cycle);
                    }
                }
            }
        }
        Ok(CycleBasis::new(cycles, total_length, total_weight))
    }
}
