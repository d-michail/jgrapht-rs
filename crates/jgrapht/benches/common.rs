// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

//! Shared random-graph construction for the benchmarks (deterministic, no generator coupling).
#![allow(dead_code)]

use jgrapht::graph::sparse::SparseIntDirectedWeightedGraph;
use jgrapht::*;

pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self, bound: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % bound
    }

    pub fn unit(&mut self) -> f64 {
        self.next(1 << 30) as f64 / (1u64 << 30) as f64
    }
}

/// `m` distinct random directed edges on `n` vertices, without loops, with weights in `[0, 1)`.
pub fn random_edges(n: usize, m: usize, seed: u64) -> Vec<(usize, usize, f64)> {
    let mut rng = Lcg(seed);
    let mut seen = std::collections::HashSet::new();
    let mut edges = Vec::with_capacity(m);
    while edges.len() < m {
        let (u, v) = (rng.next(n as u64) as usize, rng.next(n as u64) as usize);
        if u != v && seen.insert((u, v)) {
            edges.push((u, v, rng.unit()));
        }
    }
    edges
}

pub fn base_graph(n: usize, edges: &[(usize, usize, f64)]) -> DiGraph<usize> {
    let mut g = DiGraph::<usize>::simple().weighted();
    g.add_vertices(0..n);
    for &(u, v, w) in edges {
        g.add_weighted_edge(&u, &v, w).unwrap();
    }
    g
}

pub fn sparse_graph(n: usize, edges: &[(usize, usize, f64)]) -> SparseIntDirectedWeightedGraph {
    SparseIntDirectedWeightedGraph::new(n, edges).unwrap()
}
