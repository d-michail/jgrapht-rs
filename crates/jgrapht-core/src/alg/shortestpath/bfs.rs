use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::{Graph, GraphExt};
use crate::{GraphError, Result};
use std::collections::{HashMap, VecDeque};

/// Fewest-edges paths (edge weights are ignored; a path's weight is its edge count).
pub struct BfsShortestPath<'g, G>(pub &'g G);

impl<G: Graph> ShortestPathAlgorithm<G> for BfsShortestPath<'_, G> {
    fn single_source(&self, s: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        let g = self.0;
        if !g.contains_vertex(s) {
            return Err(GraphError::VertexNotFound);
        }
        let mut out = SingleSourcePaths {
            source: s.clone(),
            dist: HashMap::from([(s.clone(), 0.0)]),
            pred: HashMap::new(),
        };
        let mut q = VecDeque::from([s]);
        while let Some(v) = q.pop_front() {
            let d = out.dist[v];
            for (e, n) in g.outgoing_of(v) {
                if !out.dist.contains_key(n) {
                    out.dist.insert(n.clone(), d + 1.0);
                    out.pred.insert(n.clone(), (e.clone(), v.clone()));
                    q.push_back(n);
                }
            }
        }
        Ok(out)
    }
}
