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

use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::IncidenceGraph;
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::util::heap::JavaPairingHeap;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use rheaps::{AddressableHeap, DecreaseKeyHeap};
use std::borrow::Borrow;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

fn materialize_single_source<G, F>(
    graph: &G,
    source: &G::Vertex,
    mut path_to: F,
) -> Result<SingleSourcePaths<G::Vertex, G::Edge>>
where
    G: Graph,
    F: FnMut(&G::Vertex) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>>,
{
    if !graph.contains_vertex(source) {
        return Err(GraphError::VertexNotFound);
    }
    let mut paths = SingleSourcePaths {
        source: source.clone(),
        dist: HashMap::new(),
        pred: HashMap::new(),
    };
    for target in graph.vertices() {
        let target = target.borrow();
        let Some(path) = path_to(target)? else {
            continue;
        };
        paths.dist.insert(target.clone(), path.weight);
        if let (Some(edge), Some(parent)) = (
            path.edges.last(),
            path.vertices.get(path.vertices.len().saturating_sub(2)),
        ) {
            paths
                .pred
                .insert(target.clone(), (edge.clone(), parent.clone()));
        }
    }
    Ok(paths)
}

/// Bidirectional Dijkstra: a forward search from the source and a backward search (over
/// incoming edges) from the sink, stopped when the frontiers can no longer improve the best
/// meeting point. Requires non-negative weights.
pub struct BidirectionalDijkstra<'g, G>(pub &'g G);

/// A bidirectional version of Dijkstra's algorithm.
///
/// See the Wikipedia article for details and references about
/// [bidirectional search](https://en.wikipedia.org/wiki/Bidirectional_search). This
/// technique does not change the worst-case behavior of the algorithm but reduces, in some cases,
/// the number of visited vertices in practice. This implementation alternatively constructs forward
/// and reverse paths from the source and target vertices respectively.
///
/// This iterator can use a custom heap implementation, which can specified during the construction
/// time. Pairing heap is used by default
///
/// # Rust notes
///
/// Java-style bidirectional Dijkstra class, including its optional search
/// radius.
///
/// The backward search needs incoming edges, so a graph without an incoming index is rejected at
/// compile time:
///
/// ```compile_fail,E0277
/// use jgrapht::alg::shortestpath::{BidirectionalDijkstraShortestPath, ShortestPathAlgorithm};
/// use jgrapht::graph::sparse::{NoIncoming, SparseIntDirectedGraph};
///
/// let g = SparseIntDirectedGraph::<NoIncoming>::from_edges(2, &[(0, 1)]).unwrap();
/// let _ = BidirectionalDijkstraShortestPath::new(&g).path_weight(&0, &1);
/// ```
pub struct BidirectionalDijkstraShortestPath<'g, G> {
    graph: &'g G,
    radius: f64,
}

impl<'g, G: IncidenceGraph> BidirectionalDijkstraShortestPath<'g, G> {
    /// Constructs a new instance for a specified graph.
    pub fn new(graph: &'g G) -> Self {
        Self {
            graph,
            radius: f64::INFINITY,
        }
    }

    /// Creates an instance with the given radius.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if a numeric or collection parameter is out of range; errors of the underlying graph operations are propagated.
    pub fn with_radius(graph: &'g G, radius: f64) -> Result<Self> {
        if radius.is_nan() || radius < 0.0 {
            return Err(GraphError::must_be_non_negative("radius"));
        }
        Ok(Self { graph, radius })
    }
}

impl<G: IncidenceGraph> ShortestPathAlgorithm<G> for BidirectionalDijkstraShortestPath<'_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        materialize_single_source(self.graph, source, |target| self.path(source, target))
    }

    /// Get a shortest path from a source vertex to a sink vertex.
    fn path(
        &self,
        source: &G::Vertex,
        target: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        Ok(BidirectionalDijkstra(self.graph)
            .path(source, target)?
            .filter(|path| path.weight <= self.radius))
    }
}

