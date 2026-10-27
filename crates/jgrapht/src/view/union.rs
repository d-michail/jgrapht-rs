use crate::GraphType;
use crate::graph::{DirectedGraph, Graph, UndirectedGraph};

/// How to merge the weights of an edge present in both graphs of a union.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeightCombiner {
    Sum,
    Mult,
    Min,
    Max,
    First,
    Second,
}

impl WeightCombiner {
    pub fn combine(self, a: f64, b: f64) -> f64 {
        match self {
            WeightCombiner::Sum => a + b,
            WeightCombiner::Mult => a * b,
            WeightCombiner::Min => a.min(b),
            WeightCombiner::Max => a.max(b),
            WeightCombiner::First => a,
            WeightCombiner::Second => b,
        }
    }
}

/// Read-only union of two graphs over the same vertex and edge types (`AsGraphUnion`). Shared
/// vertices/edges are merged. Uniting a directed with an undirected graph gives a *mixed* graph.
pub struct GraphUnion<A, B> {
    a: A,
    b: B,
    combiner: WeightCombiner,
}

impl<A, B> GraphUnion<A, B>
where
    A: Graph,
    B: Graph<Vertex = A::Vertex, Edge = A::Edge>,
{
    /// Union with weights of shared edges summed.
    pub fn new(a: A, b: B) -> Self {
        Self::with_combiner(a, b, WeightCombiner::Sum)
    }
    pub fn with_combiner(a: A, b: B, combiner: WeightCombiner) -> Self {
        GraphUnion { a, b, combiner }
    }
}

impl<A, B> Graph for GraphUnion<A, B>
where
    A: Graph,
    B: Graph<Vertex = A::Vertex, Edge = A::Edge>,
{
    type Vertex = A::Vertex;
    type Edge = A::Edge;
    fn graph_type(&self) -> GraphType {
        let (x, y) = (self.a.graph_type(), self.b.graph_type());
        let base = GraphType {
            directed: x.directed && y.directed,
            self_loops: x.self_loops || y.self_loops,
            multiple_edges: true,
            weighted: true,
            allows_cycles: true,
            modifiable: false,
            mixed: false,
        };
        if x.directed == y.directed {
            base
        } else {
            base.as_mixed()
        }
    }
    fn vertices(&self) -> impl Iterator<Item = &A::Vertex> {
        self.a
            .vertices()
            .chain(self.b.vertices().filter(|v| !self.a.contains_vertex(v)))
    }
    fn edges(&self) -> impl Iterator<Item = &A::Edge> {
        self.a
            .edges()
            .chain(self.b.edges().filter(|e| !self.a.contains_edge(e)))
    }
    fn vertex_count(&self) -> usize {
        self.vertices().count()
    }
    fn edge_count(&self) -> usize {
        self.edges().count()
    }
    fn contains_vertex(&self, v: &A::Vertex) -> bool {
        self.a.contains_vertex(v) || self.b.contains_vertex(v)
    }
    fn contains_edge(&self, e: &A::Edge) -> bool {
        self.a.contains_edge(e) || self.b.contains_edge(e)
    }
    fn edge_source(&self, e: &A::Edge) -> Option<&A::Vertex> {
        self.a.edge_source(e).or_else(|| self.b.edge_source(e))
    }
    fn edge_target(&self, e: &A::Edge) -> Option<&A::Vertex> {
        self.a.edge_target(e).or_else(|| self.b.edge_target(e))
    }
    fn edge_weight(&self, e: &A::Edge) -> Option<f64> {
        match (self.a.edge_weight(e), self.b.edge_weight(e)) {
            (Some(x), Some(y)) => Some(self.combiner.combine(x, y)),
            (x, y) => x.or(y),
        }
    }
    fn is_edge_directed(&self, e: &A::Edge) -> bool {
        if self.a.contains_edge(e) {
            self.a.is_edge_directed(e)
        } else {
            self.b.is_edge_directed(e)
        }
    }
    fn edges_of(&self, v: &A::Vertex) -> impl Iterator<Item = &A::Edge> {
        self.a
            .edges_of(v)
            .chain(self.b.edges_of(v).filter(|e| !self.a.contains_edge(e)))
    }
    fn edges_between<'x>(
        &'x self,
        u: &'x A::Vertex,
        v: &'x A::Vertex,
    ) -> impl Iterator<Item = &'x A::Edge> {
        self.a.edges_between(u, v).chain(
            self.b
                .edges_between(u, v)
                .filter(|e| !self.a.contains_edge(e)),
        )
    }
}

impl<A, B> DirectedGraph for GraphUnion<A, B>
where
    A: DirectedGraph,
    B: DirectedGraph<Vertex = A::Vertex, Edge = A::Edge>,
{
}
impl<A, B> UndirectedGraph for GraphUnion<A, B>
where
    A: UndirectedGraph,
    B: UndirectedGraph<Vertex = A::Vertex, Edge = A::Edge>,
{
}
