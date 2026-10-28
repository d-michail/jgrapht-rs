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
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

fn k_shortest_paths(c: &mut Criterion) {
    let (n, k) = (300, 50);
    let g = base_graph(n, &random_edges(n, 6_000, 17));
    let mut group = c.benchmark_group("k_shortest_paths");
    group.bench_function("yen", |b| {
        b.iter(|| {
            black_box(
                YenKShortestPath::new(&g)
                    .paths(&0, &(n - 1), k)
                    .unwrap()
                    .len(),
            )
        })
    });
    group.bench_function("bounded_pruned_yen", |b| {
        b.iter(|| {
            black_box(
                BoundedPrunedYenKShortestPath::new(&g)
                    .paths(&0, &(n - 1), k)
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
                    .all_paths(&s, &t, true, None)
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
                                .all_paths(&0, &target, true, Some(CHAIN_LEN + 5))
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
            b.iter(|| black_box(algorithm.all_paths(&0, &(N - 1), true, Some(3)).unwrap()))
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

#[derive(Clone, Copy)]
enum HamiltonianFamily {
    Path,
    Cycle,
    Complete,
    Sparse,
    StarNegative,
    ModularBridges,
    DagPositive,
}

impl HamiltonianFamily {
    const ALL: [Self; 7] = [
        Self::Path,
        Self::Cycle,
        Self::Complete,
        Self::Sparse,
        Self::StarNegative,
        Self::ModularBridges,
        Self::DagPositive,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Cycle => "cycle",
            Self::Complete => "complete",
            Self::Sparse => "sparse",
            Self::StarNegative => "star_negative",
            Self::ModularBridges => "modular_bridges",
            Self::DagPositive => "dag_positive",
        }
    }

    fn ordinal(self) -> u64 {
        match self {
            Self::Path => 0,
            Self::Cycle => 1,
            Self::Complete => 2,
            Self::Sparse => 3,
            Self::StarNegative => 4,
            Self::ModularBridges => 5,
            Self::DagPositive => 6,
        }
    }
}

enum HamiltonianGraph {
    Undirected(UGraph<usize>),
    Directed(DiGraph<usize>),
}

fn undirected_graph(n: usize) -> UGraph<usize> {
    let mut graph = UGraph::<usize>::simple();
    graph.add_vertices(0..n);
    graph
}

fn hamiltonian_graph(family: HamiltonianFamily, n: usize) -> HamiltonianGraph {
    let seed = 0xBEEF_CAFE_1234_5678u64 ^ ((n as u64) << 16) ^ family.ordinal();
    let mut random = StdRng::seed_from_u64(seed);
    match family {
        HamiltonianFamily::Path => {
            let mut graph = undirected_graph(n);
            for vertex in 0..n - 1 {
                graph.add_edge(&vertex, &(vertex + 1)).unwrap();
            }
            HamiltonianGraph::Undirected(graph)
        }
        HamiltonianFamily::Cycle => {
            let mut graph = undirected_graph(n);
            for vertex in 0..n {
                graph.add_edge(&vertex, &((vertex + 1) % n)).unwrap();
            }
            HamiltonianGraph::Undirected(graph)
        }
        HamiltonianFamily::Complete => {
            let mut graph = undirected_graph(n);
            for source in 0..n {
                for target in source + 1..n {
                    graph.add_edge(&source, &target).unwrap();
                }
            }
            HamiltonianGraph::Undirected(graph)
        }
        HamiltonianFamily::Sparse => {
            let mut graph = undirected_graph(n);
            let probability = 3.0 / n as f64;
            for source in 0..n {
                for target in source + 1..n {
                    if random.random::<f64>() < probability {
                        graph.add_edge(&source, &target).unwrap();
                    }
                }
            }
            HamiltonianGraph::Undirected(graph)
        }
        HamiltonianFamily::StarNegative => {
            let mut graph = undirected_graph(n);
            for leaf in 1..n {
                graph.add_edge(&0, &leaf).unwrap();
            }
            HamiltonianGraph::Undirected(graph)
        }
        HamiltonianFamily::ModularBridges => {
            let mut graph = undirected_graph(n);
            let mut index = 0;
            while index + 3 <= n {
                graph.add_edge(&index, &(index + 1)).unwrap();
                graph.add_edge(&(index + 1), &(index + 2)).unwrap();
                graph.add_edge(&index, &(index + 2)).unwrap();
                if index + 3 < n {
                    graph.add_edge(&(index + 2), &(index + 3)).unwrap();
                }
                index += 3;
            }
            for vertex in index..n {
                graph.add_edge(&(vertex - 1), &vertex).unwrap();
            }
            HamiltonianGraph::Undirected(graph)
        }
        HamiltonianFamily::DagPositive => {
            let mut graph = DiGraph::<usize>::simple();
            graph.add_vertices(0..n);
            for vertex in 0..n - 1 {
                graph.add_edge(&vertex, &(vertex + 1)).unwrap();
            }
            for _ in 0..(n / 3).max(1) {
                let source = random.random_range(0..(n - 2) as i32) as usize;
                let target =
                    source + 2 + random.random_range(0..(n - source - 2).max(1) as i32) as usize;
                graph.add_edge(&source, &target.min(n - 1)).unwrap();
            }
            HamiltonianGraph::Directed(graph)
        }
    }
}

fn hamiltonian_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("hamiltonian_path");
    for n in [8, 12, 16] {
        for family in HamiltonianFamily::ALL {
            let graph = hamiltonian_graph(family, n);
            group.bench_function(
                BenchmarkId::new(format!("{}/backtracking", family.name()), n),
                |b| {
                    b.iter(|| match &graph {
                        HamiltonianGraph::Undirected(graph) => {
                            black_box(BacktrackingHamiltonianPath::new().path(graph).unwrap())
                        }
                        HamiltonianGraph::Directed(graph) => {
                            black_box(BacktrackingHamiltonianPath::new().path(graph).unwrap())
                        }
                    })
                },
            );
            group.bench_function(
                BenchmarkId::new(format!("{}/held_karp", family.name()), n),
                |b| {
                    b.iter(|| match &graph {
                        HamiltonianGraph::Undirected(graph) => {
                            black_box(HeldKarpHamiltonianPath::new().path(graph).unwrap())
                        }
                        HamiltonianGraph::Directed(graph) => {
                            black_box(HeldKarpHamiltonianPath::new().path(graph).unwrap())
                        }
                    })
                },
            );
        }
    }
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
                    .matching()
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
