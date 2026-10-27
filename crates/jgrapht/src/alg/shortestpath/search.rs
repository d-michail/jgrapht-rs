// (C) Copyright 2003-2026, by John V Sichi and JGraphT Contributors.
// Rust port (C) Copyright 2026, by Dimitrios Michail and jgrapht-rs Contributors.
//
// See the CONTRIBUTORS.md file distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the GNU Lesser General Public
// License v2.1 or later which is available at
// http://www.gnu.org/licenses/old-licenses/lgpl-2.1-standalone.html.
//
// SPDX-License-Identifier: EPL-2.0 OR LGPL-2.1-or-later

//! Heap-driven best-first search shared by Dijkstra, A* and Johnson.

use super::SingleSourcePaths;
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::{GraphError, Result};
use rheaps::DecreaseKeyHeap;
use std::collections::HashMap;

/// Best-first search from `source`. `weight(e, tail, head)` is the cost of traversing an edge
/// (must be non-negative), `estimate(v)` a lower bound of the remaining cost (`0` for Dijkstra).
/// Stops once `target` is settled; vertices farther than `radius` are ignored.
pub(crate) fn search<G, H>(
    g: &G,
    source: &G::Vertex,
    target: Option<&G::Vertex>,
    weight: impl Fn(&G::Edge, &G::Vertex, &G::Vertex) -> f64,
    estimate: impl Fn(&G::Vertex) -> f64,
    radius: f64,
) -> Result<SingleSourcePaths<G::Vertex, G::Edge>>
where
    G: Graph,
    H: DecreaseKeyHeap<OrderedF64, usize> + Default,
{
    if !g.contains_vertex(source) {
        return Err(GraphError::VertexNotFound);
    }
    let mut nodes: Vec<&G::Vertex> = vec![source];
    let mut index: HashMap<&G::Vertex, usize> = HashMap::from([(source, 0)]);
    let mut dist = vec![0.0f64];
    let mut pred: Vec<Option<(&G::Edge, usize)>> = vec![None];
    let mut open: Vec<Option<H::Handle>> = vec![None];
    let mut settled = vec![false];
    let mut heap = H::default();
    open[0] = Some(heap.insert(OrderedF64(estimate(source)), 0));

    while let Some((_, i)) = heap.pop() {
        open[i] = None;
        settled[i] = true;
        let v = nodes[i];
        if target == Some(v) {
            break;
        }
        for (e, n) in g.outgoing_of(v) {
            let w = weight(e, v, n);
            if w.is_nan() || w < 0.0 {
                return Err(GraphError::InvalidArgument(
                    "negative or NaN edge weight".into(),
                ));
            }
            let nd = dist[i] + w;
            if nd > radius {
                continue;
            }
            let j = *index.entry(n).or_insert_with(|| {
                nodes.push(n);
                dist.push(f64::INFINITY);
                pred.push(None);
                open.push(None);
                settled.push(false);
                nodes.len() - 1
            });
            if nd < dist[j] {
                dist[j] = nd;
                pred[j] = Some((e, i));
                let key = OrderedF64(nd + estimate(n));
                match open[j] {
                    Some(h) => heap.decrease_key(h, key).expect("key decreases"),
                    None => open[j] = Some(heap.insert(key, j)),
                }
            }
        }
    }

    let mut out = SingleSourcePaths {
        source: source.clone(),
        dist: HashMap::new(),
        pred: HashMap::new(),
    };
    for (j, v) in nodes.iter().enumerate() {
        if dist[j].is_finite() && (settled[j] || target.is_none()) {
            out.dist.insert((*v).clone(), dist[j]);
            if let Some((e, p)) = pred[j] {
                out.pred.insert((*v).clone(), (e.clone(), nodes[p].clone()));
            }
        }
    }
    Ok(out)
}
