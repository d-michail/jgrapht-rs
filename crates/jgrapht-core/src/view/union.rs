use crate::GraphType;
use crate::graph::{DirectedGraph, Graph, UndirectedGraph};

/// Union of two graphs over the same vertex and edge types. Shared vertices/edges are merged;
/// for shared edges the first graph wins (endpoints and weight).
#[derive(Debug, Clone)]
pub struct GraphUnion<A, B> {
    a: A,
    b: B,
}

impl<A, B> GraphUnion<A, B>
where
    A: Graph,
    B: Graph<Vertex = A::Vertex, Edge = A::Edge>,
{
    pub fn new(a: A, b: B) -> Self {
        GraphUnion { a, b }
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
        GraphType {
            directed: x.directed,
            self_loops: x.self_loops || y.self_loops,
            multiple_edges: x.multiple_edges || y.multiple_edges,
            weighted: x.weighted || y.weighted,
            allows_cycles: x.allows_cycles || y.allows_cycles,
            modifiable: false,
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
        self.a.edge_weight(e).or_else(|| self.b.edge_weight(e))
    }
    fn edges_of(&self, v: &A::Vertex) -> impl Iterator<Item = &A::Edge> {
        self.a
            .edges_of(v)
            .chain(self.b.edges_of(v).filter(|e| !self.a.contains_edge(e)))
    }
    fn edges_between<'a>(
        &'a self,
        u: &'a A::Vertex,
        v: &'a A::Vertex,
    ) -> impl Iterator<Item = &'a A::Edge> {
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
