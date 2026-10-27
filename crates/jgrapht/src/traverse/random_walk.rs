use crate::graph::{Graph, GraphExt};
use crate::util::java::RandomSource;
use crate::{GraphError, Result};
use std::collections::HashMap;

/// Random walk over outgoing edges (JGraphT's `RandomWalkVertexIterator`): yields the start
/// vertex, then up to `max_hops` further vertices, stopping early at a sink.
pub struct RandomWalkVertexIterator<'g, G: Graph + 'g, R> {
    g: &'g G,
    weighted: bool,
    out_weight: HashMap<&'g G::Vertex, f64>,
    max_hops: u64,
    hops: u64,
    next_vertex: Option<&'g G::Vertex>,
    rng: R,
}

impl<'g, G: Graph + 'g, R: RandomSource> RandomWalkVertexIterator<'g, G, R> {
    pub fn new(
        g: &'g G,
        start: &'g G::Vertex,
        max_hops: u64,
        weighted: bool,
        rng: R,
    ) -> Result<Self> {
        if !g.contains_vertex(start) {
            return Err(GraphError::InvalidArgument(
                "Random walk must start at a graph vertex".into(),
            ));
        }
        Ok(RandomWalkVertexIterator {
            g,
            weighted,
            out_weight: HashMap::new(),
            max_hops,
            hops: 0,
            next_vertex: Some(start),
            rng,
        })
    }

    fn compute_next(&mut self) {
        let Some(cur) = self.next_vertex else { return };
        if self.hops >= self.max_hops {
            self.next_vertex = None;
            return;
        }
        self.hops += 1;
        let g = self.g;
        if g.out_degree_of(cur).unwrap() == 0 {
            self.next_vertex = None;
            return;
        }
        let edge = if self.weighted {
            let total = *self.out_weight.entry(cur).or_insert_with(|| {
                g.out_edges_of(cur).unwrap()
                    .map(|e| g.edge_weight(e).unwrap_or(1.0))
                    .sum()
            });
            let p = total * self.rng.next_double();
            let mut cumulative = 0.0;
            let mut picked = None;
            for e in g.out_edges_of(cur).unwrap() {
                cumulative += g.edge_weight(e).unwrap_or(1.0);
                if p <= cumulative {
                    picked = Some(e);
                    break;
                }
            }
            picked
        } else {
            let edges: Vec<_> = g.out_edges_of(cur).unwrap().collect();
            Some(edges[self.rng.next_int_bound(edges.len())])
        };
        self.next_vertex = edge.and_then(|e| g.opposite_vertex(e, cur));
    }
}

impl<'g, G: Graph + 'g, R: RandomSource> Iterator for RandomWalkVertexIterator<'g, G, R> {
    type Item = &'g G::Vertex;
    fn next(&mut self) -> Option<&'g G::Vertex> {
        let v = self.next_vertex?;
        self.compute_next();
        Some(v)
    }
}
