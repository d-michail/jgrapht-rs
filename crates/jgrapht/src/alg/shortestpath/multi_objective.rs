use crate::alg::util::Indexed;
use crate::graph::Graph;
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct MultiObjectiveSingleSourcePaths<V, E> {
    source: V,
    paths: HashMap<V, Vec<GraphWalk<V, E>>>,
}

impl<V: Clone + Eq + std::hash::Hash, E: Clone> MultiObjectiveSingleSourcePaths<V, E> {
    pub fn source(&self) -> &V {
        &self.source
    }

    pub fn paths(&self, target: &V) -> &[GraphWalk<V, E>] {
        self.paths.get(target).map_or(&[], Vec::as_slice)
    }
}

struct Label {
    vertex: usize,
    value: Vec<f64>,
    previous: Option<usize>,
    edge: Option<usize>,
}

fn dominates(left: &[f64], right: &[f64]) -> bool {
    let mut strict = false;
    for (&a, &b) in left.iter().zip(right) {
        if a > b {
            return false;
        }
        strict |= a < b;
    }
    strict
}

fn close(left: &[f64], right: &[f64], epsilon: f64) -> bool {
    left.iter()
        .zip(right)
        .all(|(&a, &b)| (a - b).abs() <= a.abs() * epsilon)
}

fn validate_weights<G: Graph, F: Fn(&G::Edge) -> Vec<f64>>(
    indexed: &Indexed<'_, G>,
    weights: &F,
) -> Result<usize> {
    let mut dimensions = None;
    for edge in &indexed.es {
        let value = weights(edge);
        if dimensions.is_some_and(|d| d != value.len())
            || value.iter().any(|x| x.is_nan() || *x < 0.0)
        {
            return Err(GraphError::InvalidArgument(
                "invalid multi-objective edge weight function".into(),
            ));
        }
        dimensions.get_or_insert(value.len());
    }
    Ok(dimensions.unwrap_or(0))
}

fn build_paths<G: Graph>(
    indexed: &Indexed<'_, G>,
    source: usize,
    labels: &[Label],
    node_labels: &[Vec<usize>],
) -> MultiObjectiveSingleSourcePaths<G::Vertex, G::Edge> {
    let mut paths = HashMap::new();
    for sink in 0..indexed.n() {
        let mut sink_paths = Vec::new();
        if sink == source {
            sink_paths.push(GraphWalk::single(indexed.vertex(source)));
        } else {
            for &label_index in &node_labels[sink] {
                let mut edges = Vec::new();
                let mut vertices = Vec::new();
                let mut cursor = Some(label_index);
                while let Some(index) = cursor {
                    let label = &labels[index];
                    vertices.push(indexed.vertex(label.vertex));
                    if let Some(edge) = label.edge {
                        edges.push(edge);
                    }
                    cursor = label.previous;
                }
                vertices.reverse();
                edges.reverse();
                let weight = edges.iter().map(|&edge| indexed.w[edge]).sum();
                sink_paths.push(GraphWalk::new(
                    vertices,
                    edges.iter().map(|&edge| indexed.es[edge].clone()).collect(),
                    weight,
                ));
            }
        }
        paths.insert(indexed.vertex(sink), sink_paths);
    }
    MultiObjectiveSingleSourcePaths {
        source: indexed.vertex(source),
        paths,
    }
}

/// Martin's label-setting multi-objective shortest-path algorithm.
pub struct MartinShortestPath<'g, G: Graph, F> {
    graph: &'g G,
    edge_weight: F,
}

impl<'g, G: Graph, F: Fn(&G::Edge) -> Vec<f64>> MartinShortestPath<'g, G, F> {
    pub fn new(graph: &'g G, edge_weight: F) -> Self {
        Self { graph, edge_weight }
    }

