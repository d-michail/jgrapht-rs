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

//! Second valid-input sweep, for the families `alg_no_panic_tests` does not cover: matching,
//! tours, cycles, planarity, isomorphism, cuts and trees. Same rule: a value or an `Err`, never a
//! panic, on small pseudographs with loops, parallel edges, isolated vertices and odd weights.

use crate::alg_no_panic_tests::{POSITIVE, VALID, directed, sweep, undirected};
use indexmap::IndexSet;
use jgrapht::alg::cycle::*;
use jgrapht::alg::flow::*;
use jgrapht::alg::isomorphism::*;
use jgrapht::alg::matching::blossom::{
    KolmogorovWeightedMatching, KolmogorovWeightedPerfectMatching,
};
use jgrapht::alg::matching::*;
use jgrapht::alg::planar::*;
use jgrapht::alg::stoer_wagner::StoerWagnerMinimumCut;
use jgrapht::alg::tour::*;
use jgrapht::*;

#[test]
fn matching_does_not_panic() {
    sweep("matching", undirected, &VALID, |g| {
        let _ = GreedyMaximumCardinalityMatching::new(g, true).and_then(|mut m| m.matching());
        let _ = GreedyMaximumCardinalityMatching::new(g, false).and_then(|mut m| m.matching());
        let _ = GreedyWeightedMatching::new(g, true).and_then(|mut m| m.matching());
        let _ = GreedyWeightedMatching::new(g, false).and_then(|mut m| m.matching());
        let _ = PathGrowingWeightedMatching::new(g).and_then(|mut m| m.matching());
        let _ = DenseEdmonds::new(g).and_then(|mut m| m.matching());
        let _ = SparseEdmonds::new(g).and_then(|mut m| m.matching());
        let _ = KolmogorovWeightedPerfectMatching::new(g).and_then(|mut m| m.matching());
        let _ = KolmogorovWeightedMatching::new(g).and_then(|mut m| m.matching());
        let vs: Vec<usize> = g.vertices_cloned().collect();
        let (a, b) = vs.split_at(vs.len() / 2);
        let (a, b): (IndexSet<usize>, IndexSet<usize>) =
            (a.iter().copied().collect(), b.iter().copied().collect());
        let _ = HopcroftKarp::new(g, a.clone(), b.clone()).and_then(|mut m| m.matching());
        let _ = MaximumWeightBipartiteMatching::new(g, a.clone(), b.clone())
            .and_then(|mut m| m.matching());
        let _ = KuhnMunkres::new(g, a, b).matching();
    });
    sweep("matching positive", undirected, &POSITIVE, |g| {
        let _ = KolmogorovWeightedPerfectMatching::new(g).and_then(|mut m| m.matching());
        let _ = KolmogorovWeightedMatching::new(g).and_then(|mut m| m.matching());
    });
}

#[test]
fn tours_do_not_panic() {
    sweep("tours", undirected, &POSITIVE, |g| {
        let _ = RandomTourTsp::new().tour(g);
        let _ = NearestNeighborHeuristicTsp::new().tour(g);
        let _ = NearestInsertionHeuristicTsp::new().tour(g);
        let _ = FarthestInsertionHeuristicTsp::new().tour(g);
        let _ = GreedyHeuristicTsp.tour(g);
        let _ = TwoApproxMetricTsp.tour(g);
        let _ = Christofides.tour(g);
        let _ = HeldKarpTsp.tour(g);
        let _ = TwoOptHeuristicTsp::new().tour(g);
        let _ = PalmerHamiltonianCycle.tour(g);
    });
}

#[test]
fn cycles_do_not_panic() {
    sweep("directed cycles", directed, &VALID, |g| {
        let _ = TiernanSimpleCycles::with_graph(g).find_simple_cycles();
        let _ = TarjanSimpleCycles::with_graph(g).find_simple_cycles();
        let _ = JohnsonSimpleCycles::new(g).and_then(|mut c| c.find_simple_cycles());
        let _ = SzwarcfiterLauerSimpleCycles::with_graph(g).find_simple_cycles();
        let _ = HawickJamesSimpleCycles::with_graph(g).find_simple_cycles();
        let _ = HowardMinimumMeanCycle::new(g).cycle_mean();
        let _ = HierholzerEulerianCycle::new().eulerian_cycle(g);
    });
    sweep("undirected cycles", undirected, &VALID, |g| {
        let _ = QueueBFSFundamentalCycleBasis::new(g).and_then(|mut c| c.cycle_basis());
        let _ = StackBFSFundamentalCycleBasis::new(g).and_then(|mut c| c.cycle_basis());
        let _ = PatonCycleBase::new(g).and_then(|mut c| c.cycle_basis());
        let _ = ChinesePostman::new().cpp_solution(g);
        let _ = HierholzerEulerianCycle::new().eulerian_cycle(g);
    });
}

