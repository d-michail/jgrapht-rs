//! Ports of the k-shortest-path (`KShortestPathsPerformance`, `BoundedPrunedYen...`), delta
//! stepping, `AllDirectedPaths` (including sandwich pruning), DAG, Hamiltonian-path and Blossom V
//! performance tests.

mod common;

use common::*;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use jgrapht::alg::matching::MatchingAlgorithm;
use jgrapht::alg::matching::blossom::KolmogorovWeightedPerfectMatching;
use jgrapht::alg::shortestpath::{
    AllDirectedPaths, BoundedPrunedYenKShortestPath, DeltaStepping, Dijkstra,
    EppsteinKShortestPath, ParallelDeltaSteppingShortestPath, ShortestPathAlgorithm,
    YenKShortestPath,
};
use jgrapht::alg::tour::{
    BacktrackingHamiltonianPath, HamiltonianPathAlgorithm, HeldKarpHamiltonianPath,
};
use jgrapht::traverse::topological_sort;
use jgrapht::*;

fn k_shortest_paths(c: &mut Criterion) {
    let (n, k) = (300, 50);
    let g = base_graph(n, &random_edges(n, 6_000, 17));
    let mut group = c.benchmark_group("k_shortest_paths");
    group.bench_function("yen", |b| {
        b.iter(|| {
            black_box(
                YenKShortestPath::new(&g)
                    .get_paths(&0, &(n - 1), k)
                    .unwrap()
                    .len(),
            )
        })
    });
    group.bench_function("bounded_pruned_yen", |b| {
        b.iter(|| {
            black_box(
                BoundedPrunedYenKShortestPath::new(&g)
                    .get_paths(&0, &(n - 1), k)
                    .unwrap()
                    .len(),
            )
        })
    });
    group.bench_function("eppstein", |b| {
        b.iter(|| {
            black_box(
                EppsteinKShortestPath::new(&g)
                    .paths(&0, &(n - 1), k)
                    .unwrap()
                    .len(),
            )
        })
    });
    group.finish();
}

fn delta_stepping(c: &mut Criterion) {
    let n = 20_000;
    let g = base_graph(n, &random_edges(n, 200_000, 31));
    let mut group = c.benchmark_group("single_source");
    group.bench_function("dijkstra", |b| {
        b.iter(|| black_box(Dijkstra::new(&g).single_source(&0).unwrap()))
    });
    group.bench_function("delta_stepping", |b| {
        b.iter(|| black_box(DeltaStepping::new(&g).single_source(&0).unwrap()))
    });
    group.bench_function("parallel_delta_stepping", |b| {
        b.iter(|| {
            black_box(
                ParallelDeltaSteppingShortestPath::new(&g)
                    .single_source(&0)
                    .unwrap(),
            )
        })
    });
    group.finish();
}

fn all_directed_paths(c: &mut Criterion) {
    // layered DAG: every layer is fully connected to the next one
    let (layers, width) = (8, 4);
    let mut g = DiGraph::<usize>::simple();
    g.add_vertices(0..layers * width + 2);
    let (s, t) = (layers * width, layers * width + 1);
    for w in 0..width {
        g.add_edge(&s, &w).unwrap();
        g.add_edge(&((layers - 1) * width + w), &t).unwrap();
    }
    for l in 0..layers - 1 {
        for a in 0..width {
            for b in 0..width {
                g.add_edge(&(l * width + a), &((l + 1) * width + b))
                    .unwrap();
            }
        }
    }
    c.bench_function("all_directed_paths/layered_dag", |b| {
        b.iter(|| {
            black_box(
                AllDirectedPaths::new(&g)
                    .get_all_paths(&s, &t, true, None)
                    .unwrap()
                    .len(),
            )
        })
    });
}

