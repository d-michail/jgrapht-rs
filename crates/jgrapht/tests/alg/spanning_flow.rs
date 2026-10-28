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

use jgrapht::alg::flow::*;
use jgrapht::alg::spanning::*;
use jgrapht::alg::stoer_wagner::StoerWagnerMinimumCut;
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
        GnmRandomGraphGenerator::new(25, 80, StdRng::seed_from_u64(rng.random()), false, false)
            .unwrap()
            .generate_indexed(&mut g)
            .unwrap();
        for e in g.edges_cloned().collect::<Vec<_>>() {
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
    let flows = [
        EdmondsKarpMFImpl::new(&g)
            .unwrap()
            .maximum_flow(&0, &5)
            .unwrap(),
        DinicMFImpl::new(&g).unwrap().maximum_flow(&0, &5).unwrap(),
        PushRelabelMFImpl::new(&g)
            .unwrap()
            .maximum_flow(&0, &5)
            .unwrap(),
    ];
    for f in flows {
        assert_eq!(f.value(), 23.0);
        // flow conservation at inner vertices
        for v in 1..5u32 {
            let inflow: f64 = g
                .edges()
                .filter(|e| g.edge_target(e) == Some(&v))
                .map(|e| f.flow(e).unwrap())
                .sum();
            let outflow: f64 = g
                .edges()
                .filter(|e| g.edge_source(e) == Some(&v))
                .map(|e| f.flow(e).unwrap())
                .sum();
            assert!((inflow - outflow).abs() < 1e-9);
        }
        assert!(
            g.edges()
                .all(|e| f.flow(e).unwrap() <= g.edge_weight(e).unwrap() + 1e-9)
        );
    }
    let mut cut = PushRelabelMFImpl::new(&g).unwrap();
    let min_cut = cut.minimum_cut(&0, &5).unwrap();
    assert_eq!(min_cut.capacity(), 23.0);
    assert_eq!(
        min_cut
            .cut_edges()
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
        GnmRandomGraphGenerator::new(12, 40, StdRng::seed_from_u64(rng.random()), false, false)
            .unwrap()
            .generate_indexed(&mut g)
            .unwrap();
        for e in g.edges_cloned().collect::<Vec<_>>() {
            g.set_edge_weight(&e, rng.random_range(1..10) as f64)
                .unwrap();
        }
        let a = EdmondsKarpMFImpl::new(&g)
            .unwrap()
            .maximum_flow_value(&0, &11)
            .unwrap();
        assert_eq!(
            a,
            DinicMFImpl::new(&g)
                .unwrap()
                .maximum_flow_value(&0, &11)
                .unwrap()
        );
        assert_eq!(
            a,
            PushRelabelMFImpl::new(&g)
                .unwrap()
                .maximum_flow_value(&0, &11)
                .unwrap()
        );
    }
}

#[test]
fn gomory_hu_and_stoer_wagner() {
    let mut rng = StdRng::seed_from_u64(9);
    for _ in 0..10 {
        let mut g = UGraph::<usize>::simple().weighted();
        GnpRandomGraphGenerator::new(9, 0.5, StdRng::seed_from_u64(rng.random()), false)
            .unwrap()
            .generate_indexed(&mut g)
            .unwrap();
        for e in g.edges_cloned().collect::<Vec<_>>() {
            g.set_edge_weight(&e, rng.random_range(1..8) as f64)
                .unwrap();
        }
        let mut tree = GusfieldEquivalentFlowTree::new(&g).unwrap();
        let mut dinic = DinicMFImpl::new(&g).unwrap();
        let mut global = f64::INFINITY;
        for a in 0..9 {
            for b in a + 1..9 {
                let expect = dinic.maximum_flow_value(&a, &b).unwrap();
                assert_eq!(tree.maximum_flow_value(&a, &b).unwrap(), expect);
                global = global.min(expect);
            }
        }
        let sw = StoerWagnerMinimumCut::new(&g).unwrap();
        assert_eq!(sw.min_cut_weight(), global);
        let side = sw.min_cut();
        assert_eq!(
            g.edges()
                .filter(|e| {
                    let (u, v) = g.edge_endpoints(e).unwrap();
                    side.contains(u) != side.contains(v)
                })
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
    // edges in insertion order: 0-1, 0-2, 1-3, 2-3, 1-2
    let order: Vec<DefaultEdge> = g.edges_cloned().collect();
    let pos = |e: &DefaultEdge| order.iter().position(|x| x == e).unwrap();
    let cap = |e: &DefaultEdge| [4, 2, 3, 5, 10][pos(e)];
    let cost = |e: &DefaultEdge| [1.0, 2.0, 3.0, 1.0, 1.0][pos(e)];
    let problem = MinimumCostFlowProblemImpl::with_costs(
        &g,
        |vertex| match *vertex {
            0 => 6,
            3 => -6,
            _ => 0,
        },
        cap,
        |_| 0,
        cost,
    );
    let flow = CapacityScalingMinimumCostFlow::new()
        .get_minimum_cost_flow(&problem)
        .unwrap();
    assert_eq!(
        g.out_edges_of(&0)
            .map(|edge| flow.flow(edge).unwrap())
            .sum::<f64>(),
        6.0
    );
    assert_eq!(flow.cost, 19.0);
    // cheapest: 0-1-2-3 costs 3, 0-1-3 costs 4, 0-2-3 costs 3: fill 0-1 (4) and 0-2 (2)
    let problem = MinimumCostFlowProblemImpl::with_costs(
        &g,
        |vertex| match *vertex {
            0 => 2,
            3 => -2,
            _ => 0,
        },
        cap,
        |_| 0,
        cost,
    );
    let flow = CapacityScalingMinimumCostFlow::new()
        .get_minimum_cost_flow(&problem)
        .unwrap();
    assert_eq!(flow.cost, 6.0);
}