    pub fn get_paths(
        &self,
        source: &G::Vertex,
    ) -> Result<MultiObjectiveSingleSourcePaths<G::Vertex, G::Edge>> {
        let indexed = Indexed::new(self.graph);
        let source = *indexed.idx.get(source).ok_or(GraphError::VertexNotFound)?;
        let dimensions = validate_weights(&indexed, &self.edge_weight)?;
        let mut labels = vec![Label {
            vertex: source,
            value: vec![0.0; dimensions],
            previous: None,
            edge: None,
        }];
        let mut node_labels = vec![Vec::new(); indexed.n()];
        node_labels[source].push(0);
        let mut heap = BinaryHeap::new();
        heap.push(Reverse((vec![OrderedF64(0.0); dimensions], 0usize)));
        while let Some(Reverse((_, label_index))) = heap.pop() {
            let vertex = labels[label_index].vertex;
            for &(target, edge) in &indexed.out[vertex] {
                let edge_value = (self.edge_weight)(indexed.es[edge]);
                let value: Vec<_> = labels[label_index]
                    .value
                    .iter()
                    .zip(edge_value)
                    .map(|(&a, b)| a + b)
                    .collect();
                if node_labels[target]
                    .iter()
                    .any(|&old| dominates(&labels[old].value, &value))
                {
                    continue;
                }
                node_labels[target].retain(|&old| !dominates(&value, &labels[old].value));
                let next = labels.len();
                labels.push(Label {
                    vertex: target,
                    value: value.clone(),
                    previous: Some(label_index),
                    edge: Some(edge),
                });
                node_labels[target].push(next);
                heap.push(Reverse((value.into_iter().map(OrderedF64).collect(), next)));
            }
        }
        Ok(build_paths(&indexed, source, &labels, &node_labels))
    }

    pub fn get_paths_between(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        if !self.graph.contains_vertex(sink) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(self.get_paths(source)?.paths(sink).to_vec())
    }
}

/// Label-correcting multi-objective shortest paths, with optional epsilon approximation.
pub struct LabelCorrectingMultiObjectiveShortestPath<'g, G: Graph, F> {
    graph: &'g G,
    edge_weight: F,
    epsilon: f64,
}

impl<'g, G: Graph, F: Fn(&G::Edge) -> Vec<f64>>
    LabelCorrectingMultiObjectiveShortestPath<'g, G, F>
{
    pub fn new(graph: &'g G, edge_weight: F) -> Self {
        Self {
            graph,
            edge_weight,
            epsilon: 0.0,
        }
    }

    pub fn with_epsilon(graph: &'g G, edge_weight: F, epsilon: f64) -> Result<Self> {
        if epsilon.is_nan() || epsilon < 0.0 {
            return Err(GraphError::InvalidArgument(
                "epsilon must be non-negative".into(),
            ));
        }
        Ok(Self {
            graph,
            edge_weight,
            epsilon,
        })
    }

    pub fn get_paths(
        &self,
        source: &G::Vertex,
    ) -> Result<MultiObjectiveSingleSourcePaths<G::Vertex, G::Edge>> {
        let indexed = Indexed::new(self.graph);
        let source = *indexed.idx.get(source).ok_or(GraphError::VertexNotFound)?;
        let dimensions = validate_weights(&indexed, &self.edge_weight)?;
        let mut labels = vec![Label {
            vertex: source,
            value: vec![0.0; dimensions],
            previous: None,
            edge: None,
        }];
        let mut node_labels = vec![Vec::new(); indexed.n()];
        node_labels[source].push(0);
        let mut queue = VecDeque::from([0usize]);
        while let Some(label_index) = queue.pop_front() {
            let vertex = labels[label_index].vertex;
            for &(target, edge) in &indexed.out[vertex] {
                let edge_value = (self.edge_weight)(indexed.es[edge]);
                let value: Vec<_> = labels[label_index]
                    .value
                    .iter()
                    .zip(edge_value)
                    .map(|(&a, b)| a + b)
                    .collect();
                if node_labels[target].iter().any(|&old| {
                    dominates(&labels[old].value, &value)
                        || (self.epsilon > 0.0 && close(&labels[old].value, &value, self.epsilon))
                }) {
                    continue;
                }
                node_labels[target].retain(|&old| !dominates(&value, &labels[old].value));
                let next = labels.len();
                labels.push(Label {
                    vertex: target,
                    value,
                    previous: Some(label_index),
                    edge: Some(edge),
                });
                node_labels[target].push(next);
                queue.push_back(next);
            }
        }
        Ok(build_paths(&indexed, source, &labels, &node_labels))
    }

    pub fn get_paths_between(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        if !self.graph.contains_vertex(sink) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(self.get_paths(source)?.paths(sink).to_vec())
    }
}
