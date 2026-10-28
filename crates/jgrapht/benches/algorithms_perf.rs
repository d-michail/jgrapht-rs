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

//! Ports of the flow (`MaximumFlowAlgorithmPerformanceTest`), spanning-tree
//! (`MinimumSpanningTreePerformanceTest`) and clique (`MaximalCliqueEnumerationPerformanceTest`)
//! performance tests on deterministic random graphs.

mod common;

use common::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use jgrapht::alg::clique::maximal_cliques;
use jgrapht::alg::flow::{Dinic, EdmondsKarp, MaximumFlowValueAlgorithm, PushRelabel};
use jgrapht::alg::spanning::{kruskal, prim};
use jgrapht::*;

fn maximum_flow(c: &mut Criterion) {
    let (n, m) = (500, 20_000);
    let g = base_graph(n, &random_edges(n, m, 7));
    let mut group = c.benchmark_group("max_flow");
    group.bench_function("edmonds_karp", |b| {
        b.iter(|| {
            black_box(
                EdmondsKarp::new(&g)
                    .unwrap()
                    .maximum_flow_value(&0, &(n - 1))
                    .unwrap(),
            )
        })
    });
    group.bench_function("dinic", |b| {
        b.iter(|| {
            black_box(
                Dinic::new(&g)
                    .unwrap()
                    .maximum_flow_value(&0, &(n - 1))
                    .unwrap(),
            )
        })
    });
    group.bench_function("push_relabel", |b| {
        b.iter(|| {
            black_box(
                PushRelabel::new(&g)
                    .unwrap()
                    .maximum_flow_value(&0, &(n - 1))
                    .unwrap(),
            )
        })
    });
    group.finish();
}

fn undirected(n: usize, edges: &[(usize, usize, f64)]) -> UGraph<usize> {
    let mut g = UGraph::<usize>::simple().weighted();
    g.add_vertices(0..n);
    for &(u, v, w) in edges {
        // the random edges are directed, so a reversed duplicate is skipped by the simple graph
        let _ = g.add_weighted_edge(&u, &v, w);
    }
    g
}

fn spanning_tree(c: &mut Criterion) {
    let (n, m) = (5_000, 50_000);
    let g = undirected(n, &random_edges(n, m, 11));
    let mut group = c.benchmark_group("mst");
    group.bench_function("kruskal", |b| b.iter(|| black_box(kruskal(&g).weight)));
    group.bench_function("prim", |b| b.iter(|| black_box(prim(&g).weight)));
    group.finish();
}

fn cliques(c: &mut Criterion) {
    let n = 80;
    let mut rng = Lcg(3);
    let mut g = UGraph::<usize>::simple();
    g.add_vertices(0..n);
    for u in 0..n {
        for v in u + 1..n {
            if rng.unit() < 0.3 {
                g.add_edge(&u, &v).unwrap();
            }
        }
    }
    c.bench_function("clique/maximal_cliques_gnp_80", |b| {
        b.iter(|| black_box(maximal_cliques(&g).len()))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = maximum_flow, spanning_tree, cliques
}
criterion_main!(benches);