/// A bidirectional version of A* algorithm.
///
/// See the Wikipedia article for details and references about
/// [bidirectional search](https://en.wikipedia.org/wiki/Bidirectional_search). This
/// technique does not change the worst-case behavior of the algorithm but reduces, in some cases,
/// the number of visited vertices in practice.
///
/// The algorithm was first introduced in Ira Sheldon Pohl. 1969. Bi-Directional and Heuristic Search
/// in Path Problems. Ph.D. Dissertation. Stanford University, Stanford, CA, USA. AAI7001588.
///
/// The implementation uses two termination criteria depending on if the provided heuristic is
/// consistent or not. Both criteria are based on the shortest path distance $\mu$ seen thus far in
/// the search. Initially, in both cases the algorithm sets $\mu=\infty$. Whenever the search updates
/// the information about the vertex $v$, it sets $\mu = min\{\mu; g_f(v) + g_b(v)\}$, where $g_f(v)$
/// is the current best-known path cost from $source$ to $sink$ and $g_b(v)$ is the current
/// best-known path cost from $sink$ to $source$.
///
/// # Rust notes
///
/// Bidirectional A* point-to-point search.
///
/// Both frontiers re-open closed vertices when the heuristic is inconsistent.
/// The conservative inconsistent-heuristic stopping rule is used, so any
/// admissible heuristic is accepted.
pub struct BidirectionalAStarShortestPath<'g, G, F> {
    graph: &'g G,
    heuristic: F,
}

impl<'g, G: IncidenceGraph, F: Fn(&G::Vertex, &G::Vertex) -> f64>
    BidirectionalAStarShortestPath<'g, G, F>
{
    /// Creates a new instance.
    pub fn new(graph: &'g G, heuristic: F) -> Self {
        Self { graph, heuristic }
    }
}

impl<G: IncidenceGraph, F: Fn(&G::Vertex, &G::Vertex) -> f64> ShortestPathAlgorithm<G>
    for BidirectionalAStarShortestPath<'_, G, F>
{
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        materialize_single_source(self.graph, source, |target| self.path(source, target))
    }

    fn path(
        &self,
        source: &G::Vertex,
        target: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        bidirectional_astar(self.graph, source, target, &self.heuristic)
    }
}

struct AStarSide<G: Graph> {
    score: HashMap<G::Vertex, f64>,
    key: HashMap<G::Vertex, f64>,
    pred: HashMap<G::Vertex, (G::Edge, G::Vertex)>,
    open: HashSet<G::Vertex>,
    closed: HashSet<G::Vertex>,
    nodes: Vec<G::Vertex>,
    heap: BinaryHeap<Reverse<(OrderedF64, usize)>>,
}

impl<G: IncidenceGraph> AStarSide<G> {
    fn new(start: &G::Vertex, estimate: f64) -> Self {
        Self {
            score: HashMap::from([(start.clone(), 0.0)]),
            key: HashMap::from([(start.clone(), estimate)]),
            pred: HashMap::new(),
            open: HashSet::from([start.clone()]),
            closed: HashSet::new(),
            nodes: vec![start.clone()],
            heap: BinaryHeap::from([Reverse((OrderedF64(estimate), 0))]),
        }
    }

    fn clean(&mut self) {
        while let Some(Reverse((OrderedF64(key), index))) = self.heap.peek().copied() {
            let vertex = &self.nodes[index];
            if !self.open.contains(vertex) || key > self.key[vertex] {
                self.heap.pop();
            } else {
                break;
            }
        }
    }

    fn min_key(&mut self) -> f64 {
        self.clean();
        self.heap
            .peek()
            .map_or(f64::INFINITY, |Reverse((OrderedF64(key), _))| *key)
    }

    fn pop(&mut self) -> Option<(G::Vertex, f64)> {
        self.clean();
        let Reverse((_, index)) = self.heap.pop()?;
        let vertex = self.nodes[index].clone();
        self.open.remove(&vertex);
        self.closed.insert(vertex.clone());
        let score = self.score[&vertex];
        Some((vertex, score))
    }

    fn update(
        &mut self,
        vertex: &G::Vertex,
        parent: &G::Vertex,
        edge: &G::Edge,
        score: f64,
        key: f64,
    ) {
        if score >= self.score.get(vertex).copied().unwrap_or(f64::INFINITY) {
            return;
        }
        self.score.insert(vertex.clone(), score);
        self.key.insert(vertex.clone(), key);
        self.pred
            .insert(vertex.clone(), (edge.clone(), parent.clone()));
        self.closed.remove(vertex);
        self.open.insert(vertex.clone());
        self.nodes.push(vertex.clone());
        self.heap
            .push(Reverse((OrderedF64(key), self.nodes.len() - 1)));
    }
}