fn sandwich_prune(c: &mut Criterion) {
    let mut group = c.benchmark_group("all_directed_paths/sandwich_prune");

    const CHAIN_LEN: usize = 20;
    for garden_size in [500, 1_000, 2_000] {
        let mut graph = DiGraph::<usize>::simple();
        graph.add_vertices(0..CHAIN_LEN + garden_size);
        for vertex in 0..CHAIN_LEN - 1 {
            graph.add_edge(&vertex, &(vertex + 1)).unwrap();
        }
        let target = CHAIN_LEN - 1;
        for garden_vertex in CHAIN_LEN..CHAIN_LEN + garden_size {
            graph.add_edge(&garden_vertex, &target).unwrap();
        }
        for forward_pruning in [false, true] {
            let mut algorithm = AllDirectedPaths::new(&graph);
            algorithm.set_forward_pruning(forward_pruning);
            group.bench_with_input(
                BenchmarkId::new(format!("win/pruning={forward_pruning}"), garden_size),
                &garden_size,
                |b, _| {
                    b.iter(|| {
                        black_box(
                            algorithm
                                .get_all_paths(&0, &target, true, Some(CHAIN_LEN + 5))
                                .unwrap(),
                        )
                    })
                },
            );
        }
    }

    const N: usize = 20;
    let mut graph = DiGraph::<usize>::simple();
    graph.add_vertices(0..N);
    for source in 0..N {
        for target in 0..N {
            if source != target {
                graph.add_edge(&source, &target).unwrap();
            }
        }
    }
    for forward_pruning in [false, true] {
        let mut algorithm = AllDirectedPaths::new(&graph);
        algorithm.set_forward_pruning(forward_pruning);
        group.bench_function(format!("loss/pruning={forward_pruning}"), |b| {
            b.iter(|| {
                black_box(
                    algorithm
                        .get_all_paths(&0, &(N - 1), true, Some(3))
                        .unwrap(),
                )
            })
        });
    }
    group.finish();
}

fn dag(c: &mut Criterion) {
    let n = 5_000;
    c.bench_function("dag/build_and_topological_sort", |b| {
        b.iter(|| {
            let mut g = DiGraph::<usize>::dag();
            g.add_vertices(0..n);
            let mut rng = Lcg(77);
            for v in 1..n {
                let u = rng.next(v as u64) as usize;
                g.add_edge(&u, &v).unwrap();
            }
            black_box(topological_sort(&g).map(|o| o.len()))
        })
    });
}

fn hamiltonian_path(c: &mut Criterion) {
    let n = 12;
    let mut rng = Lcg(55);
    let mut g = DiGraph::<usize>::simple();
    g.add_vertices(0..n);
    for u in 0..n {
        for v in 0..n {
            if u != v && rng.unit() < 0.5 {
                g.add_edge(&u, &v).unwrap();
            }
        }
    }
    let mut group = c.benchmark_group("hamiltonian_path");
    group.bench_function("held_karp", |b| {
        b.iter(|| black_box(HeldKarpHamiltonianPath::new().get_path(&g).unwrap()))
    });
    group.bench_function("backtracking", |b| {
        b.iter(|| black_box(BacktrackingHamiltonianPath::new().get_path(&g).unwrap()))
    });
    group.finish();
}

fn blossom_v(c: &mut Criterion) {
    let n = 400;
    let mut rng = Lcg(91);
    let mut g = UGraph::<usize>::simple().weighted();
    g.add_vertices(0..n);
    for u in 0..n {
        for v in u + 1..n {
            if rng.unit() < 0.1 || v == u + 1 {
                g.add_weighted_edge(&u, &v, 1.0 + rng.next(1000) as f64)
                    .unwrap();
            }
        }
    }
    c.bench_function("matching/blossom_v_perfect", |b| {
        b.iter(|| {
            black_box(
                KolmogorovWeightedPerfectMatching::new(&g)
                    .unwrap()
                    .get_matching()
                    .unwrap()
                    .weight(),
            )
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = k_shortest_paths, delta_stepping, all_directed_paths, sandwich_prune, dag, hamiltonian_path, blossom_v
}
criterion_main!(benches);
