//! Graph generators. A generator describes *structure* as vertex indices `0..n` plus index
//! edges; [`GraphGenerator::generate`] maps indices to real vertices with a caller-supplied
//! function and adds them to any [`GraphMut`].
//!
//! Edges the target graph refuses (parallel edges in a simple graph, ...) are skipped, as
//! in JGraphT.

mod random;
mod structured;

pub use random::{BarabasiAlbert, Gnm, Gnp, Prufer, WattsStrogatz};
pub use structured::{
    Complete, CompleteBipartite, Empty, Grid, Hypercube, Linear, Ring, Star, Wheel,
};

use crate::graph::GraphMut;
use crate::{GraphError, Result};

/// Index-based description of a generated graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structure {
    pub vertex_count: usize,
    pub edges: Vec<(usize, usize)>,
}

pub trait GraphGenerator {
    /// Produces the structure. `directed` tells whether the target is a directed graph.
    fn structure(&mut self, directed: bool) -> Result<Structure>;

    /// Adds the generated vertices and edges to `target`; returns the new vertices in index order.
    fn generate<G: GraphMut>(
        &mut self,
        target: &mut G,
        mut vertex: impl FnMut(usize) -> G::Vertex,
    ) -> Result<Vec<G::Vertex>> {
        let s = self.structure(target.graph_type().directed)?;
        let vs: Vec<_> = (0..s.vertex_count).map(&mut vertex).collect();
        for v in &vs {
            target.add_vertex(v.clone());
        }
        for (a, b) in s.edges {
            match target.add_edge(&vs[a], &vs[b]) {
                Ok(_) | Err(GraphError::LoopsNotAllowed | GraphError::CycleProhibited) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(vs)
    }

    /// Convenience for graphs whose vertices are `usize` indices.
    fn generate_indexed<G: GraphMut<Vertex = usize>>(&mut self, target: &mut G) -> Result<()> {
        self.generate(target, |i| i).map(drop)
    }
}

fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(GraphError::InvalidArgument(msg.into()))
}
