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

//! Port of `perf/shortestpath/DijkstraShortestPathPerformanceTest` (250 vertices, edge
//! probability 0.3, all-pairs queries), plus the Phase 3b comparison of `BaseGraph` and the
//! CSR sparse graph on single-source Dijkstra and BFS. Also ports
//! `DijkstraManyToManyShortestPathsGetPathsPerformance`.

mod common;

use common::*;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use jgrapht::alg::shortestpath::{
    BfsShortestPath, Dijkstra, DijkstraManyToManyShortestPaths, ShortestPathAlgorithm,
};
use jgrapht::generate::{GnpRandomGraphGenerator, GraphGenerator};
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

fn all_pairs_dijkstra(c: &mut Criterion) {
    let n = 250;
    let mut rng = Lcg(13);
    let mut edges = Vec::new();
    for u in 0..n {
        for v in 0..n {
            if u != v && rng.unit() < 0.3 {
                edges.push((u, v, rng.unit()));
            }
        }
    }
    let g = base_graph(n, &edges);
    c.bench_function("dijkstra/all_pairs_250", |b| {
        b.iter(|| {
            let dijkstra = Dijkstra::new(&g);
            for s in 0..n {
                black_box(dijkstra.single_source(&s).unwrap());
            }
        })
    });
}

fn base_vs_sparse(c: &mut Criterion) {
    let (n, m) = (20_000, 200_000);
    let edges = random_edges(n, m, 42);
    let base = base_graph(n, &edges);
    let sparse = sparse_graph(n, &edges);

    let mut group = c.benchmark_group("base_vs_sparse");
    group.bench_function("dijkstra/base", |b| {
        b.iter(|| black_box(Dijkstra::new(&base).single_source(&0).unwrap()))
    });
    group.bench_function("dijkstra/sparse", |b| {
        b.iter(|| black_box(Dijkstra::new(&sparse).single_source(&0).unwrap()))
    });
    group.bench_function("bfs/base", |b| {
        b.iter(|| black_box(BfsShortestPath::new(&base).single_source(&0).unwrap()))
    });
    group.bench_function("bfs/sparse", |b| {
        b.iter(|| black_box(BfsShortestPath::new(&sparse).single_source(&0).unwrap()))
    });
    group.finish();
}

fn many_to_many_state(n: usize) -> (DiGraph<usize>, Vec<usize>) {
    const SEED: i64 = 19;
    let mut graph = DiGraph::<usize>::simple()
        .weighted()
        .with_vertex_factory(|id| id as usize);
    GnpRandomGraphGenerator::new(n, 0.1, StdRng::seed_from_u64(SEED as u64), false)
        .unwrap()
        .generate_default(&mut graph)
        .unwrap();

    for vertex in 0..n - 1 {
        if !graph.contains_edge_between(&vertex, &(vertex + 1)) {
            graph.add_edge(&vertex, &(vertex + 1)).unwrap();
        }
    }

    let mut random = StdRng::seed_from_u64(SEED as u64);
    let edges: Vec<_> = graph.edges().cloned().collect();
    for edge in edges {
        graph
            .set_edge_weight(&edge, 1.0 + random.random_range(0..1000) as f64)
            .unwrap();
    }

    let mut sources = Vec::with_capacity(5);
    while sources.len() < 5 {
        let source = random.random_range(0..n as i32) as usize;
        if !sources.contains(&source) {
            sources.push(source);
        }
    }
    (graph, sources)
}

fn many_to_many_get_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("dijkstra_many_to_many_get_paths");
    for n in [50, 100, 200] {
        let (graph, sources) = many_to_many_state(n);
        let algorithm = DijkstraManyToManyShortestPaths::new(&graph);

        group.bench_with_input(BenchmarkId::new("base_class_loop", n), &n, |b, _| {
            b.iter(|| {
                let mut result = Vec::with_capacity(sources.len());
                for source in &sources {
                    let paths: Vec<_> = graph
                        .vertices()
                        .map(|target| {
                            algorithm
                                .get_many_to_many_paths([*source], [*target])
                                .unwrap()
                                .path(source, target)
                                .cloned()
                        })
                        .collect();
                    result.push(paths);
                }
                black_box(result)
            })
        });
        group.bench_with_input(
            BenchmarkId::new("optimized_single_source", n),
            &n,
            |b, _| {
                b.iter(|| {
                    black_box(
                        sources
                            .iter()
                            .map(|source| algorithm.single_source(source).unwrap())
                            .collect::<Vec<_>>(),
                    )
                })
            },
        );
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = all_pairs_dijkstra, base_vs_sparse, many_to_many_get_paths
}
criterion_main!(benches);
