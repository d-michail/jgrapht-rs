//! Cyclic-exchange local augmentation of Ahuja, Orlin and Sharma
//! (`AhujaOrlinSharmaCyclicExchangeLocalAugmentation`): finds a negative cost cycle of bounded
//! length in a labelled directed improvement graph that visits every label at most once.

use crate::graph::DirectedGraph;
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

pub struct AhujaOrlinSharmaCyclicExchangeLocalAugmentation<'g, G: DirectedGraph> {
    graph: &'g G,
    length_bound: usize,
    label_map: &'g HashMap<G::Vertex, i64>,
    best_improvement: bool,
}

impl<'g, G: DirectedGraph> AhujaOrlinSharmaCyclicExchangeLocalAugmentation<'g, G>
where
    G::Vertex: Eq + Hash,
{
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
        GraphWalk::new(vertices, edges, weight)
    }

    /// A negative cycle (first improving one, or the best one when `best_improvement`). If there
    /// is none the walk is empty with weight `f64::MAX`.
    pub fn local_augmentation_cycle(&self) -> GraphWalk<G::Vertex, G::Edge> {
        let g = self.graph;
        let label = |v: &G::Vertex| self.label_map[v];
        let weight = |e: &G::Edge| g.edge_weight(e).unwrap_or(1.0);
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
                for e in g.out_edges_of(tail).unwrap() {
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

fn update_path_index<V: Clone + Eq + Hash>(paths: &mut PathIndex<V>, path: LabeledPath<V>) {
    paths.insert(key_of(&path, &path.labels), path);
}
