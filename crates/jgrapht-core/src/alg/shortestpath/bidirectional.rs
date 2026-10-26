use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Bidirectional Dijkstra: a forward search from the source and a backward search (over
/// incoming edges) from the sink, stopped when the frontiers can no longer improve the best
/// meeting point. Requires non-negative weights.
pub struct BidirectionalDijkstra<'g, G>(pub &'g G);

struct Side<'g, G: Graph> {
    dist: HashMap<&'g G::Vertex, f64>,
    /// Edge used to reach the vertex and the vertex it was reached from.
    pred: HashMap<&'g G::Vertex, (&'g G::Edge, &'g G::Vertex)>,
    settled: HashSet<&'g G::Vertex>,
    nodes: Vec<&'g G::Vertex>,
    heap: BinaryHeap<Reverse<(OrderedF64, usize)>>,
}

impl<'g, G: Graph> Side<'g, G> {
    fn new(start: &'g G::Vertex) -> Self {
        Side {
            dist: HashMap::from([(start, 0.0)]),
            pred: HashMap::new(),
            settled: HashSet::new(),
            nodes: vec![start],
            heap: BinaryHeap::from([Reverse((OrderedF64(0.0), 0))]),
        }
    }
    fn top(&mut self) -> f64 {
        while let Some(&Reverse((OrderedF64(d), i))) = self.heap.peek() {
            if self.settled.contains(self.nodes[i]) || d > self.dist[self.nodes[i]] {
                self.heap.pop();
            } else {
                return d;
            }
        }
        f64::INFINITY
    }
}

impl<G: Graph> BidirectionalDijkstra<'_, G> {
    pub fn path(
        &self,
        s: &G::Vertex,
        t: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        let g = self.0;
        let s = g
            .vertices()
            .find(|v| *v == s)
            .ok_or(GraphError::VertexNotFound)?;
        let t = g
            .vertices()
            .find(|v| *v == t)
            .ok_or(GraphError::VertexNotFound)?;
        if s == t {
            return Ok(Some(GraphWalk::single(s.clone())));
        }
        let mut f = Side::<G>::new(s);
        let mut b = Side::<G>::new(t);
        let mut mu = f64::INFINITY;
        let mut meet: Option<&G::Vertex> = None;
        loop {
            let (tf, tb) = (f.top(), b.top());
            if tf.is_infinite() && tb.is_infinite() || tf + tb >= mu {
                break;
            }
            let forward = tf <= tb;
            let (me, other) = if forward { (&mut f, &b) } else { (&mut b, &f) };
            let Reverse((OrderedF64(d), i)) = me.heap.pop().expect("non-empty frontier");
            let v = me.nodes[i];
            me.settled.insert(v);
            let edges: Vec<_> = if forward {
                g.outgoing_of(v).collect()
            } else {
                g.incoming_of(v).collect()
            };
            for (e, n) in edges {
                let w = g.edge_weight(e).unwrap_or(1.0);
                if w.is_nan() || w < 0.0 {
                    return Err(GraphError::InvalidArgument(
                        "negative or NaN edge weight".into(),
                    ));
                }
                let nd = d + w;
                if nd < me.dist.get(n).copied().unwrap_or(f64::INFINITY) {
                    me.dist.insert(n, nd);
                    me.pred.insert(n, (e, v));
                    me.nodes.push(n);
                    me.heap.push(Reverse((OrderedF64(nd), me.nodes.len() - 1)));
                }
                if let Some(&od) = other.dist.get(n) {
                    if nd + od < mu {
                        mu = nd + od;
                        meet = Some(n);
                    }
                }
            }
        }
        let Some(m) = meet else { return Ok(None) };
        let (mut vertices, mut edges) = (vec![m.clone()], Vec::new());
        let mut cur = m;
        while let Some(&(e, p)) = f.pred.get(cur) {
            edges.push(e.clone());
            vertices.push(p.clone());
            cur = p;
        }
        vertices.reverse();
        edges.reverse();
        cur = m;
        while let Some(&(e, p)) = b.pred.get(cur) {
            edges.push(e.clone());
            vertices.push(p.clone());
            cur = p;
        }
        Ok(Some(GraphWalk {
            vertices,
            edges,
            weight: mu,
        }))
    }
}
