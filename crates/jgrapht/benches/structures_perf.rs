//! Ports of the LCA (`LowestCommonAncestorAlgorithmPerformanceTest`), tree dynamic connectivity
//! (`TreeDynamicConnectivityPerformanceTest`), matching (`PathGrowingWeightedMatching...`,
//! `MaximumCardinalityBipartiteMatching...`) and DAG Hamiltonian path performance tests.

mod common;

use common::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use jgrapht::alg::interfaces::LowestCommonAncestorAlgorithm;
use jgrapht::alg::lca::{BinaryLiftingLcaFinder, EulerTourRmqLcaFinder, HeavyPathLcaFinder};
use jgrapht::alg::matching::{
    HopcroftKarpMaximumCardinalityBipartiteMatching, MatchingAlgorithm, PathGrowingWeightedMatching,
};
use jgrapht::alg::tour::{DagHamiltonianPath, HamiltonianPathAlgorithm};
use jgrapht::alg::tree_connectivity::TreeDynamicConnectivity;
use jgrapht::*;

/// Random recursive tree (vertex `i` hangs under a random smaller vertex), directed away from 0.
fn random_tree(n: usize, seed: u64) -> DiGraph<usize> {
    let mut rng = Lcg(seed);
    let mut g = DiGraph::<usize>::simple();
    g.add_vertices(0..n);
    for v in 1..n {
        g.add_edge(&(rng.next(v as u64) as usize), &v).unwrap();
    }
    g
}

fn lca(c: &mut Criterion) {
    let n = 20_000;
    let tree = random_tree(n, 5);
    let mut rng = Lcg(6);
    let queries: Vec<(usize, usize)> = (0..20_000)
        .map(|_| (rng.next(n as u64) as usize, rng.next(n as u64) as usize))
        .collect();

    let mut group = c.benchmark_group("lca");
    group.bench_function("binary_lifting", |b| {
        b.iter(|| {
            let f = BinaryLiftingLcaFinder::new(&tree, 0).unwrap();
            for (u, v) in &queries {
                black_box(f.lca(u, v).unwrap());
            }
        })
    });
    group.bench_function("euler_tour_rmq", |b| {
        b.iter(|| {
            let f = EulerTourRmqLcaFinder::new(&tree, 0).unwrap();
            for (u, v) in &queries {
                black_box(f.lca(u, v).unwrap());
            }
        })
    });
    group.bench_function("heavy_path", |b| {
        b.iter(|| {
            let f = HeavyPathLcaFinder::new(&tree, 0).unwrap();
            for (u, v) in &queries {
                black_box(f.lca(u, v).unwrap());
            }
        })
    });
    group.finish();
}

fn tree_connectivity(c: &mut Criterion) {
    let n = 50_000;
    let mut rng = Lcg(9);
    let links: Vec<(usize, usize)> = (1..n).map(|v| (rng.next(v as u64) as usize, v)).collect();
    c.bench_function("tree_connectivity/link_connected_cut", |b| {
        b.iter(|| {
            let mut t = TreeDynamicConnectivity::new();
            for v in 0..n {
                t.add(v);
            }
            for &(u, v) in &links {
                t.link(u, v);
            }
            for &(u, v) in links.iter().step_by(10) {
                black_box(t.connected(&u, &v));
                t.cut(&u, &v).unwrap();
                black_box(t.connected(&u, &v));
            }
        })
    });
}

fn matching(c: &mut Criterion) {
    let n = 2_000;
    let edges = random_edges(n, 20_000, 21);
    let mut g = UGraph::<usize>::simple().weighted();
    g.add_vertices(0..n);
    for &(u, v, w) in &edges {
        let _ = g.add_weighted_edge(&u, &v, w);
    }
    c.bench_function("matching/path_growing", |b| {
        b.iter(|| black_box(PathGrowingWeightedMatching::new(&g).unwrap().get_matching().unwrap().weight()))
    });

    // bipartite: left 0..n/2, right n/2..n
    let half = n / 2;
    let mut bip = UGraph::<usize>::simple();
    bip.add_vertices(0..n);
    for &(u, v, _) in &edges {
        let (l, r) = (u % half, half + v % half);
        let _ = bip.add_edge(&l, &r);
    }
    c.bench_function("matching/hopcroft_karp", |b| {
        b.iter(|| {
            black_box(
                HopcroftKarpMaximumCardinalityBipartiteMatching::new(&bip, 0..half, half..n)
                    .unwrap()
                    .get_matching()
                    .unwrap()
                    .edges()
                    .len(),
            )
        })
    });
}

fn dag_hamiltonian_path(c: &mut Criterion) {
    let n = 2_000;
    let mut g = DiGraph::<usize>::dag();
    g.add_vertices(0..n);
    for v in 1..n {
        g.add_edge(&(v - 1), &v).unwrap();
        if v > 2 {
            let _ = g.add_edge(&(v - 2), &v);
        }
    }
    c.bench_function("tour/dag_hamiltonian_path", |b| {
        b.iter(|| black_box(DagHamiltonianPath::new().get_path(&g).unwrap()))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = lca, tree_connectivity, matching, dag_hamiltonian_path
}
criterion_main!(benches);
