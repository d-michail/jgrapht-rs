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

//! Valid-input sweep: every algorithm below runs on hundreds of small pseudographs (empty,
//! single vertex, self-loops, parallel edges, isolated vertices, zero and negative weights) and must
//! return a value or an `Err`, never panic. Weights that are NaN or infinite are checked separately
//! for the fallible entry points, because they are invalid input.

use jgrapht::alg::clique::*;
use jgrapht::alg::color::*;
use jgrapht::alg::connectivity::*;
use jgrapht::alg::cover::*;
use jgrapht::alg::flow::*;
use jgrapht::alg::scoring::*;
use jgrapht::alg::shortestpath::*;
use jgrapht::alg::spanning::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::borrow::Borrow;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn random_graph<G: GraphMut<Vertex = usize>>(mut g: G, rng: &mut StdRng, weights: &[f64]) -> G {
    let n = rng.random_range(0..7usize);
    g.add_vertices(0..n);
    if n > 0 {
        for _ in 0..rng.random_range(0..14usize) {
            let (a, b) = (rng.random_range(0..n), rng.random_range(0..n));
            if let Ok(Some(e)) = g.add_edge(&a, &b) {
                let w = weights[rng.random_range(0..weights.len())];
                let _ = g.set_edge_weight(&e, w);
            }
        }
    }
    g
}

pub(crate) fn sweep<G: GraphMut<Vertex = usize>>(
    name: &str,
    make: impl Fn() -> G,
    weights: &[f64],
    run: impl Fn(&G),
) {
    let mut rng = StdRng::seed_from_u64(7);
    for _ in 0..300 {
        let g = random_graph(make(), &mut rng, weights);
        let description = format!(
            "{name}: n={} edges={:?}",
            g.vertex_count(),
            g.edges()
                .map(|e| {
                    let e = e.borrow();
                    g.edge_endpoints(e).map(|(a, b)| (*a, *b))
                })
                .collect::<Vec<_>>()
        );
        if catch_unwind(AssertUnwindSafe(|| run(&g))).is_err() {
            panic!("panicked on {description}");
        }
    }
}

pub(crate) const VALID: [f64; 5] = [0.0, 1.0, 2.5, 7.0, -1.0];
pub(crate) const POSITIVE: [f64; 4] = [0.0, 1.0, 2.5, 7.0];
const INVALID: [f64; 4] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0];

pub(crate) fn undirected() -> UGraph<usize> {
    UGraph::<usize>::pseudo().weighted()
}

pub(crate) fn directed() -> DiGraph<usize> {
    DiGraph::<usize>::pseudo().weighted()
}

#[test]
fn undirected_algorithms_do_not_panic() {
    sweep("undirected", undirected, &VALID, |g| {
        let _ = maximal_cliques(g);
        let _ = maximum_clique(g);
        let _ = greedy_independent_set(g);
        let _ = maximum_independent_set(g);
        let _ = greedy_coloring(g);
        let _ = largest_degree_first_coloring(g);
        let _ = smallest_degree_last_coloring(g);
        let _ = saturation_degree_coloring(g);
        let _ = color_refinement(g);
        let _ = connected_components(g);
        let _ = biconnectivity(g);
        let _ = is_biconnected(g);
        let _ = block_cut_tree(g);
        let _ = greedy_vertex_cover(g);
        let _ = two_approximation_vertex_cover(g);
        let _ = kruskal(g);
        let _ = prim(g);
        let _ = boruvka(g);
        let _ = coreness(g);
        let _ = local_clustering_coefficients(g);
        let _ = average_clustering_coefficient(g);
        let _ = global_clustering_coefficient(g);
        let _ = count_triangles(g);
        let _ = degree_centrality(g, true);
        let _ = closeness_centrality(g, false);
        let _ = harmonic_centrality(g, false);
        let _ = jgrapht::graph::props::is_connected(g);
        let _ = jgrapht::graph::props::is_tree(g);
        let _ = jgrapht::graph::props::is_forest(g);
        let _ = jgrapht::graph::props::is_bipartite(g);
        let _ = jgrapht::graph::props::is_chordal(g);
        let _ = jgrapht::graph::props::is_planar(g);
        let _ = jgrapht::graph::props::is_eulerian(g);
        let _ = jgrapht::graph::props::is_perfect(g);
        let _ = jgrapht::graph::props::is_weakly_chordal(g);
    });
}

#[test]
fn directed_algorithms_do_not_panic() {
    sweep("directed", directed, &VALID, |g| {
        let _ = strongly_connected_components(g);
        let _ = strongly_connected_components_kosaraju(g);
        let _ = is_strongly_connected(g);
        let _ = condensation(g);
        let _ = greedy_coloring(g);
        let _ = connected_components(g);
        let _ = jgrapht::graph::props::is_dag(g);
        let _ = jgrapht::graph::props::is_eulerian(g);
        let _ = jgrapht::graph::props::is_weakly_connected(g);
        let _ = jgrapht::graph::props::is_strongly_connected(g);
        let _ = jgrapht::traverse::topological_sort(g);
    });
}

fn shortest_paths<G: IncidenceGraph<Vertex = usize> + IncomingGraph + GraphMut>(g: &G) {
    let vs: Vec<usize> = g.vertices_cloned().collect();
    for a in vs.iter().chain([&99]) {
        for b in vs.iter().chain([&99]) {
            let _ = Dijkstra::new(g).path_weight(a, b);
            let _ = BellmanFord::new(g).path_weight(a, b);
            let _ = BfsShortestPath::new(g).path_weight(a, b);
            let _ = BidirectionalDijkstraShortestPath::new(g).path_weight(a, b);
            if let Ok(mut flow) = EdmondsKarpMFImpl::new(g) {
                let _ = flow.maximum_flow(a, b);
            }
            if let Ok(mut flow) = DinicMFImpl::new(g) {
                let _ = flow.maximum_flow(a, b);
            }
            if let Ok(mut flow) = BoykovKolmogorovMFImpl::new(g) {
                let _ = flow.maximum_flow(a, b);
            }
            if let Ok(mut flow) = PushRelabelMFImpl::new(g) {
                let _ = flow.maximum_flow(a, b);
            }
        }
    }
}

#[test]
fn shortest_paths_and_flows_do_not_panic_on_valid_weights() {
    sweep("directed paths", directed, &VALID, shortest_paths);
    sweep("directed paths", directed, &POSITIVE, shortest_paths);
    sweep("undirected paths", undirected, &POSITIVE, shortest_paths);
}

#[test]
fn invalid_weights_are_reported_not_panicked_on() {
    sweep("invalid weights", directed, &INVALID, |g| {
        shortest_paths(g);
        let _ = betweenness_centrality(g, false, false);
        let _ = pagerank(g, 0.85, 10, 1e-6);
        let _ = kruskal(g);
        let _ = prim(g);
    });
}
