use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::{Graph, GraphExt};
use crate::{GraphError, Result};
use std::collections::HashMap;

/// Bellman–Ford: handles negative edge weights, reports `NegativeCycle` if one is reachable.
/// (An undirected edge of negative weight is a negative cycle.)
pub struct BellmanFord<'g, G>(pub &'g G);

/// Relaxes all edges up to `|V|` rounds from the given initial distances.
/// Returns `Err(NegativeCycle)` if still improving afterwards.
pub(crate) fn relax_all<'g, G: Graph>(
    g: &'g G,
    dist: &mut HashMap<&'g G::Vertex, f64>,
    pred: &mut HashMap<&'g G::Vertex, (&'g G::Edge, &'g G::Vertex)>,
) -> Result<()> {
    let n = g.vertex_count();
    for round in 0..=n {
        let mut changed = false;
        for v in g.vertices() {
            let Some(&d) = dist.get(v) else { continue };
            for (e, t) in g.outgoing_of(v) {
                let nd = d + g.edge_weight(e).unwrap_or(1.0);
                if nd < dist.get(t).copied().unwrap_or(f64::INFINITY) {
                    dist.insert(t, nd);
                    pred.insert(t, (e, v));
                    changed = true;
                }
            }
        }
        if !changed {
            return Ok(());
        }
        if round == n {
            return Err(GraphError::NegativeCycle);
        }
    }
    Ok(())
}

impl<G: Graph> ShortestPathAlgorithm<G> for BellmanFord<'_, G> {
    fn single_source(&self, s: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        let g = self.0;
        let src = g
            .vertices()
            .find(|v| *v == s)
            .ok_or(GraphError::VertexNotFound)?;
        let mut dist = HashMap::from([(src, 0.0)]);
        let mut pred = HashMap::new();
        relax_all(g, &mut dist, &mut pred)?;
        Ok(SingleSourcePaths {
            source: s.clone(),
            dist: dist.into_iter().map(|(v, d)| (v.clone(), d)).collect(),
            pred: pred
                .into_iter()
                .map(|(v, (e, p))| (v.clone(), (e.clone(), p.clone())))
                .collect(),
        })
    }
}