fn bidirectional_astar<G, F>(
    graph: &G,
    source: &G::Vertex,
    target: &G::Vertex,
    heuristic: &F,
) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>>
where
    G: IncidenceGraph,
    F: Fn(&G::Vertex, &G::Vertex) -> f64,
{
    if !graph.contains_vertex(source) || !graph.contains_vertex(target) {
        return Err(GraphError::VertexNotFound);
    }
    if source == target {
        return Ok(Some(GraphWalk::single(source.clone())));
    }

    let mut forward = AStarSide::<G>::new(source, heuristic(source, target));
    let mut backward = AStarSide::<G>::new(target, heuristic(source, target));
    let mut scan_forward = true;
    let mut best = f64::INFINITY;
    let mut meeting: Option<G::Vertex> = None;

    loop {
        let forward_key = forward.min_key();
        let backward_key = backward.min_key();
        if forward_key.is_infinite()
            || backward_key.is_infinite()
            || forward_key.max(backward_key) >= best
        {
            break;
        }

        let (side, other) = if scan_forward {
            (&mut forward, &backward)
        } else {
            (&mut backward, &forward)
        };
        let Some((vertex, vertex_score)) = side.pop() else {
            break;
        };
        let adjacent: Vec<(G::Edge, G::Vertex)> = if scan_forward {
            graph
                .outgoing_of(&vertex)
                .map(|(e, v)| (e.clone(), v.clone()))
                .collect()
        } else {
            graph
                .incoming_of(&vertex)
                .map(|(e, v)| (e.clone(), v.clone()))
                .collect()
        };
        for (edge, successor) in adjacent {
            if successor == vertex {
                continue;
            }
            let edge_weight = graph.weight_of(&edge);
            if edge_weight.is_nan() || edge_weight < 0.0 {
                return Err(GraphError::InvalidWeight(
                    crate::WeightProblem::NegativeOrNaN,
                ));
            }
            let candidate = vertex_score + edge_weight;
            let estimate = if scan_forward {
                heuristic(&successor, target)
            } else {
                heuristic(source, &successor)
            };
            side.update(&successor, &vertex, &edge, candidate, candidate + estimate);

            let joined = vertex_score
                + edge_weight
                + other
                    .score
                    .get(&successor)
                    .copied()
                    .unwrap_or(f64::INFINITY);
            if joined < best {
                best = joined;
                meeting = Some(successor);
            }
        }
        scan_forward = if scan_forward {
            forward.open.len() <= backward.open.len()
        } else {
            !(backward.open.len() <= forward.open.len())
        };
    }

    let Some(meeting) = meeting else {
        return Ok(None);
    };
    let mut vertices = vec![meeting.clone()];
    let mut edges = Vec::new();
    let mut current = meeting.clone();
    while let Some((edge, parent)) = forward.pred.get(&current) {
        edges.push(edge.clone());
        vertices.push(parent.clone());
        current = parent.clone();
    }
    vertices.reverse();
    edges.reverse();
    current = meeting;
    while let Some((edge, parent)) = backward.pred.get(&current) {
        edges.push(edge.clone());
        vertices.push(parent.clone());
        current = parent.clone();
    }
    Ok(Some(GraphWalk {
        vertices,
        edges,
        weight: best,
    }))
}

type JavaHandle =
    <JavaPairingHeap<OrderedF64, usize> as AddressableHeap<OrderedF64, usize>>::Handle;

/// One search frontier of Java's `DijkstraSearchFrontier`: a pairing heap and the seen vertices
/// with their current distance and tree edge.
struct Side<G: Graph> {
    heap: JavaPairingHeap<OrderedF64, usize>,
    nodes: Vec<G::Vertex>,
    index: HashMap<G::Vertex, usize>,
    seen: Vec<(f64, JavaHandle, Option<(G::Edge, G::Vertex)>)>,
}

impl<G: IncidenceGraph> Side<G> {
    fn new(start: &G::Vertex) -> Self {
        let mut side = Side {
            heap: JavaPairingHeap::default(),
            nodes: Vec::new(),
            index: HashMap::new(),
            seen: Vec::new(),
        };
        side.update_distance(start, None, 0.0);
        side
    }

