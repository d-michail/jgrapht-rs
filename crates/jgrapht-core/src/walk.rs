//! Paths/walks through a graph and vertex/edge mappings between graphs.

use crate::graph::{Graph, GraphExt};
use std::collections::HashMap;
use std::hash::Hash;

/// A walk: consecutive vertices joined by edges (JGraphT's `GraphWalk`/`GraphPath`).
#[derive(Debug, Clone, PartialEq)]
pub struct GraphWalk<V, E> {
    pub start: V,
    pub end: V,
    pub vertices: Vec<V>,
    pub edges: Vec<E>,
    pub weight: f64,
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash> GraphWalk<V, E> {
    /// Zero-length walk.
    pub fn single(v: V) -> Self {
        GraphWalk {
            start: v.clone(),
            end: v.clone(),
            vertices: vec![v],
            edges: vec![],
            weight: 0.0,
        }
    }

    /// Builds a walk from its edges; the vertex list is derived from `g`.
    /// Returns `None` if the edges are not consecutive starting at `start`.
    pub fn from_edges<G>(g: &G, start: V, edges: Vec<E>, weight: f64) -> Option<Self>
    where
        G: Graph<Vertex = V, Edge = E>,
    {
        let mut vertices = vec![start.clone()];
        let mut cur = start.clone();
        for e in &edges {
            cur = g.opposite_vertex(e, &cur)?.clone();
            vertices.push(cur.clone());
        }
        Some(GraphWalk {
            start,
            end: cur,
            vertices,
            edges,
            weight,
        })
    }

    /// Builds a walk from a vertex list, picking the lightest edge between consecutive vertices.
    pub fn from_vertices<G>(g: &G, vertices: Vec<V>) -> Option<Self>
    where
        G: Graph<Vertex = V, Edge = E>,
    {
        let (start, end) = (vertices.first()?.clone(), vertices.last()?.clone());
        let mut edges = Vec::with_capacity(vertices.len() - 1);
        let mut weight = 0.0;
        for w in vertices.windows(2) {
            let (e, we) = g
                .edges_between(&w[0], &w[1])
                .map(|e| (e, g.edge_weight(e).unwrap_or(1.0)))
                .min_by(|a, b| a.1.total_cmp(&b.1))?;
            edges.push(e.clone());
            weight += we;
        }
        Some(GraphWalk {
            start,
            end,
            vertices,
            edges,
            weight,
        })
    }

    /// Number of edges.
    pub fn length(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// Whether this walk exists in `g` and its recorded weight matches.
    pub fn is_valid<G>(&self, g: &G) -> bool
    where
        G: Graph<Vertex = V, Edge = E>,
    {
        let mut cur = &self.start;
        let mut w = 0.0;
        for e in &self.edges {
            match g.opposite_vertex(e, cur) {
                Some(n) => cur = n,
                None => return false,
            }
            w += g.edge_weight(e).unwrap_or(1.0);
        }
        cur == &self.end && (w - self.weight).abs() <= 1e-9 * w.abs().max(1.0)
    }

    /// Reversed walk (only meaningful in undirected graphs).
    pub fn reversed(&self) -> Self {
        GraphWalk {
            start: self.end.clone(),
            end: self.start.clone(),
            vertices: self.vertices.iter().rev().cloned().collect(),
            edges: self.edges.iter().rev().cloned().collect(),
            weight: self.weight,
        }
    }

    /// Appends `other`, which must start where `self` ends.
    pub fn concat(&self, other: &Self) -> Option<Self> {
        if self.end != other.start {
            return None;
        }
        let mut vertices = self.vertices.clone();
        vertices.extend(other.vertices.iter().skip(1).cloned());
        let mut edges = self.edges.clone();
        edges.extend(other.edges.iter().cloned());
        Some(GraphWalk {
            start: self.start.clone(),
            end: other.end.clone(),
            vertices,
            edges,
            weight: self.weight + other.weight,
        })
    }
}

/// Bidirectional mapping between vertices/edges of two graphs (`GraphMapping`).
#[derive(Debug, Clone)]
pub struct GraphMapping<V, E> {
    pub vertex_forward: HashMap<V, V>,
    pub vertex_backward: HashMap<V, V>,
    pub edge_forward: HashMap<E, E>,
    pub edge_backward: HashMap<E, E>,
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash> Default for GraphMapping<V, E> {
    fn default() -> Self {
        GraphMapping {
            vertex_forward: HashMap::new(),
            vertex_backward: HashMap::new(),
            edge_forward: HashMap::new(),
            edge_backward: HashMap::new(),
        }
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash> GraphMapping<V, E> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn map_vertex(&mut self, from: V, to: V) {
        self.vertex_forward.insert(from.clone(), to.clone());
        self.vertex_backward.insert(to, from);
    }
    pub fn map_edge(&mut self, from: E, to: E) {
        self.edge_forward.insert(from.clone(), to.clone());
        self.edge_backward.insert(to, from);
    }
    pub fn vertex(&self, from: &V) -> Option<&V> {
        self.vertex_forward.get(from)
    }
    pub fn vertex_inverse(&self, to: &V) -> Option<&V> {
        self.vertex_backward.get(to)
    }
    pub fn edge(&self, from: &E) -> Option<&E> {
        self.edge_forward.get(from)
    }
    pub fn edge_inverse(&self, to: &E) -> Option<&E> {
        self.edge_backward.get(to)
    }
}
