use super::{AllPairsPaths, SingleSourcePaths};
use crate::graph::{Graph, GraphExt};
use crate::{GraphError, Result};
use std::collections::HashMap;

/// All-pairs shortest paths in `O(V^3)`. Negative weights are fine; a negative cycle gives
/// `Err(NegativeCycle)`.
pub fn floyd_warshall<G: Graph>(g: &G) -> Result<AllPairsPaths<G::Vertex, G::Edge>> {
    let vs: Vec<&G::Vertex> = g.vertices().collect();
    let n = vs.len();
    let idx: HashMap<&G::Vertex, usize> = vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
    let mut d = vec![vec![f64::INFINITY; n]; n];
    let mut pred: Vec<Vec<Option<usize>>> = vec![vec![None; n]; n]; // predecessor of j on i ~> j
    let mut edge: Vec<Vec<Option<&G::Edge>>> = vec![vec![None; n]; n]; // lightest direct edge
    for i in 0..n {
        d[i][i] = 0.0;
        for (e, t) in g.outgoing_of(vs[i]) {
            let j = idx[t];
            let w = g.edge_weight(e).unwrap_or(1.0);
            if w < d[i][j] || (i == j && w < 0.0) {
                d[i][j] = w;
                pred[i][j] = Some(i);
                edge[i][j] = Some(e);
            }
        }
    }
    for k in 0..n {
        for i in 0..n {
            if d[i][k].is_infinite() {
                continue;
            }
            for j in 0..n {
                let nd = d[i][k] + d[k][j];
                if nd < d[i][j] {
                    d[i][j] = nd;
                    pred[i][j] = pred[k][j];
                }
            }
        }
    }
    if (0..n).any(|i| d[i][i] < 0.0) {
        return Err(GraphError::NegativeCycle);
    }
    let mut by_source = HashMap::new();
    for i in 0..n {
        let mut sp = SingleSourcePaths {
            source: vs[i].clone(),
            dist: HashMap::new(),
            pred: HashMap::new(),
        };
        for j in 0..n {
            if d[i][j].is_finite() {
                sp.dist.insert(vs[j].clone(), d[i][j]);
                if i != j {
                    let p = pred[i][j].expect("finite distance has a predecessor");
                    let e = edge[p][j].expect("direct edge into j");
                    sp.pred.insert(vs[j].clone(), (e.clone(), vs[p].clone()));
                }
            }
        }
        by_source.insert(vs[i].clone(), sp);
    }
    Ok(AllPairsPaths { by_source })
}