    fn update_distance(
        &mut self,
        v: &G::Vertex,
        tree: Option<(G::Edge, G::Vertex)>,
        distance: f64,
    ) {
        match self.index.get(v) {
            None => {
                let id = self.nodes.len();
                let handle = self.heap.insert(OrderedF64(distance), id);
                self.nodes.push(v.clone());
                self.index.insert(v.clone(), id);
                self.seen.push((distance, handle, tree));
            }
            Some(&id) if distance < self.seen[id].0 => {
                self.seen[id].0 = distance;
                self.seen[id].2 = tree;
                let _ = self
                    .heap
                    .decrease_key(self.seen[id].1, OrderedF64(distance));
            }
            Some(_) => {}
        }
    }

    fn distance(&self, v: &G::Vertex) -> f64 {
        self.index
            .get(v)
            .map_or(f64::INFINITY, |&id| self.seen[id].0)
    }

    fn min_key(&self) -> Option<f64> {
        self.heap.peek().map(|(_, key, _)| key.0)
    }

    fn tree_edge(&self, v: &G::Vertex) -> Option<(G::Edge, G::Vertex)> {
        self.index.get(v).and_then(|&id| self.seen[id].2.clone())
    }
}

impl<G: IncidenceGraph> BidirectionalDijkstra<'_, G> {
    /// Java's bidirectional Dijkstra: the frontiers are scanned alternately, the best path is
    /// updated whenever an edge reaches a vertex seen by the other frontier, and the search stops
    /// when the sum of the two minimum keys reaches the best path.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if a vertex argument is not in the graph; an edge weight is negative, NaN or otherwise unusable; errors of the underlying graph operations are propagated.
    ///
    /// # Panics
    ///
    /// Panics only if the graph breaks the integrity contract of [`Graph`] (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
    pub fn path(
        &self,
        s: &G::Vertex,
        t: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        let g = self.0;
        if !g.contains_vertex(s) || !g.contains_vertex(t) {
            return Err(GraphError::VertexNotFound);
        }
        if s == t {
            return Ok(Some(GraphWalk::single(s.clone())));
        }
        let mut sides = [Side::<G>::new(s), Side::<G>::new(t)];
        let mut best_path = f64::INFINITY;
        let mut common: Option<G::Vertex> = None;
        let mut current = 0;
        while let (Some(min), Some(other_min)) =
            (sides[current].min_key(), sides[1 - current].min_key())
        {
            if min + other_min >= best_path {
                break;
            }
            // frontier scan
            let (OrderedF64(distance), id) = sides[current].heap.pop().expect("non-empty heap");
            let v = sides[current].nodes[id].clone();
            let edges: Vec<(G::Edge, G::Vertex)> = if current == 0 {
                g.outgoing_of(&v)
                    .map(|(e, u)| (e.clone(), u.clone()))
                    .collect()
            } else {
                g.incoming_of(&v)
                    .map(|(e, u)| (e.clone(), u.clone()))
                    .collect()
            };
            for (e, u) in edges {
                let weight = g.weight_of(&e);
                if weight.is_nan() || weight < 0.0 {
                    return Err(GraphError::InvalidWeight(
                        crate::WeightProblem::NegativeOrNaN,
                    ));
                }
                sides[current].update_distance(&u, Some((e, v.clone())), distance + weight);
                // check path with u's distance from the other frontier
                let path_distance = distance + weight + sides[1 - current].distance(&u);
                if path_distance < best_path {
                    best_path = path_distance;
                    common = Some(u);
                }
            }
            // swap frontiers
            current = 1 - current;
        }
        let Some(common) = common.filter(|_| best_path.is_finite()) else {
            return Ok(None);
        };
        let mut vertices = VecDeque::from([common.clone()]);
        let mut edges = VecDeque::new();
        let mut v = common.clone();
        while let Some((e, parent)) = sides[0].tree_edge(&v) {
            edges.push_front(e);
            vertices.push_front(parent.clone());
            v = parent;
        }
        v = common;
        while let Some((e, parent)) = sides[1].tree_edge(&v) {
            edges.push_back(e);
            vertices.push_back(parent.clone());
            v = parent;
        }
        Ok(Some(GraphWalk {
            vertices: vertices.into(),
            edges: edges.into(),
            weight: best_path,
        }))
    }
}
