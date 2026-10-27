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

use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::alg::util::Indexed;
use crate::graph::Graph;
use crate::{GraphError, Result};
use std::collections::HashMap;

/// Δ-stepping (Meyer & Sanders), single-threaded. Vertices are kept in buckets of width `delta`;
/// "light" edges (`w <= delta`) are relaxed repeatedly inside the current bucket, "heavy" edges
/// once per bucket. Requires non-negative weights.
pub struct DeltaStepping<'g, G> {
    g: &'g G,
    delta: Option<f64>,
}

impl<'g, G: Graph> DeltaStepping<'g, G> {
    /// Chooses `delta` automatically: the largest edge weight divided by the largest degree.
    pub fn new(g: &'g G) -> Self {
        DeltaStepping { g, delta: None }
    }
    pub fn with_delta(g: &'g G, delta: f64) -> Self {
        DeltaStepping {
            g,
            delta: Some(delta),
        }
    }
}

impl<G: Graph> ShortestPathAlgorithm<G> for DeltaStepping<'_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        let ix = Indexed::new(self.g);
        let s = *ix.idx.get(source).ok_or(GraphError::VertexNotFound)?;
        if ix.w.iter().any(|w| w.is_nan() || *w < 0.0) {
            return Err(GraphError::InvalidArgument(
                "negative or NaN edge weight".into(),
            ));
        }
        let max_w = ix.w.iter().copied().fold(0.0, f64::max);
        let max_deg = ix.out.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let delta = self.delta.unwrap_or(if max_w > 0.0 {
            max_w / max_deg as f64
        } else {
            1.0
        });
        if delta.is_nan() || delta <= 0.0 {
            return Err(GraphError::InvalidArgument("delta must be positive".into()));
        }
        let n = ix.n();
        let mut dist = vec![f64::INFINITY; n];
        let mut pred: Vec<Option<(usize, usize)>> = vec![None; n]; // (edge, tail)
        let mut buckets: Vec<Vec<usize>> = vec![Vec::new()];
        let bucket_of = |d: f64| (d / delta) as usize;

        macro_rules! relax {
            ($v:expr, $d:expr, $e:expr, $from:expr) => {{
                let (v, d) = ($v, $d);
                if d < dist[v] {
                    dist[v] = d;
                    pred[v] = Some(($e, $from));
                    let b = bucket_of(d);
                    if b >= buckets.len() {
                        buckets.resize_with(b + 1, Vec::new);
                    }
                    buckets[b].push(v);
                }
            }};
        }

        dist[s] = 0.0;
        buckets[0].push(s);
        let mut i = 0;
        while i < buckets.len() {
            if buckets[i].is_empty() {
                i += 1;
                continue;
            }
            let mut settled: Vec<usize> = Vec::new();
            let mut in_settled = vec![false; n];
            while !buckets[i].is_empty() {
                let req = std::mem::take(&mut buckets[i]);
                for v in req {
                    if bucket_of(dist[v]) != i {
                        continue; // stale entry, vertex moved to a lower bucket
                    }
                    if !in_settled[v] {
                        in_settled[v] = true;
                        settled.push(v);
                    }
                    for &(u, k) in ix.out[v].iter().filter(|&&(_, k)| ix.w[k] <= delta) {
                        relax!(u, dist[v] + ix.w[k], k, v);
                    }
                }
            }
            for v in settled {
                for &(u, k) in ix.out[v].iter().filter(|&&(_, k)| ix.w[k] > delta) {
                    relax!(u, dist[v] + ix.w[k], k, v);
                }
            }
            i += 1;
        }

        let mut out = SingleSourcePaths {
            source: source.clone(),
            dist: HashMap::new(),
            pred: HashMap::new(),
        };
        for v in 0..n {
            if dist[v].is_finite() {
                out.dist.insert(ix.vertex(v), dist[v]);
                if let Some((k, from)) = pred[v] {
                    out.pred
                        .insert(ix.vertex(v), (ix.es[k].clone(), ix.vertex(from)));
                }
            }
        }
        Ok(out)
    }
}
