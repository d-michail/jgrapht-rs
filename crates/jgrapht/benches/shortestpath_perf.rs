//! Port of `perf/shortestpath/DijkstraShortestPathPerformanceTest` (250 vertices, edge
//! probability 0.3, all-pairs queries), plus the Phase 3b comparison of `BaseGraph` and the
//! CSR sparse graph on single-source Dijkstra and BFS.

mod common;

use common::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use jgrapht::alg::shortestpath::{BfsShortestPath, Dijkstra, ShortestPathAlgorithm};

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

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = all_pairs_dijkstra, base_vs_sparse
}
criterion_main!(benches);
