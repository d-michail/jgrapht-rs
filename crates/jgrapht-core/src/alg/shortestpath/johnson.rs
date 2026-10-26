use super::bellman_ford::relax_all;
use super::search::search;
use super::{AllPairsPaths, SingleSourcePaths};
use crate::Result;
use crate::graph::Graph;
use crate::util::OrderedF64;
use rheaps::tree::PairingHeap;
use std::collections::HashMap;

/// Johnson's all-pairs algorithm: Bellman–Ford potentials, then Dijkstra from every vertex on
/// reweighted edges. Good for sparse graphs with negative weights; `Err(NegativeCycle)` if the
/// graph has a negative cycle.
pub fn johnson<G: Graph>(g: &G) -> Result<AllPairsPaths<G::Vertex, G::Edge>> {
    // a virtual source reaching every vertex at cost 0 == all potentials start at 0
    let mut h: HashMap<&G::Vertex, f64> = g.vertices().map(|v| (v, 0.0)).collect();
    relax_all(g, &mut h, &mut HashMap::new())?;
    let mut by_source = HashMap::new();
    for s in g.vertices() {
        let r = search::<G, PairingHeap<OrderedF64, usize>>(
            g,
            s,
            None,
            |e, u, v| (g.edge_weight(e).unwrap_or(1.0) + h[u] - h[v]).max(0.0),
            |_| 0.0,
            f64::INFINITY,
        )?;
        let dist = r
            .dist
            .iter()
            .map(|(t, d)| (t.clone(), d - h[s] + h[t]))
            .collect();
        by_source.insert(
            s.clone(),
            SingleSourcePaths {
                source: r.source,
                dist,
                pred: r.pred,
            },
        );
    }
    Ok(AllPairsPaths { by_source })
}
