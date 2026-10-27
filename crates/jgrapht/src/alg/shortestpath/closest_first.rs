use super::SingleSourcePaths;
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Lightweight closest-first Dijkstra iterator.
///
/// Vertices are yielded when their distance becomes final. The predecessor
/// tree also contains tentative vertices discovered inside the radius, just
/// like JGraphT's internal `DijkstraClosestFirstIterator`.
pub struct DijkstraClosestFirstIterator<'g, G: Graph> {
    graph: &'g G,
    source: &'g G::Vertex,
    radius: f64,
    distance: HashMap<&'g G::Vertex, f64>,
    predecessor: HashMap<&'g G::Vertex, (&'g G::Edge, &'g G::Vertex)>,
    settled: HashSet<&'g G::Vertex>,
    nodes: Vec<&'g G::Vertex>,
    heap: BinaryHeap<Reverse<(OrderedF64, usize)>>,
}

impl<'g, G: Graph> DijkstraClosestFirstIterator<'g, G> {
    pub fn new(graph: &'g G, source: &G::Vertex) -> Result<Self> {
        Self::with_radius(graph, source, f64::INFINITY)
    }

    pub fn with_radius(graph: &'g G, source: &G::Vertex, radius: f64) -> Result<Self> {
        if radius.is_nan() || radius < 0.0 {
            return Err(GraphError::InvalidArgument(
                "radius must be non-negative".into(),
            ));
        }
        let source = graph
            .vertices()
            .find(|vertex| *vertex == source)
            .ok_or(GraphError::VertexNotFound)?;
        // The Java iterator detects this while relaxing. Eager validation lets
        // Rust's `Iterator::next` stay infallible without changing semantics
        // for any successful iteration.
        if graph.edges().any(|edge| {
            let weight = graph.edge_weight(edge).unwrap_or(1.0);
            weight.is_nan() || weight < 0.0
        }) {
            return Err(GraphError::InvalidArgument(
                "negative or NaN edge weight".into(),
            ));
        }
        Ok(Self {
            graph,
            source,
            radius,
            distance: HashMap::from([(source, 0.0)]),
            predecessor: HashMap::new(),
            settled: HashSet::new(),
            nodes: vec![source],
            heap: BinaryHeap::from([Reverse((OrderedF64(0.0), 0))]),
        })
    }

    /// Return the current shortest-path tree. Paths to vertices which have
    /// not yet been yielded are tentative upper bounds, as in JGraphT.
    pub fn paths(&self) -> SingleSourcePaths<G::Vertex, G::Edge> {
        SingleSourcePaths {
            source: self.source.clone(),
            dist: self
                .distance
                .iter()
                .filter(|(_, distance)| **distance <= self.radius)
                .map(|(vertex, distance)| ((*vertex).clone(), *distance))
                .collect(),
            pred: self
                .predecessor
                .iter()
                .filter(|(vertex, _)| self.distance[*vertex] <= self.radius)
                .map(|(vertex, (edge, parent))| {
                    ((*vertex).clone(), ((*edge).clone(), (*parent).clone()))
                })
                .collect(),
        }
    }

    pub fn distance_and_predecessor_map(&self) -> HashMap<G::Vertex, (f64, Option<G::Edge>)> {
        self.distance
            .iter()
            .filter(|(_, distance)| **distance <= self.radius)
            .map(|(vertex, distance)| {
                (
                    (*vertex).clone(),
                    (
                        *distance,
                        self.predecessor
                            .get(vertex)
                            .map(|(edge, _)| (*edge).clone()),
                    ),
                )
            })
            .collect()
    }

    fn discard_stale(&mut self) {
        while let Some(Reverse((OrderedF64(distance), index))) = self.heap.peek().copied() {
            let vertex = self.nodes[index];
            if self.settled.contains(vertex) || distance > self.distance[vertex] {
                self.heap.pop();
            } else {
                break;
            }
        }
        if self
            .heap
            .peek()
            .is_some_and(|Reverse((OrderedF64(distance), _))| *distance > self.radius)
        {
            self.heap.clear();
        }
    }
}

impl<G: Graph> Iterator for DijkstraClosestFirstIterator<'_, G> {
    type Item = G::Vertex;

    fn next(&mut self) -> Option<Self::Item> {
        self.discard_stale();
        let Reverse((OrderedF64(distance), index)) = self.heap.pop()?;
        let vertex = self.nodes[index];
        self.settled.insert(vertex);

        for (edge, target) in self.graph.outgoing_of(vertex) {
            if self.settled.contains(target) {
                continue;
            }
            let candidate = distance + self.graph.edge_weight(edge).unwrap_or(1.0);
            if candidate < self.distance.get(target).copied().unwrap_or(f64::INFINITY) {
                self.distance.insert(target, candidate);
                self.predecessor.insert(target, (edge, vertex));
                self.nodes.push(target);
                self.heap
                    .push(Reverse((OrderedF64(candidate), self.nodes.len() - 1)));
            }
        }

        Some(vertex.clone())
    }
}
