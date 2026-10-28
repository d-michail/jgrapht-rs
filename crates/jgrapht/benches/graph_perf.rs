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

//! Port of `perf/graph/GraphPerformanceTest`: build a random directed weighted graph
//! (1000 vertices, 100000 edges), then run Dijkstra, Edmonds-Karp and a strong-connectivity test.

mod common;

use common::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use jgrapht::alg::connectivity::is_strongly_connected;
use jgrapht::alg::flow::EdmondsKarp;
use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
use jgrapht::{Graph, GraphExt, OutgoingGraph};

const N: usize = 1000;
const M: usize = 100_000;

fn graph_performance(c: &mut Criterion) {
    let edges = random_edges(N, M, 1446523573696201013);

    c.bench_function("graph/generate", |b| {
        b.iter(|| black_box(base_graph(N, &edges)))
    });

    let g = base_graph(N, &edges);
    c.bench_function("graph/dijkstra_path", |b| {
        b.iter(|| black_box(Dijkstra::new(&g).path_weight(&0, &(N - 1)).unwrap()))
    });
    c.bench_function("graph/edmonds_karp", |b| {
        b.iter(|| {
            black_box(
                EdmondsKarp::new(&g)
                    .unwrap()
                    .maximum_flow_value(&0, &(N - 1))
                    .unwrap(),
            )
        })
    });
    c.bench_function("graph/is_strongly_connected", |b| {
        b.iter(|| black_box(is_strongly_connected(&g)))
    });

    // Micro-benchmarks of the edge lookups, to see what a hash lookup per query costs against
    // the sparse graph, whose endpoints are array reads.
    let s = sparse_graph(N, &edges);
    let edge_list: Vec<_> = g.edges_cloned().collect();
    c.bench_function("graph/base_edge_endpoints_and_weight", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for e in &edge_list {
                let (u, v) = g.edge_endpoints(e).unwrap();
                sum += g.edge_weight(e).unwrap() + (*u + *v) as f64;
            }
            black_box(sum)
        })
    });
    c.bench_function("graph/sparse_edge_endpoints_and_weight", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for e in 0..M {
                let (u, v) = s.edge_endpoints(&e).unwrap();
                sum += s.edge_weight(&e).unwrap() + (*u + *v) as f64;
            }
            black_box(sum)
        })
    });
    c.bench_function("graph/base_out_edges_scan", |b| {
        b.iter(|| {
            let mut sum = 0usize;
            for v in 0..N {
                for e in g.out_edges_of(&v) {
                    sum += *g.edge_target(e).unwrap();
                }
            }
            black_box(sum)
        })
    });
    c.bench_function("graph/sparse_out_edges_scan", |b| {
        b.iter(|| {
            let mut sum = 0usize;
            for v in 0..N {
                for e in s.out_edges_of(&v) {
                    sum += *s.edge_target(e).unwrap();
                }
            }
            black_box(sum)
        })
    });
    c.bench_function("graph/base_edge_between_lookup", |b| {
        b.iter(|| {
            let mut found = 0usize;
            for &(u, v, _) in edges.iter().take(10_000) {
                found += g.edges_between(&u, &v).count();
            }
            black_box(found)
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = graph_performance
}
criterion_main!(benches);
