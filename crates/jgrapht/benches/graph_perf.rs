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
