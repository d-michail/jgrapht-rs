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
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

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

/// Java-style bidirectional Dijkstra class, including its optional search
/// radius.
pub struct BidirectionalDijkstraShortestPath<'g, G> {
    graph: &'g G,
    radius: f64,
}

impl<'g, G: Graph> BidirectionalDijkstraShortestPath<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self {
            graph,
            radius: f64::INFINITY,
        }
    }

    pub fn with_radius(graph: &'g G, radius: f64) -> Result<Self> {
        if radius.is_nan() || radius < 0.0 {
            return Err(GraphError::InvalidArgument(
                "radius must be non-negative".into(),
            ));
        }
        Ok(Self { graph, radius })
    }
}

impl<G: Graph> ShortestPathAlgorithm<G> for BidirectionalDijkstraShortestPath<'_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        materialize_single_source(self.graph, source, |target| self.path(source, target))
    }

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

/// Bidirectional A* point-to-point search.
///
/// Both frontiers re-open closed vertices when the heuristic is inconsistent.
/// The conservative inconsistent-heuristic stopping rule is used, so any
/// admissible heuristic is accepted.
pub struct BidirectionalAStarShortestPath<'g, G, F> {
    graph: &'g G,
    heuristic: F,
}

impl<'g, G: Graph, F: Fn(&G::Vertex, &G::Vertex) -> f64> BidirectionalAStarShortestPath<'g, G, F> {
    pub fn new(graph: &'g G, heuristic: F) -> Self {
        Self { graph, heuristic }
    }
}

impl<G: Graph, F: Fn(&G::Vertex, &G::Vertex) -> f64> ShortestPathAlgorithm<G>
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

struct AStarSide<'g, G: Graph> {
    score: HashMap<&'g G::Vertex, f64>,
    key: HashMap<&'g G::Vertex, f64>,
    pred: HashMap<&'g G::Vertex, (&'g G::Edge, &'g G::Vertex)>,
    open: HashSet<&'g G::Vertex>,
    closed: HashSet<&'g G::Vertex>,
    nodes: Vec<&'g G::Vertex>,
    heap: BinaryHeap<Reverse<(OrderedF64, usize)>>,
}

impl<'g, G: Graph> AStarSide<'g, G> {
    fn new(start: &'g G::Vertex, estimate: f64) -> Self {
        Self {
            score: HashMap::from([(start, 0.0)]),
            key: HashMap::from([(start, estimate)]),
            pred: HashMap::new(),
            open: HashSet::from([start]),
            closed: HashSet::new(),
            nodes: vec![start],
            heap: BinaryHeap::from([Reverse((OrderedF64(estimate), 0))]),
        }
    }

