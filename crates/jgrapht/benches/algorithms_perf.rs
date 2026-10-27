//! Ports of the flow (`MaximumFlowAlgorithmPerformanceTest`), spanning-tree
//! (`MinimumSpanningTreePerformanceTest`) and clique (`MaximalCliqueEnumerationPerformanceTest`)
//! performance tests on deterministic random graphs.

mod common;

use common::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use jgrapht::alg::clique::maximal_cliques;
use jgrapht::alg::flow::{DinicMFImpl, EdmondsKarpMFImpl, MaximumFlowAlgorithm, PushRelabelMFImpl};
use jgrapht::alg::spanning::{kruskal, prim};
use jgrapht::*;

fn maximum_flow(c: &mut Criterion) {
    let (n, m) = (500, 20_000);
    let g = base_graph(n, &random_edges(n, m, 7));
    let mut group = c.benchmark_group("max_flow");
    group.bench_function("edmonds_karp", |b| {
        b.iter(|| black_box(EdmondsKarpMFImpl::new(&g).unwrap().maximum_flow_value(&0, &(n - 1)).unwrap()))
    });
    group.bench_function("dinic", |b| {
        b.iter(|| black_box(DinicMFImpl::new(&g).unwrap().maximum_flow_value(&0, &(n - 1)).unwrap()))
    });
    group.bench_function("push_relabel", |b| {
        b.iter(|| black_box(PushRelabelMFImpl::new(&g).unwrap().maximum_flow_value(&0, &(n - 1)).unwrap()))
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
