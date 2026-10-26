use jgrapht::alg::flow::*;
use jgrapht::alg::spanning::*;
use jgrapht::generate::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

fn wug(edges: &[(u32, u32, f64)]) -> UGraph<u32> {
    let mut g = UGraph::<u32>::simple().weighted();
    for &(a, b, w) in edges {
        g.add_vertex(a);
        g.add_vertex(b);
        g.add_weighted_edge(&a, &b, w).unwrap();
    }
    g
}
fn wdg(edges: &[(u32, u32, f64)]) -> DiGraph<u32> {
    let mut g = DiGraph::<u32>::simple().weighted();
    for &(a, b, w) in edges {
        g.add_vertex(a);
        g.add_vertex(b);
        g.add_weighted_edge(&a, &b, w).unwrap();
    }
    g
}

#[test]
fn mst_algorithms_agree() {
    let g = wug(&[
        (1, 2, 4.0),
        (1, 3, 1.0),
        (2, 3, 2.0),
        (2, 4, 5.0),
        (3, 4, 8.0),
        (4, 5, 3.0),
        (6, 7, 1.0),
    ]);
    for t in [kruskal(&g), prim(&g), boruvka(&g)] {
        assert_eq!(t.weight, 1.0 + 2.0 + 5.0 + 3.0 + 1.0);
        assert_eq!(t.edges.len(), 5);
    }
    let mut rng = StdRng::seed_from_u64(3);
    for _ in 0..20 {
        let mut g = UGraph::<usize>::simple().weighted();
        Gnm {
            n: 25,
            m: 80,
            loops: false,
            rng: StdRng::seed_from_u64(rng.random()),
        }
        .generate_indexed(&mut g)
        .unwrap();
        for e in g.edges().cloned().collect::<Vec<_>>() {
            g.set_edge_weight(&e, rng.random_range(0..10) as f64)
                .unwrap();
        }
        let (k, p, b) = (kruskal(&g).weight, prim(&g).weight, boruvka(&g).weight);
        assert_eq!((k, k), (p, b));
    }
}

#[test]
fn arborescence() {
    let g = wdg(&[
        (0, 1, 5.0),
        (0, 2, 1.0),
        (2, 1, 1.0),
        (1, 3, 2.0),
        (3, 2, 1.0),
        (2, 4, 7.0),
        (3, 4, 1.0),
    ]);
    let t = minimum_arborescence(&g, &0).unwrap();
    assert_eq!(t.edges.len(), 4);
    assert_eq!(t.weight, 1.0 + 1.0 + 2.0 + 1.0);
    // cycle that must be broken: 1<->2 cheap, root enters expensively
    let g = wdg(&[(0, 1, 10.0), (0, 2, 11.0), (1, 2, 1.0), (2, 1, 1.0)]);
    assert_eq!(minimum_arborescence(&g, &0).unwrap().weight, 11.0);
    assert_eq!(maximum_arborescence(&g, &0).unwrap().weight, 21.0);
    assert!(minimum_arborescence(&wdg(&[(1, 2, 1.0), (3, 2, 1.0)]), &1).is_err());
}

fn clrs() -> DiGraph<u32> {
    wdg(&[
        (0, 1, 16.0),
        (0, 2, 13.0),
        (1, 2, 10.0),
        (2, 1, 4.0),
        (1, 3, 12.0),
        (3, 2, 9.0),
        (2, 4, 14.0),
        (4, 3, 7.0),
        (3, 5, 20.0),
        (4, 5, 4.0),
    ])
}

