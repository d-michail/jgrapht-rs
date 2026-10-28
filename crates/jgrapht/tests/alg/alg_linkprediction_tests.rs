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

//! Ports of the `org.jgrapht.alg.linkprediction` tests.

use crate::common::construct_graph;
use jgrapht::GraphError;
use jgrapht::alg::interfaces::LinkPredictionAlgorithm;
use jgrapht::alg::linkprediction::*;
use jgrapht::*;

const EDGES: &[&[i32]] = &[
    &[0, 1],
    &[0, 3],
    &[1, 2],
    &[1, 4],
    &[2, 3],
    &[2, 4],
    &[3, 4],
    &[3, 5],
    &[4, 5],
];

fn sample_graph() -> UGraph<i32> {
    let mut g = UGraph::<i32>::simple();
    construct_graph(&mut g, EDGES);
    g
}

fn missing_pairs(g: &UGraph<i32>) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for &u in g.vertices() {
        for &v in g.vertices() {
            if u >= v || g.contains_edge_between(&u, &v) {
                continue;
            }
            out.push((u, v));
        }
    }
    out
}

fn scores(alg: &impl LinkPredictionAlgorithm<i32>, g: &UGraph<i32>) -> Vec<f64> {
    missing_pairs(g)
        .into_iter()
        .map(|(u, v)| alg.predict(&u, &v).unwrap())
        .collect()
}

fn assert_close(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() < 1e-9, "{actual:?} vs {expected:?}");
    }
}

fn is_not_well_defined<T: std::fmt::Debug>(r: Result<T>) -> bool {
    matches!(r, Err(GraphError::IndexNotWellDefined(_)))
}

fn two_isolated() -> UGraph<i32> {
    let mut g = UGraph::<i32>::simple();
    g.add_vertices([0, 1]);
    g
}

#[test]
fn adamic_adar_prediction() {
    let g = sample_graph();
    let alg = AdamicAdarIndexLinkPrediction::new(&g);
    let expected = [
        1.631586747071319,
        1.631586747071319,
        0.7213475204444817,
        3.074281787960283,
        0.7213475204444817,
        1.4426950408889634,
    ];
    assert_close(&scores(&alg, &g), &expected);
}

#[test]
fn adamic_adar_prediction_with_list_of_pairs() {
    let g = sample_graph();
    let alg = AdamicAdarIndexLinkPrediction::new(&g);
    let predictions = alg.predict_all(&missing_pairs(&g)).unwrap();
    let got: Vec<f64> = predictions.iter().map(|t| t.2).collect();
    assert_close(
        &got,
        &[
            1.631586747071319,
            1.631586747071319,
            0.7213475204444817,
            3.074281787960283,
            0.7213475204444817,
            1.4426950408889634,
        ],
    );
    assert_eq!((predictions[0].0, predictions[0].1), missing_pairs(&g)[0]);
}

#[test]
fn adamic_adar_invalid_prediction() {
    let mut g = DiGraph::<i32>::simple();
    construct_graph(&mut g, &[&[0, 2], &[1, 2]]);
    assert!(is_not_well_defined(
        AdamicAdarIndexLinkPrediction::new(&g).predict(&0, &1)
    ));
}

#[test]
fn common_neighbors_prediction() {
    let g = sample_graph();
    let alg = CommonNeighborsLinkPrediction::new(&g);
    assert_eq!(alg.predict(&0, &2).unwrap(), 2.0);
    assert_eq!(alg.predict(&0, &4).unwrap(), 2.0);
    assert_eq!(alg.predict(&3, &2).unwrap(), 1.0);
}

#[test]
fn hub_depressed_prediction() {
    let g = sample_graph();
    let alg = HubDepressedIndexLinkPrediction::new(&g);
    assert_close(
        &scores(&alg, &g),
        &[
            0.6666666666666666,
            0.5,
            0.5,
            0.75,
            0.3333333333333333,
            0.6666666666666666,
        ],
    );
    assert!(is_not_well_defined(alg_on(&two_isolated(), |g| {
        HubDepressedIndexLinkPrediction::new(g).predict(&0, &1)
    })));
}

