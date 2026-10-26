//! Graph generators, ported from JGraphT step for step: vertices are created through the target's
//! vertex factory (`add_new_vertex`) in the same order, edges are added in the same order, and the
//! random generators consume random numbers exactly like the Java code, so seeded runs with a
//! [`JavaRandom`](crate::util::java::JavaRandom) reproduce JGraphT's graphs.
//!
//! Invalid parameters are reported when generating (as `Err`), before the target is touched.
//! A generator may refuse individual edges (a simple target rejects parallel edges); like JGraphT it
//! skips them.

pub mod named;
pub mod netgen;
mod random;
mod structured;

pub use random::{
    BarabasiAlbertForestGenerator, BarabasiAlbertGraphGenerator, DirectedScaleFreeGraphGenerator,
    GnmRandomBipartiteGraphGenerator, GnmRandomGraphGenerator, GnpRandomBipartiteGraphGenerator,
    GnpRandomGraphGenerator, KleinbergSmallWorldGraphGenerator,
    LinearizedChordDiagramGraphGenerator, PlantedPartitionGraphGenerator, PruferTreeGenerator,
    RandomRegularGraphGenerator, ScaleFreeGraphGenerator, WattsStrogatzGraphGenerator,
    ordered_partitions,
};
pub use structured::{
    Complement, Complete, CompleteBipartite, Empty, GeneralizedPetersen, Grid, Hypercube, Linear,
    Ring, SimpleWeightedBipartiteGraphMatrixGenerator, SimpleWeightedGraphMatrixGenerator, Star,
    Wheel, Windmill, WindmillMode,
};

use crate::graph::Graph;
use crate::view::forward_graph;
use crate::{GraphError, GraphMut, GraphType, Result};
use std::collections::HashMap;

/// Named vertices produced by a generator (JGraphT's `resultMap`); single vertices are one-element lists.
pub type GeneratorResult<V> = HashMap<String, Vec<V>>;

pub(crate) fn put<V>(result: &mut Option<&mut GeneratorResult<V>>, key: impl Into<String>, v: V) {
    if let Some(r) = result {
        r.insert(key.into(), vec![v]);
    }
}

pub(crate) fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(GraphError::InvalidArgument(msg.into()))
}

/// Adds an edge the way generators do: a refused edge (`Ok(None)`) is skipped, real errors
/// (unknown vertex, loops not allowed, ...) propagate.
pub(crate) fn add_edge<G: GraphMut>(
    g: &mut G,
    u: &G::Vertex,
    v: &G::Vertex,
) -> Result<Option<G::Edge>> {
    g.add_edge(u, v)
}

pub trait GraphGenerator {
    /// Generates into `target`, creating vertices with the target's vertex factory. Named vertices
    /// are stored in `result` when given.
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        result: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()>;

    /// [`GraphGenerator::generate_graph`] without a result map.
    fn generate_default<G: GraphMut>(&mut self, target: &mut G) -> Result<()> {
        self.generate_graph(target, None)
    }

    /// Generates with vertices produced by `vertex(0)`, `vertex(1)`, ... instead of the target's factory.
    fn generate<G: GraphMut>(
        &mut self,
        target: &mut G,
        vertex: impl FnMut(usize) -> G::Vertex,
    ) -> Result<()> {
        let mut adapter = WithVertexFactory {
            g: target,
            f: vertex,
            next: 0,
        };
        self.generate_graph(&mut adapter, None)
    }

    /// Generates into a graph whose vertices are `usize` indices `0, 1, ...`.
    fn generate_indexed<G: GraphMut<Vertex = usize>>(&mut self, target: &mut G) -> Result<()> {
        self.generate(target, |i| i)
    }
}

/// Wraps a graph, replacing its vertex factory by a closure.
pub struct WithVertexFactory<'a, G, F> {
    g: &'a mut G,
    f: F,
    next: usize,
}

impl<G: Graph, F> Graph for WithVertexFactory<'_, G, F> {
    type Vertex = G::Vertex;
    type Edge = G::Edge;
    forward_graph!(self => self.g; graph_type, vertices, edges, vertex_count, edge_count, contains_vertex, contains_edge,
        edge_source, edge_target, edge_weight, is_edge_directed, edges_of, in_edges_of, out_edges_of, in_degree_of,
        out_degree_of, degree_of, edges_between);
}

impl<G: GraphMut, F: FnMut(usize) -> G::Vertex> GraphMut for WithVertexFactory<'_, G, F> {
    fn add_vertex(&mut self, v: G::Vertex) -> bool {
        self.g.add_vertex(v)
    }
    fn remove_vertex(&mut self, v: &G::Vertex) -> bool {
        self.g.remove_vertex(v)
    }
    fn add_edge_with(&mut self, u: &G::Vertex, v: &G::Vertex, e: G::Edge) -> Result<bool> {
        self.g.add_edge_with(u, v, e)
    }
    fn remove_edge(&mut self, e: &G::Edge) -> bool {
        self.g.remove_edge(e)
    }
    fn set_edge_weight(&mut self, e: &G::Edge, w: f64) -> Result<()> {
        self.g.set_edge_weight(e, w)
    }
    fn new_edge(&mut self) -> Result<G::Edge> {
        self.g.new_edge()
    }
    fn new_vertex(&mut self) -> Result<G::Vertex> {
        let v = (self.f)(self.next);
        self.next += 1;
        Ok(v)
    }
}

/// Index-based description of a named graph (vertex count plus edges in insertion order).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Structure {
    pub vertex_count: usize,
    pub edges: Vec<(usize, usize)>,
    pub groups: std::collections::BTreeMap<&'static str, Vec<usize>>,
}

pub(crate) fn require_directed_or_undirected(ty: &GraphType) -> Result<()> {
    if ty.mixed {
        return invalid("Graph must be directed or undirected");
    }
    Ok(())
}