    fn clean(&mut self) {
        while let Some(Reverse((OrderedF64(key), index))) = self.heap.peek().copied() {
            let vertex = self.nodes[index];
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

    fn pop(&mut self) -> Option<(&'g G::Vertex, f64)> {
        self.clean();
        let Reverse((_, index)) = self.heap.pop()?;
        let vertex = self.nodes[index];
        self.open.remove(vertex);
        self.closed.insert(vertex);
        Some((vertex, self.score[vertex]))
    }

    fn update(
        &mut self,
        vertex: &'g G::Vertex,
        parent: &'g G::Vertex,
        edge: &'g G::Edge,
        score: f64,
        key: f64,
    ) {
        if score >= self.score.get(vertex).copied().unwrap_or(f64::INFINITY) {
            return;
        }
        self.score.insert(vertex, score);
        self.key.insert(vertex, key);
        self.pred.insert(vertex, (edge, parent));
        self.closed.remove(vertex);
        self.open.insert(vertex);
        self.nodes.push(vertex);
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
    G: Graph,
    F: Fn(&G::Vertex, &G::Vertex) -> f64,
{
    let source = graph
        .vertices()
        .find(|vertex| *vertex == source)
        .ok_or(GraphError::VertexNotFound)?;
    let target = graph
        .vertices()
        .find(|vertex| *vertex == target)
        .ok_or(GraphError::VertexNotFound)?;
    if source == target {
        return Ok(Some(GraphWalk::single(source.clone())));
    }

    let mut forward = AStarSide::<G>::new(source, heuristic(source, target));
    let mut backward = AStarSide::<G>::new(target, heuristic(source, target));
    let mut scan_forward = true;
    let mut best = f64::INFINITY;
    let mut meeting = None;

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
        let adjacent: Vec<_> = if scan_forward {
            graph.try_outgoing_of(vertex)?.collect()
        } else {
            graph.try_incoming_of(vertex)?.collect()
        };
        for (edge, successor) in adjacent {
            if successor == vertex {
                continue;
            }
            let edge_weight = graph.edge_weight(edge).unwrap_or(1.0);
            if edge_weight.is_nan() || edge_weight < 0.0 {
                return Err(GraphError::InvalidArgument(
                    "negative or NaN edge weight".into(),
                ));
            }
            let candidate = vertex_score + edge_weight;
            let estimate = if scan_forward {
                heuristic(successor, target)
            } else {
                heuristic(source, successor)
            };
            side.update(successor, vertex, edge, candidate, candidate + estimate);

            let joined = vertex_score
                + edge_weight
                + other.score.get(successor).copied().unwrap_or(f64::INFINITY);
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
    let mut current = meeting;
    while let Some(&(edge, parent)) = forward.pred.get(current) {
        edges.push(edge.clone());
        vertices.push(parent.clone());
        current = parent;
    }
    vertices.reverse();
    edges.reverse();
    current = meeting;
    while let Some(&(edge, parent)) = backward.pred.get(current) {
        edges.push(edge.clone());
        vertices.push(parent.clone());
        current = parent;
    }
    Ok(Some(GraphWalk {
        vertices,
        edges,
        weight: best,
    }))
}

struct Side<'g, G: Graph> {
    dist: HashMap<&'g G::Vertex, f64>,
    /// Edge used to reach the vertex and the vertex it was reached from.
    pred: HashMap<&'g G::Vertex, (&'g G::Edge, &'g G::Vertex)>,
    settled: HashSet<&'g G::Vertex>,
    nodes: Vec<&'g G::Vertex>,
    heap: BinaryHeap<Reverse<(OrderedF64, usize)>>,
}

impl<'g, G: Graph> Side<'g, G> {
    fn new(start: &'g G::Vertex) -> Self {
        Side {
            dist: HashMap::from([(start, 0.0)]),
            pred: HashMap::new(),
            settled: HashSet::new(),
            nodes: vec![start],
            heap: BinaryHeap::from([Reverse((OrderedF64(0.0), 0))]),
        }
    }
    fn top(&mut self) -> f64 {
        while let Some(&Reverse((OrderedF64(d), i))) = self.heap.peek() {
            if self.settled.contains(self.nodes[i]) || d > self.dist[self.nodes[i]] {
                self.heap.pop();
            } else {
                return d;
            }
        }
        f64::INFINITY
    }
}

impl<G: Graph> BidirectionalDijkstra<'_, G> {
    pub fn path(
        &self,
        s: &G::Vertex,
        t: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        let g = self.0;
        let s = g
            .vertices()
            .find(|v| *v == s)
            .ok_or(GraphError::VertexNotFound)?;
        let t = g
            .vertices()
            .find(|v| *v == t)
            .ok_or(GraphError::VertexNotFound)?;
        if s == t {
            return Ok(Some(GraphWalk::single(s.clone())));
        }
        let mut f = Side::<G>::new(s);
        let mut b = Side::<G>::new(t);
        let mut mu = f64::INFINITY;
        let mut meet: Option<&G::Vertex> = None;
        loop {
            let (tf, tb) = (f.top(), b.top());
            if tf.is_infinite() && tb.is_infinite() || tf + tb >= mu {
                break;
            }
            let forward = tf <= tb;
            let (me, other) = if forward { (&mut f, &b) } else { (&mut b, &f) };
            let Reverse((OrderedF64(d), i)) = me.heap.pop().expect("non-empty frontier");
            let v = me.nodes[i];
            me.settled.insert(v);
            let edges: Vec<_> = if forward {
                g.try_outgoing_of(v)?.collect()
            } else {
                g.try_incoming_of(v)?.collect()
            };
            for (e, n) in edges {
                let w = g.edge_weight(e).unwrap_or(1.0);
                if w.is_nan() || w < 0.0 {
                    return Err(GraphError::InvalidArgument(
                        "negative or NaN edge weight".into(),
                    ));
                }
                let nd = d + w;
                if nd < me.dist.get(n).copied().unwrap_or(f64::INFINITY) {
                    me.dist.insert(n, nd);
                    me.pred.insert(n, (e, v));
                    me.nodes.push(n);
                    me.heap.push(Reverse((OrderedF64(nd), me.nodes.len() - 1)));
                }
                if let Some(&od) = other.dist.get(n) {
                    if nd + od < mu {
                        mu = nd + od;
                        meet = Some(n);
                    }
                }
            }
        }
        let Some(m) = meet else { return Ok(None) };
        let (mut vertices, mut edges) = (vec![m.clone()], Vec::new());
        let mut cur = m;
        while let Some(&(e, p)) = f.pred.get(cur) {
            edges.push(e.clone());
            vertices.push(p.clone());
            cur = p;
        }
        vertices.reverse();
        edges.reverse();
        cur = m;
        while let Some(&(e, p)) = b.pred.get(cur) {
            edges.push(e.clone());
            vertices.push(p.clone());
            cur = p;
        }
        Ok(Some(GraphWalk {
            vertices,
            edges,
            weight: mu,
        }))
    }
}
