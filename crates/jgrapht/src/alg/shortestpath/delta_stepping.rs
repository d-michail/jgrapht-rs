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

/// Implementation of a single-source shortest path algorithm: the delta-stepping algorithm.
/// The algorithm computes single source shortest paths in a graphs with non-negative edge weights.
/// When using multiple threads, this implementation typically outperforms
/// `DijkstraShortestPath` and `BellmanFordShortestPath`.
///
/// The delta-stepping algorithm is described in the paper: U. Meyer, P. Sanders, $\Delta$-stepping:
/// a parallelizable shortest path algorithm, Journal of Algorithms, Volume 49, Issue 1, 2003, Pages
/// 114-152, ISSN 0196-6774.
///
/// The $\Delta$-stepping algorithm takes as input a weighted graph $G(V,E)$, a source node $s$ and a
/// parameter $\Delta > 0$. Let $tent\[v\]$ be the best known shortest distance from $s$ to vertex
/// $v\in V$. At the start of the algorithm, $tent\[s\]=0$, $tent\[v\]=\infty$ for $v\in V\setminus
/// \{s\}$. The algorithm partitions vertices in a series of buckets $B=(B_0, B_1, B_2, \dots)$,
/// where a vertex $v\in V$ is placed in bucket $B_{\lfloor\frac{tent\[v\]}{\Delta}\rfloor}$. During
/// the execution of the algorithm, vertices in bucket $B_i$, for $i=0,1,2,\dots$, are removed
/// one-by-one. For each removed vertex $v$, and for all its outgoing edges $(v,w)$, the algorithm
/// checks whether $tent\[v\]+c(v,w) < tent\[w\]$. If so, $w$ is removed from its current bucket,
/// $tent\[w\]$ is updated ($tent\[w\]=tent\[v\]+c(v,w)$), and $w$ is placed into bucket
/// $B_{\lfloor\frac{tent\[w\]}{\Delta}\rfloor}$. Parallelism is achieved by processing all vertices
/// belonging to the same bucket concurrently. The algorithm terminates when all buckets are empty.
/// At this stage the array $tent$ contains the minimal cost from $s$ to every vertex $v \in V$. For
/// a more detailed description of the algorithm, refer to the aforementioned paper.
///
/// For a given graph $G(V,E)$ and parameter $\Delta$, let a $\Delta$-path be a path of total weight
/// at most $\Delta$ with no repeated edges. The time complexity of the algorithm is $O(\frac{(|V| +
/// |E| + n_{\Delta} + m_{\Delta})}{p} + \frac{L}{\Delta}\cdot d\cdot l_{\Delta}\cdot \log n)$, where
///
/// - $n_{\Delta}$ - number of vertex pairs $(u,v)$, where $u$ and $v$ are connected by some
/// $\Delta$-path.
/// - $m_{\Delta}$ - number of vertex triples $(u,v^{\prime},v)$, where $u$ and $v^{\prime}$ are
/// connected by some $\Delta$-path and edge $(v^{\prime},v)$ has weight at most $\Delta$.
/// - $L$ - maximum weight of a shortest path from selected source to any sink.
/// - $d$ - maximum vertex degree.
/// - $l_{\Delta}$ - maximum number of edges in a $\Delta$-path $+1$.
///
/// # Rust notes
///
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
            return Err(GraphError::InvalidWeight(crate::WeightProblem::NegativeOrNaN));
        }
        let max_w = ix.w.iter().copied().fold(0.0, f64::max);
        let max_deg = ix.out.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let delta = self.delta.unwrap_or(if max_w > 0.0 {
            max_w / max_deg as f64
        } else {
            1.0
        });
        if delta.is_nan() || delta <= 0.0 {
            return Err(GraphError::must_be_positive("delta"));
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