#[test]
fn hub_promoted_prediction() {
    let g = sample_graph();
    let alg = HubPromotedIndexLinkPrediction::new(&g);
    assert_close(&scores(&alg, &g), &[1.0, 1.0, 0.5, 1.0, 0.5, 1.0]);
    assert!(is_not_well_defined(alg_on(&two_isolated(), |g| {
        HubPromotedIndexLinkPrediction::new(g).predict(&0, &1)
    })));
}

fn alg_on<R>(g: &UGraph<i32>, f: impl FnOnce(&UGraph<i32>) -> R) -> R {
    f(g)
}

#[test]
fn jaccard_prediction() {
    let g = sample_graph();
    let alg = JaccardCoefficientLinkPrediction::new(&g);
    assert!((alg.predict(&0, &2).unwrap() - 2.0 / 3.0).abs() < 1e-9);
    assert!((alg.predict(&0, &4).unwrap() - 2.0 / 4.0).abs() < 1e-9);
    assert!((alg.predict(&3, &2).unwrap() - 1.0 / 6.0).abs() < 1e-9);
    assert_eq!(alg.predict(&3, &3).unwrap(), 1.0);
}

#[test]
fn jaccard_invalid_prediction() {
    assert!(is_not_well_defined(alg_on(&two_isolated(), |g| {
        JaccardCoefficientLinkPrediction::new(g).predict(&0, &1)
    })));
}

#[test]
fn leicht_holme_newman_prediction() {
    let g = sample_graph();
    let alg = LeichtHolmeNewmanIndexLinkPrediction::new(&g);
    assert_close(
        &scores(&alg, &g),
        &[3.0, 4.0, 1.0, 4.0, 0.6666666666666666, 1.3333333333333333],
    );
}

#[test]
fn leicht_holme_newman_invalid_prediction() {
    assert!(is_not_well_defined(alg_on(&two_isolated(), |g| {
        LeichtHolmeNewmanIndexLinkPrediction::new(g).predict(&0, &1)
    })));
}

#[test]
fn preferential_attachment_prediction() {
    let g = sample_graph();
    let alg = PreferentialAttachmentLinkPrediction::new(&g);
    assert_close(&scores(&alg, &g), &[6.0, 8.0, 4.0, 12.0, 6.0, 6.0]);
}

#[test]
fn resource_allocation_prediction() {
    let g = sample_graph();
    let alg = ResourceAllocationIndexLinkPrediction::new(&g);
    assert_close(
        &scores(&alg, &g),
        &[
            0.5833333333333333,
            0.5833333333333333,
            0.25,
            1.0833333333333333,
            0.25,
            0.5,
        ],
    );
}

#[test]
fn resource_allocation_invalid_prediction() {
    // a directed graph where the common successor has no outgoing edge
    let mut g = DiGraph::<i32>::simple();
    g.add_vertices([0, 1, 2]);
    g.add_edge(&0, &2).unwrap();
    g.add_edge(&1, &2).unwrap();
    assert!(is_not_well_defined(
        ResourceAllocationIndexLinkPrediction::new(&g).predict(&0, &1)
    ));
}

#[test]
fn salton_prediction() {
    let g = sample_graph();
    let alg = SaltonIndexLinkPrediction::new(&g);
    assert_close(
        &scores(&alg, &g),
        &[
            0.8164965809277261,
            0.7071067811865475,
            0.5,
            0.8660254037844387,
            0.4082482904638631,
            0.8164965809277261,
        ],
    );
}

#[test]
fn salton_invalid_prediction() {
    assert!(is_not_well_defined(alg_on(&two_isolated(), |g| {
        SaltonIndexLinkPrediction::new(g).predict(&0, &1)
    })));
}

#[test]
fn sorensen_prediction() {
    let g = sample_graph();
    let alg = SorensenIndexLinkPrediction::new(&g);
    assert_close(
        &scores(&alg, &g),
        &[0.8, 0.6666666666666666, 0.5, 0.8571428571428571, 0.4, 0.8],
    );
}

#[test]
fn sorensen_invalid_prediction() {
    assert!(is_not_well_defined(alg_on(&two_isolated(), |g| {
        SorensenIndexLinkPrediction::new(g).predict(&0, &1)
    })));
}
