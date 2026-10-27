//! Fluent graph construction (JGraphT's `GraphBuilder` and `GraphTypeBuilder`).

use super::base::{BaseGraph, DefaultGraphSpecifics, Directed, Direction, Undirected};
use super::{Graph, GraphMut, GraphMutExt};
use crate::view::Unmodifiable;
use crate::{GraphError, GraphType, Result};
use std::hash::Hash;
use std::marker::PhantomData;

/// Populates a graph with chained calls. The first failure is remembered and reported by
/// [`GraphBuilder::build`].
pub struct GraphBuilder<G: GraphMut> {
    graph: G,
    error: Option<GraphError>,
}

impl<G: GraphMut> GraphBuilder<G> {
    pub fn new(graph: G) -> Self {
        GraphBuilder { graph, error: None }
    }

    fn record<T>(&mut self, r: Result<T>) {
        if let Err(e) = r {
            self.error.get_or_insert(e);
        }
    }

    pub fn add_vertex(mut self, v: G::Vertex) -> Self {
        self.graph.add_vertex(v);
        self
    }
    pub fn add_vertices(mut self, vs: impl IntoIterator<Item = G::Vertex>) -> Self {
        self.graph.add_vertices(vs);
        self
    }
    /// Adds the edge `u -> v`, creating missing vertices.
    pub fn add_edge(mut self, u: G::Vertex, v: G::Vertex) -> Self {
        let r = self.graph.add_edge_with_vertices(u, v);
        self.record(r);
        self
    }
    /// Adds the edge with the given weight (the graph must be weighted).
    pub fn add_weighted_edge(mut self, u: G::Vertex, v: G::Vertex, weight: f64) -> Self {
        self.graph.add_vertex(u.clone());
        self.graph.add_vertex(v.clone());
        let r = self.graph.add_weighted_edge(&u, &v, weight);
        self.record(r);
        self
    }
    /// Adds the given edge object between `u` and `v`.
    pub fn add_edge_with(mut self, u: G::Vertex, v: G::Vertex, e: G::Edge) -> Self {
        self.graph.add_vertex(u.clone());
        self.graph.add_vertex(v.clone());
        let r = self.graph.add_edge_with(&u, &v, e);
        self.record(r);
        self
    }
    pub fn add_weighted_edge_with(
        mut self,
        u: G::Vertex,
        v: G::Vertex,
        e: G::Edge,
        weight: f64,
    ) -> Self {
        self.graph.add_vertex(u.clone());
        self.graph.add_vertex(v.clone());
        let r = self
            .graph
            .add_edge_with(&u, &v, e.clone())
            .and_then(|_| self.graph.set_edge_weight(&e, weight));
        self.record(r);
        self
    }
    /// Adds the path `vs[0] -> vs[1] -> ...`.
    pub fn add_edge_chain(mut self, vs: impl IntoIterator<Item = G::Vertex>) -> Self {
        let vs: Vec<_> = vs.into_iter().collect();
        for v in &vs {
            self.graph.add_vertex(v.clone());
        }
        for w in vs.windows(2) {
            let r = self.graph.add_edge(&w[0], &w[1]);
            self.record(r);
        }
        self
    }
    /// Copies another graph (vertices, edges, weights) into this one.
    pub fn add_graph<H: Graph<Vertex = G::Vertex, Edge = G::Edge>>(mut self, other: &H) -> Self {
        let r = self.graph.add_graph(other);
        self.record(r);
        self
    }
    pub fn remove_vertex(mut self, v: &G::Vertex) -> Self {
        self.graph.remove_vertex(v);
        self
    }
    pub fn remove_vertices<'a>(mut self, vs: impl IntoIterator<Item = &'a G::Vertex>) -> Self
    where
        G::Vertex: 'a,
    {
        for v in vs {
            self.graph.remove_vertex(v);
        }
        self
    }
    /// Removes one edge between `u` and `v`.
    pub fn remove_edge_between(mut self, u: &G::Vertex, v: &G::Vertex) -> Self {
        self.graph.remove_edge_between(u, v);
        self
    }
    pub fn remove_edge(mut self, e: &G::Edge) -> Self {
        self.graph.remove_edge(e);
        self
    }
    pub fn build(self) -> Result<G> {
        match self.error {
            Some(e) => Err(e),
            None => Ok(self.graph),
        }
    }
    pub fn build_as_unmodifiable(self) -> Result<Unmodifiable<G>> {
        self.build().map(Unmodifiable)
    }
}

/// Describes the kind of graph to create. Use [`GraphTypeBuilder::directed`] or
/// [`GraphTypeBuilder::undirected`], then optionally set loops/multi-edges/weights and the vertex
/// and edge factories.
pub struct GraphTypeBuilder<V, E, D> {
    ty: GraphType,
    edge_factory: Option<fn(u64) -> E>,
    vertex_factory: Option<fn(u64) -> V>,
    _d: PhantomData<D>,
}

impl<V, E> GraphTypeBuilder<V, E, Directed> {
    pub fn directed() -> Self {
        Self::start()
    }
}
impl<V, E> GraphTypeBuilder<V, E, Undirected> {
    pub fn undirected() -> Self {
        Self::start()
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction> GraphTypeBuilder<V, E, D> {
    /// A builder preconfigured with the type and factories of an existing graph.
    pub fn for_graph(g: &BaseGraph<V, E, D>) -> Self {
        GraphTypeBuilder {
            ty: g.graph_type(),
            edge_factory: g.edge_factory(),
            vertex_factory: g.vertex_factory(),
            _d: PhantomData,
        }
    }
}

impl<V, E, D: Direction> GraphTypeBuilder<V, E, D> {
    fn start() -> Self {
        GraphTypeBuilder {
            ty: GraphType::simple().directed(D::DIRECTED),
            edge_factory: None,
            vertex_factory: None,
            _d: PhantomData,
        }
    }
    pub fn allowing_multiple_edges(mut self, allow: bool) -> Self {
        self.ty.multiple_edges = allow;
        self
    }
    pub fn allowing_self_loops(mut self, allow: bool) -> Self {
        self.ty.self_loops = allow;
        self
    }
    pub fn weighted(mut self, weighted: bool) -> Self {
        self.ty.weighted = weighted;
        self
    }
    pub fn edge_supplier(mut self, f: fn(u64) -> E) -> Self {
        self.edge_factory = Some(f);
        self
    }
    pub fn vertex_supplier(mut self, f: fn(u64) -> V) -> Self {
        self.vertex_factory = Some(f);
        self
    }
    pub fn graph_type(&self) -> GraphType {
        self.ty
    }
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash, D: Direction> GraphTypeBuilder<V, E, D> {
    pub fn build_graph(self) -> BaseGraph<V, E, D> {
        let g = match self.edge_factory {
            Some(f) => BaseGraph::<V, E, D, DefaultGraphSpecifics>::with_edge_factory(self.ty, f),
            None => BaseGraph::<V, E, D, DefaultGraphSpecifics>::without_edge_factory(self.ty),
        };
        match self.vertex_factory {
            Some(f) => g.with_vertex_factory(f),
            None => g,
        }
    }
}

impl<V> GraphTypeBuilder<V, crate::DefaultEdge, Directed> {
    /// Use [`crate::DefaultEdge`]s (the default edge type).
    pub fn default_edges(self) -> Self {
        self.edge_supplier(crate::DefaultEdge::from_id)
    }
}
impl<V> GraphTypeBuilder<V, crate::DefaultEdge, Undirected> {
    pub fn default_edges(self) -> Self {
        self.edge_supplier(crate::DefaultEdge::from_id)
    }
}
