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

use jgrapht::alg::clustering::*;
use jgrapht::alg::linkprediction::*;
use jgrapht::alg::tour::*;
use jgrapht::alg::transform::*;
use jgrapht::generate::*;
use jgrapht::*;
use rand::{SeedableRng, rngs::StdRng};

fn build<G: GraphMut<Vertex = usize> + IncidenceGraph>(mut g: G, mut f: impl GraphGenerator) -> G {
    f.generate_indexed(&mut g).unwrap();
    g
}

#[test]
fn transforms() {
    let ring = build(UGraph::<usize>::simple(), Ring(5));
    let mut co = UGraph::<usize>::simple();
    complement_into(&ring, &mut co, false).unwrap();
    assert_eq!((co.vertex_count(), co.edge_count()), (5, 5)); // C5 is self-complementary in size
    let mut lg = UGraph::<DefaultEdge>::simple();
    line_graph_into(&ring, &mut lg).unwrap();
    assert_eq!((lg.vertex_count(), lg.edge_count()), (5, 5));
    let mut path = build(UGraph::<usize>::simple(), Linear(4));
    let e = *path.get_edge(&1, &2).unwrap();
    contract_edge(&mut path, &e).unwrap();
    assert_eq!((path.vertex_count(), path.edge_count()), (3, 2));
    let mut d = build(DiGraph::<usize>::simple(), Linear(3));
    let mut dl = DiGraph::<DefaultEdge>::simple();
    line_graph_into(&d, &mut dl).unwrap();
    assert_eq!((dl.vertex_count(), dl.edge_count()), (2, 1));
    d.add_vertex(9);
}

#[test]
fn link_prediction() {
    let mut g = UGraph::<u32>::simple();
    for (a, b) in [(1, 2), (1, 3), (2, 3), (3, 4), (4, 5)] {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    assert_eq!(pair_score(&g, Measure::CommonNeighbors, &1, &4), Some(1.0));
    assert_eq!(pair_score(&g, Measure::Jaccard, &1, &4), Some(1.0 / 3.0));
    assert_eq!(
        pair_score(&g, Measure::PreferentialAttachment, &1, &4),
        Some(4.0)
    );
    let p = predict(&g, Measure::ResourceAllocation, &1).unwrap();
    assert_eq!(p[0].0, 4);
    assert!(pair_score(&g, Measure::AdamicAdar, &1, &99).is_none());
    let sr = simrank(&g, 0.8, 5);
    assert_eq!(sr[&(1, 1)], 1.0);
    assert!(sr[&(1, 2)] > sr[&(1, 5)]);
}

#[test]
fn clusterings() {
    // two triangles joined by a light edge
    let mut g = UGraph::<u32>::simple().weighted();
    for (a, b, w) in [
        (1, 2, 1.0),
        (2, 3, 1.0),
        (1, 3, 1.0),
        (4, 5, 1.0),
        (5, 6, 1.0),
        (4, 6, 1.0),
        (3, 4, 10.0),
    ] {
        g.add_vertex(a);
        g.add_vertex(b);
        g.add_weighted_edge(&a, &b, w).unwrap();
    }
    let mut c = k_spanning_tree_clustering(&g, 2).unwrap();
    c.iter_mut().for_each(|x| x.sort());
    assert_eq!(c, [vec![1, 2, 3], vec![4, 5, 6]]);
    let lp = label_propagation(&g, 50, &mut StdRng::seed_from_u64(1));
    assert!(lp.len() <= 3);
    assert_eq!(lp.iter().map(Vec::len).sum::<usize>(), 6);
    assert!(k_spanning_tree_clustering(&g, 0).is_err());
}

#[test]
fn tours() {
    // points on a line scaled: complete graph with euclidean distances on a 4x3 lattice subset
    let pts: [(f64, f64); 8] = [
        (0., 0.),
        (3., 0.),
        (3., 2.),
        (0., 2.),
        (1., 1.),
        (2., 1.),
        (1.5, 0.),
        (1.5, 2.),
    ];
    let mut g = UGraph::<usize>::simple().weighted();
    g.add_vertices(0..8);
    for i in 0..8 {
        for j in i + 1..8 {
            let w = ((pts[i].0 - pts[j].0).powi(2) + (pts[i].1 - pts[j].1).powi(2)).sqrt();
            g.add_weighted_edge(&i, &j, w).unwrap();
        }
    }
    let opt = HeldKarpTSP::new().get_tour(&g).unwrap().unwrap();
    assert!(opt.is_valid(&g));
    assert_eq!(
        (
            opt.length(),
            opt.start_vertex().copied(),
            opt.end_vertex().copied()
        ),
        (8, Some(0), Some(0))
    );
    for t in [
        NearestNeighborHeuristicTSP::with_first(0)
            .get_tour(&g)
            .unwrap()
            .unwrap(),
        NearestInsertionHeuristicTSP::new()
            .get_tour(&g)
            .unwrap()
            .unwrap(),
        FarthestInsertionHeuristicTSP::new()
            .get_tour(&g)
            .unwrap()
            .unwrap(),
        TwoApproxMetricTSP::new().get_tour(&g).unwrap().unwrap(),
    ] {
        assert!(t.is_valid(&g));
        assert_eq!(t.length(), 8);
        assert!(t.weight >= opt.weight - 1e-9);
        let improved = TwoOptHeuristicTSP::new().improve_tour(&g, &t).unwrap();
        assert!(improved.weight <= t.weight + 1e-9 && improved.weight >= opt.weight - 1e-9);
    }
    assert!(
        HeldKarpTSP::new()
            .get_tour(&build(UGraph::<usize>::simple(), Linear(4)))
            .unwrap()
            .is_none()
    );
}