#[test]
fn max_flow_algorithms() {
    let g = clrs();
    for alg in [
        MaxFlowAlgorithm::EdmondsKarp,
        MaxFlowAlgorithm::Dinic,
        MaxFlowAlgorithm::PushRelabel,
    ] {
        let f = max_flow(&g, &0, &5, alg).unwrap();
        assert_eq!(f.value, 23.0, "{alg:?}");
        // flow conservation at inner vertices
        for v in 1..5u32 {
            let inflow: f64 = g
                .edges()
                .filter(|e| g.edge_target(e) == Some(&v))
                .map(|e| f.flow[e])
                .sum();
            let outflow: f64 = g
                .edges()
                .filter(|e| g.edge_source(e) == Some(&v))
                .map(|e| f.flow[e])
                .sum();
            assert!((inflow - outflow).abs() < 1e-9);
        }
        assert!(
            g.edges()
                .all(|e| f.flow[e] <= g.edge_weight(e).unwrap() + 1e-9)
        );
    }
    let cut = minimum_st_cut(&g, &0, &5).unwrap();
    assert_eq!(cut.weight, 23.0);
    assert_eq!(
        cut.cut_edges
            .iter()
            .map(|e| g.edge_weight(e).unwrap())
            .sum::<f64>(),
        23.0
    );
}

#[test]
fn flow_algorithms_agree_randomly() {
    let mut rng = StdRng::seed_from_u64(5);
    for _ in 0..20 {
        let mut g = DiGraph::<usize>::simple().weighted();
        Gnm {
            n: 12,
            m: 40,
            loops: false,
            rng: StdRng::seed_from_u64(rng.random()),
        }
        .generate_indexed(&mut g)
        .unwrap();
        for e in g.edges().cloned().collect::<Vec<_>>() {
            g.set_edge_weight(&e, rng.random_range(1..10) as f64)
                .unwrap();
        }
        let a = edmonds_karp(&g, &0, &11).unwrap().value;
        assert_eq!(a, dinic(&g, &0, &11).unwrap().value);
        assert_eq!(a, push_relabel(&g, &0, &11).unwrap().value);
    }
}

#[test]
fn gomory_hu_and_stoer_wagner() {
    let mut rng = StdRng::seed_from_u64(9);
    for _ in 0..10 {
        let mut g = UGraph::<usize>::simple().weighted();
        Gnp {
            n: 9,
            p: 0.5,
            loops: false,
            rng: StdRng::seed_from_u64(rng.random()),
        }
        .generate_indexed(&mut g)
        .unwrap();
        for e in g.edges().cloned().collect::<Vec<_>>() {
            g.set_edge_weight(&e, rng.random_range(1..8) as f64)
                .unwrap();
        }
        let tree = gomory_hu_tree(&g).unwrap();
        let mut global = f64::INFINITY;
        for a in 0..9 {
            for b in a + 1..9 {
                let expect = dinic(&g, &a, &b).unwrap().value;
                assert_eq!(tree.min_cut_value(&a, &b), Some(expect));
                global = global.min(expect);
            }
        }
        let sw = stoer_wagner(&g).unwrap();
        assert_eq!(sw.weight, global);
        assert_eq!(
            sw.cut_edges
                .iter()
                .map(|e| g.edge_weight(e).unwrap())
                .sum::<f64>(),
            global
        );
    }
}

#[test]
fn min_cost_flow_works() {
    let g = wdg(&[
        (0, 1, 0.0),
        (0, 2, 0.0),
        (1, 3, 0.0),
        (2, 3, 0.0),
        (1, 2, 0.0),
    ]);
    let cap = |e: &DefaultEdge| match e.0 {
        0 => 4.0,
        1 => 2.0,
        2 => 3.0,
        3 => 5.0,
        _ => 10.0,
    };
    let cost = |e: &DefaultEdge| match e.0 {
        0 => 1.0,
        1 => 2.0,
        2 => 3.0,
        3 => 1.0,
        _ => 1.0,
    };
    let r = min_cost_flow(&g, &0, &3, None, cap, cost).unwrap();
    assert_eq!(r.flow_value, 6.0);
    // cheapest: 0-1-2-3 costs 3, 0-1-3 costs 4, 0-2-3 costs 3: fill 0-1 (4) and 0-2 (2)
    let r2 = min_cost_flow(&g, &0, &3, Some(2.0), cap, cost).unwrap();
    assert_eq!((r2.flow_value, r2.cost), (2.0, 6.0));
}
