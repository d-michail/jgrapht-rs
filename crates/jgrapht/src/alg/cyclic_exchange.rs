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

//! Cyclic-exchange local augmentation of Ahuja, Orlin and Sharma
//! (`AhujaOrlinSharmaCyclicExchangeLocalAugmentation`): finds a negative cost cycle of bounded
//! length in a labelled directed improvement graph that visits every label at most once.

use crate::graph::DirectedGraph;
use crate::graph::GraphExt;
use crate::graph::OutgoingGraph;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use indexmap::IndexMap;
use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;

#[derive(Clone)]
struct LabeledPath<V> {
    vertices: Vec<V>,
    labels: BTreeSet<i64>,
    cost: f64,
}

impl<V: Clone> LabeledPath<V> {
    fn add_vertex(&mut self, v: V, edge_cost: f64, label: i64) {
        self.vertices.push(v);
        self.cost += edge_cost;
        self.labels.insert(label);
    }
    fn head(&self) -> &V {
        &self.vertices[0]
    }
    fn tail(&self) -> &V {
        self.vertices.last().unwrap()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct PathSetKey<V> {
    head: V,
    tail: V,
    labels: BTreeSet<i64>,
}

type PathIndex<V> = IndexMap<PathSetKey<V>, LabeledPath<V>>;

/// Implementation of an algorithm for the local augmentation problem for the cyclic exchange
/// neighborhood, i.e. it finds subset-disjoint negative cycles in a graph, based on Ravindra K.
/// Ahuja, James B. Orlin, Dushyant Sharma, A composite very large-scale neighborhood structure for
/// the capacitated minimum spanning tree problem, Operations Research Letters, Volume 31, Issue 3,
/// 2003, Pages 185-194, ISSN 0167-6377, [https://doi.org/10.1016/S0167-6377(02)00236-5](https://doi.org/10.1016/S0167-6377(02)00236-5).
/// [(http://www.sciencedirect.com/science/article/pii/S0167637702002365)](http://www.sciencedirect.com/science/article/pii/S0167637702002365)
///
/// A subset-disjoint cycle is a cycle such that no two vertices in the cycle are in the same subset
/// of a given partition of the whole vertex set.
///
/// This algorithm returns the first or the best found negative subset-disjoint cycle. In the case of
/// the first found cycle, the cycle has minimum number of vertices. It may enumerate all paths up to
/// the length given by the parameter `lengthBound`, i.e the algorithm runs in exponential
/// time.
///
/// This algorithm is used to detect valid cyclic exchanges in a cyclic exchange neighborhood for the
/// Capacitated Minomum Spanning Tree problem
/// `org.jgrapht.alg.spanning.AhujaOrlinSharmaCapacitatedMinimumSpanningTree`
pub struct AhujaOrlinSharmaCyclicExchangeLocalAugmentation<'g, G: DirectedGraph> {
    graph: &'g G,
    length_bound: usize,
    label_map: &'g HashMap<G::Vertex, i64>,
    best_improvement: bool,
}

impl<'g, G: DirectedGraph + OutgoingGraph> AhujaOrlinSharmaCyclicExchangeLocalAugmentation<'g, G>
where
    G::Vertex: Eq + Hash,
{
    /// Constructs an algorithm with given inputs
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if an argument violates a precondition; errors of the underlying graph operations are propagated.
    pub fn new(
        graph: &'g G,
        length_bound: usize,
        label_map: &'g HashMap<G::Vertex, i64>,
        best_improvement: bool,
    ) -> Result<Self> {
        if graph.vertices().any(|v| !label_map.contains_key(v)) {
            return Err(GraphError::InvalidArgument(
                "Every vertex has to be labeled, that is, every vertex needs an entry in labelMap."
                    .into(),
            ));
        }
        Ok(AhujaOrlinSharmaCyclicExchangeLocalAugmentation {
            graph,
            length_bound,
            label_map,
            best_improvement,
        })
    }

    fn walk(&self, vertices: Vec<G::Vertex>, weight: f64) -> GraphWalk<G::Vertex, G::Edge> {
        let edges = vertices
            .windows(2)
            .filter_map(|w| self.graph.get_edge(&w[0], &w[1]).cloned())
            .collect();
        GraphWalk::new_unchecked(vertices, edges, weight)
    }

    /// A negative cycle (first improving one, or the best one when `best_improvement`). If there
    /// is none the walk is empty with weight `f64::MAX`.
    ///
    /// # Panics
    ///
    /// Panics only if the graph breaks the integrity contract of [`Graph`](crate::Graph) (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
    pub fn local_augmentation_cycle(&self) -> GraphWalk<G::Vertex, G::Edge> {
        let g = self.graph;
        let label = |v: &G::Vertex| self.label_map[v];
        let weight = |e: &G::Edge| g.weight_of(e);
        let mut k = 1;
        let mut best_cycle = LabeledPath::<G::Vertex> {
            vertices: Vec::new(),
            cost: f64::MAX,
            labels: BTreeSet::new(),
        };
        let mut paths_k: PathIndex<G::Vertex> = IndexMap::new();
        let mut paths_k1: PathIndex<G::Vertex> = IndexMap::new();

        for e in g.edges() {
            if weight(e) < 0.0 {
                let source = g.edge_source(e).unwrap();
                let target = g.edge_target(e).unwrap();
                // self-loops
                if source == target {
                    let vertices = vec![source.clone(), target.clone()];
                    let current = weight(e);
                    let opposite = weight(g.get_edge(target, source).unwrap());
                    if self.best_improvement {
                        if best_cycle.cost > current + opposite {
                            best_cycle = LabeledPath {
                                vertices,
                                cost: current + opposite,
                                labels: BTreeSet::from([label(source)]),
                            };
                        }
                    } else {
                        return self.walk(vertices, current + opposite);
                    }
                }
                if label(source) != label(target) {
                    let path = LabeledPath {
                        vertices: vec![source.clone(), target.clone()],
                        cost: weight(e),
                        labels: BTreeSet::from([label(source), label(target)]),
                    };
                    update_path_index(&mut paths_k, path);
                }
            }
        }

        while k < self.length_bound {
            for path in paths_k.values() {
                let head = path.head();
                let tail = path.tail();
                if let Some(closing) = g.get_edge(tail, head) {
                    let current_cost = path.cost + weight(closing);
                    if current_cost < best_cycle.cost {
                        let mut cycle = path.clone();
                        cycle.add_vertex(head.clone(), weight(closing), label(head));
                        if !self.best_improvement && current_cost < 0.0 {
                            return self.walk(cycle.vertices, cycle.cost);
                        }
                        best_cycle = cycle;
                    }
                }
                for e in g.out_edges_of(tail) {
                    let current_vertex = g.edge_target(e).unwrap();
                    let edge_weight = weight(e);
                    let current_label = label(current_vertex);
                    if !path.labels.contains(&current_label) && path.cost + edge_weight < 0.0 {
                        let mut new_path = path.clone();
                        new_path.add_vertex(current_vertex.clone(), edge_weight, current_label);
                        if !dominated_k_plus_1(&new_path, &paths_k1)
                            && !dominated_k(&new_path, &paths_k)
                        {
                            update_path_index(&mut paths_k1, new_path);
                        }
                    }
                }
            }
            k += 1;
            paths_k = std::mem::take(&mut paths_k1);
        }
        self.walk(best_cycle.vertices, best_cycle.cost)
    }
}

fn key_of<V: Clone>(path: &LabeledPath<V>, labels: &BTreeSet<i64>) -> PathSetKey<V> {
    PathSetKey {
        head: path.head().clone(),
        tail: path.tail().clone(),
        labels: labels.clone(),
    }
}

fn dominated_k_plus_1<V: Clone + Eq + Hash>(path: &LabeledPath<V>, k1: &PathIndex<V>) -> bool {
    k1.get(&key_of(path, &path.labels))
        .is_some_and(|other| other.cost < path.cost)
}

fn dominated_k<V: Clone + Eq + Hash>(path: &LabeledPath<V>, k: &PathIndex<V>) -> bool {
    let mut labels = path.labels.clone();
    for &label in &path.labels {
        labels.remove(&label);
        if k.get(&key_of(path, &labels))
            .is_some_and(|other| other.cost < path.cost)
        {
            return true;
        }
        labels.insert(label);
    }
    false
}

/// Adds a path and removes the path, which has the same tail, head and label set, to the data
/// structure `paths`, which contains all paths indexed by their head, tail and label set.
fn update_path_index<V: Clone + Eq + Hash>(paths: &mut PathIndex<V>, path: LabeledPath<V>) {
    paths.insert(key_of(&path, &path.labels), path);
}
