use super::search::search;
use crate::graph::Graph;
use crate::util::OrderedF64;
use crate::util::heap::JavaPairingHeap as PairingHeap;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use rheaps::DecreaseKeyHeap;
use std::marker::PhantomData;

/// A* search. `heuristic(v, target)` must never overestimate the remaining distance
/// (admissible); vertices are re-opened if it is not consistent.
pub struct AStar<'g, G, F, H = PairingHeap<OrderedF64, usize>> {
    g: &'g G,
    heuristic: F,
    _heap: PhantomData<fn() -> H>,
}

impl<'g, G: Graph, F: Fn(&G::Vertex, &G::Vertex) -> f64> AStar<'g, G, F> {
    pub fn new(g: &'g G, heuristic: F) -> Self {
        AStar {
            g,
            heuristic,
            _heap: PhantomData,
        }
    }
}

impl<
    'g,
    G: Graph,
    F: Fn(&G::Vertex, &G::Vertex) -> f64,
    H: DecreaseKeyHeap<OrderedF64, usize> + Default,
> AStar<'g, G, F, H>
{
    pub fn with_heap(g: &'g G, heuristic: F) -> Self {
        AStar {
            g,
            heuristic,
            _heap: PhantomData,
        }
    }

    pub fn path(
        &self,
        s: &G::Vertex,
        t: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        if !self.g.contains_vertex(t) {
            return Err(GraphError::VertexNotFound);
        }
        let r = search::<G, H>(
            self.g,
            s,
            Some(t),
            |e, _, _| self.g.edge_weight(e).unwrap_or(1.0),
            |v| (self.heuristic)(v, t),
            f64::INFINITY,
        )?;
        Ok(r.path(t))
    }
}