#[test]
fn planarity_isomorphism_and_cuts_do_not_panic() {
    sweep("undirected structure", undirected, &POSITIVE, |g| {
        let _ = BoyerMyrvoldPlanarityInspector::new(g).planarity();
        let _ = VF2GraphIsomorphismInspector::new(g, g).map(|i| i.iter().count());
        let _ = ColorRefinementIsomorphismInspector::new(g, g).map(|mut i| i.isomorphism_exists());
        let _ = StoerWagnerMinimumCut::new(g).map(|c| c.min_cut_weight());
        let _ = GusfieldEquivalentFlowTree::new(g).and_then(|mut t| t.equivalent_flow_tree());
        let _ = GusfieldGomoryHuCutTree::new(g).and_then(|mut t| t.gomory_hu_tree());
        let _ = AhuUnrootedTreeIsomorphismInspector::new(g, g).map(|i| i.mapping());
        let _ = PadbergRaoOddMinimumCutset::new(g).and_then(|mut p| {
            let odd: indexmap::IndexSet<usize> = g.vertices_cloned().take(2).collect();
            p.minimum_odd_cut(odd, false)
        });
    });
    sweep("directed structure", directed, &POSITIVE, |g| {
        let _ = VF2GraphIsomorphismInspector::new(g, g).map(|i| i.iter().count());
        let _ = VF2SubgraphIsomorphismInspector::new(g, g).map(|i| i.iter().count());
    });
}

/// Regressions for the panics this sweep found.
mod regressions {
    use super::*;

    #[test]
    fn planarity_ignores_self_loops_and_parallel_edges() {
        let mut g = UGraph::<usize>::pseudo();
        g.add_vertices(0..4);
        for (a, b) in [
            (0, 1),
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (0, 2),
            (2, 2),
            (3, 3),
        ] {
            g.add_edge(&a, &b).unwrap();
        }
        assert!(BoyerMyrvoldPlanarityInspector::new(&g).is_planar());
        assert!(matches!(
            BoyerMyrvoldPlanarityInspector::new(&g).planarity(),
            PlanarityResult::Planar(_)
        ));
    }

    #[test]
    fn hopcroft_karp_ignores_edges_that_do_not_join_the_partitions() {
        let mut g = UGraph::<usize>::simple();
        g.add_vertices(0..4);
        g.add_edge(&0, &1).unwrap(); // inside the first partition
        g.add_edge(&1, &2).unwrap();
        g.add_edge(&2, &3).unwrap(); // to a vertex in neither partition
        let a: Vec<usize> = vec![0, 1];
        let b: Vec<usize> = vec![2];
        let m = HopcroftKarp::new(&g, a.clone(), b)
            .and_then(|mut m| m.matching())
            .unwrap();
        assert_eq!(m.edges().len(), 1);
        let r = HopcroftKarp::new(&g, a.clone(), a).and_then(|mut m| m.matching());
        assert!(r.is_err(), "overlapping partitions");
    }

    #[test]
    fn chinese_postman_rejects_a_disconnected_graph() {
        let mut g = UGraph::<usize>::simple().weighted();
        g.add_vertices(0..4);
        g.add_edge(&0, &1).unwrap();
        g.add_edge(&2, &3).unwrap();
        assert!(ChinesePostman::new().cpp_solution(&g).is_err());
    }

    #[test]
    fn ahu_rejects_a_graph_with_parallel_edges() {
        let mut g = UGraph::<usize>::pseudo();
        g.add_vertices(0..2);
        g.add_edge(&0, &1).unwrap();
        g.add_edge(&0, &1).unwrap();
        assert!(AhuUnrootedTreeIsomorphismInspector::new(&g, &g).is_err());
    }
}
