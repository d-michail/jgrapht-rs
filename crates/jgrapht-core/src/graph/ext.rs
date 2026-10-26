//! Convenience methods derived from the core traits (JGraphT's `Graphs` utility class).

use super::{DirectedGraph, Graph, GraphMut};
use crate::Result;

pub trait GraphExt: Graph {
    /// `(source, target)` of an edge.
    fn edge_endpoints(&self, e: &Self::Edge) -> Option<(&Self::Vertex, &Self::Vertex)> {
        Some((self.edge_source(e)?, self.edge_target(e)?))
    }

    /// The endpoint of `e` that is not `v` (`v` itself for a self-loop).
    fn opposite_vertex(&self, e: &Self::Edge, v: &Self::Vertex) -> Option<&Self::Vertex> {
        let (s, t) = self.edge_endpoints(e)?;
        if s == v {
            Some(t)
        } else if t == v {
            Some(s)
        } else {
            None
        }
    }

    /// Adjacent vertices, ignoring direction. Parallel edges yield repeats.
    fn neighbors_of<'a>(&'a self, v: &'a Self::Vertex) -> impl Iterator<Item = &'a Self::Vertex> {
        self.edges_of(v)
            .filter_map(move |e| self.opposite_vertex(e, v))
    }

    /// Edges leaving `v` with the vertex they lead to. Directed graphs follow edge direction,
    /// undirected graphs yield every incident edge.
    fn outgoing_of<'a>(
        &'a self,
        v: &'a Self::Vertex,
    ) -> impl Iterator<Item = (&'a Self::Edge, &'a Self::Vertex)> {
        let directed = self.graph_type().directed;
        self.edges_of(v).filter_map(move |e| {
            let (s, t) = self.edge_endpoints(e)?;
            if directed {
                (s == v).then_some((e, t))
            } else {
                Some((e, if s == v { t } else { s }))
            }
        })
    }

    /// Edges entering `v` with the vertex they come from (all incident edges if undirected).
    fn incoming_of<'a>(
        &'a self,
        v: &'a Self::Vertex,
    ) -> impl Iterator<Item = (&'a Self::Edge, &'a Self::Vertex)> {
        let directed = self.graph_type().directed;
        self.edges_of(v).filter_map(move |e| {
            let (s, t) = self.edge_endpoints(e)?;
            if directed {
                (t == v).then_some((e, s))
            } else {
                Some((e, if s == v { t } else { s }))
            }
        })
    }

    /// Vertices reachable over one edge (see [`GraphExt::outgoing_of`]); repeats for parallel edges.
    fn out_neighbors_of<'a>(
        &'a self,
        v: &'a Self::Vertex,
    ) -> impl Iterator<Item = &'a Self::Vertex> {
        self.outgoing_of(v).map(|(_, n)| n)
    }

    /// Sum of all edge weights.
    fn total_weight(&self) -> f64 {
        self.edges().filter_map(|e| self.edge_weight(e)).sum()
    }

    fn is_empty(&self) -> bool {
        self.vertex_count() == 0
    }
}
impl<G: Graph + ?Sized> GraphExt for G {}

pub trait DirectedExt: DirectedGraph {
    fn successors_of<'a>(&'a self, v: &'a Self::Vertex) -> impl Iterator<Item = &'a Self::Vertex> {
        self.out_edges_of(v)
            .filter_map(move |e| self.edge_target(e))
    }
    fn predecessors_of<'a>(
        &'a self,
        v: &'a Self::Vertex,
    ) -> impl Iterator<Item = &'a Self::Vertex> {
        self.in_edges_of(v).filter_map(move |e| self.edge_source(e))
    }
}
impl<G: DirectedGraph + ?Sized> DirectedExt for G {}

pub trait GraphMutExt: GraphMut {
    /// Adds both endpoints if missing, then the edge (`Graphs.addEdgeWithVertices`).
    fn add_edge_with_vertices(
        &mut self,
        u: Self::Vertex,
        v: Self::Vertex,
    ) -> Result<Option<Self::Edge>> {
        self.add_vertex(u.clone());
        self.add_vertex(v.clone());
        self.add_edge(&u, &v)
    }

    /// Adds an edge and sets its weight.
    fn add_weighted_edge(
        &mut self,
        u: &Self::Vertex,
        v: &Self::Vertex,
        w: f64,
    ) -> Result<Option<Self::Edge>> {
        let e = self.add_edge(u, v)?;
        if let Some(e) = &e {
            self.set_edge_weight(e, w)?;
        }
        Ok(e)
    }

    fn add_vertices(&mut self, vs: impl IntoIterator<Item = Self::Vertex>) -> bool {
        vs.into_iter().fold(false, |ch, v| self.add_vertex(v) | ch)
    }

    /// Copies vertices, edges and weights of `src` into `self`, reusing edge objects.
    fn add_graph<G>(&mut self, src: &G) -> Result<bool>
    where
        G: Graph<Vertex = Self::Vertex, Edge = Self::Edge>,
    {
        let mut changed = self.add_vertices(src.vertices().cloned());
        for e in src.edges() {
            let (s, t) = src.edge_endpoints(e).expect("edge of source graph");
            if self.add_edge_with(s, t, e.clone())? {
                changed = true;
                if self.graph_type().weighted
                    && let Some(w) = src.edge_weight(e)
                {
                    self.set_edge_weight(e, w)?;
                }
            }
        }
        Ok(changed)
    }

    fn remove_edges_between(&mut self, u: &Self::Vertex, v: &Self::Vertex) -> bool {
        let es: Vec<_> = self.edges_between(u, v).cloned().collect();
        es.iter().fold(false, |ch, e| self.remove_edge(e) | ch)
    }
}
impl<G: GraphMut + ?Sized> GraphMutExt for G {}
