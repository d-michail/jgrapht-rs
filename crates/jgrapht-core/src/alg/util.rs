//! Index-based snapshot of a graph, used by algorithms that prefer dense arrays.

use crate::graph::{Graph, GraphExt};
use std::collections::HashMap;

pub struct Indexed<'g, G: Graph> {
    pub g: &'g G,
    pub vs: Vec<&'g G::Vertex>,
    pub idx: HashMap<&'g G::Vertex, usize>,
    pub es: Vec<&'g G::Edge>,
    /// `(source, target)` per edge index.
    pub ends: Vec<(usize, usize)>,
    pub w: Vec<f64>,
    /// `out[v]` = `(neighbour, edge index)`; for undirected graphs every incident edge.
    pub out: Vec<Vec<(usize, usize)>>,
    /// Incoming adjacency (same as `out` for undirected graphs).
    pub inn: Vec<Vec<(usize, usize)>>,
    pub directed: bool,
}

impl<'g, G: Graph> Indexed<'g, G> {
    pub fn new(g: &'g G) -> Self {
        let vs: Vec<_> = g.vertices().collect();
        let idx: HashMap<_, _> = vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let directed = g.graph_type().directed;
        let n = vs.len();
        let (mut es, mut ends, mut w) = (Vec::new(), Vec::new(), Vec::new());
        let mut out = vec![Vec::new(); n];
        let mut inn = vec![Vec::new(); n];
        for e in g.edges() {
            let (s, t) = g.edge_endpoints(e).expect("edge endpoints");
            let (s, t) = (idx[s], idx[t]);
            let k = es.len();
            es.push(e);
            ends.push((s, t));
            w.push(g.edge_weight(e).unwrap_or(1.0));
            out[s].push((t, k));
            if directed {
                inn[t].push((s, k));
            } else if s != t {
                out[t].push((s, k));
            }
        }
        if !directed {
            inn = out.clone();
        }
        Indexed {
            g,
            vs,
            idx,
            es,
            ends,
            w,
            out,
            inn,
            directed,
        }
    }
    pub fn n(&self) -> usize {
        self.vs.len()
    }
    pub fn vertex(&self, i: usize) -> G::Vertex {
        self.vs[i].clone()
    }
    pub fn vertices_of(&self, ix: &[usize]) -> Vec<G::Vertex> {
        ix.iter().map(|&i| self.vs[i].clone()).collect()
    }
}
