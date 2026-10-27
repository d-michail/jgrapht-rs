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
use jgrapht::util::java::JavaRandom;
use jgrapht::*;

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
    GnpRandomGraphGenerator::new(n, 0.1, JavaRandom::new(SEED), false)
        .unwrap()
        .generate_default(&mut graph)
        .unwrap();

    for vertex in 0..n - 1 {
        if !graph.contains_edge_between(&vertex, &(vertex + 1)) {
            graph.add_edge(&vertex, &(vertex + 1)).unwrap();
        }
    }

    let mut random = JavaRandom::new(SEED);
    let edges: Vec<_> = graph.edges().cloned().collect();
    for edge in edges {
        graph
            .set_edge_weight(&edge, 1.0 + random.next_int_bound(1000) as f64)
            .unwrap();
    }

    let mut sources = Vec::with_capacity(5);
    while sources.len() < 5 {
        let source = random.next_int_bound(n as i32) as usize;
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
