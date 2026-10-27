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
use jgrapht::alg::flow::EdmondsKarpMFImpl;
use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};

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
                EdmondsKarpMFImpl::new(&g)
                    .unwrap()
                    .calculate_maximum_flow(&0, &(N - 1))
                    .unwrap(),
            )
        })
    });
    c.bench_function("graph/is_strongly_connected", |b| {
        b.iter(|| black_box(is_strongly_connected(&g)))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = graph_performance
}
criterion_main!(benches);
