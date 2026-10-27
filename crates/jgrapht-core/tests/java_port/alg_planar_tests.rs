//! Port of `BoyerMyrvoldPlanarityInspectorTest`.

use crate::planar_data::CASES;
use jgrapht::alg::planar::BoyerMyrvoldPlanarityInspector;
use jgrapht::graph::props;
use jgrapht::*;
use std::collections::HashMap;

fn undirected(edges: &[(i32, i32)]) -> UGraph<i32> {
    let mut g = UGraph::<i32>::with_loops().weighted();
    for (a, b) in edges {
        g.add_vertex(*a);
        g.add_vertex(*b);
        let _ = g.add_edge(a, b);
    }
    g
}

fn test_on_graph(edges: &[(i32, i32)]) {
    let graph = undirected(edges);
    let mut inspector = BoyerMyrvoldPlanarityInspector::new(&graph);
    if inspector.is_planar() {
        let embedding = inspector.embedding().unwrap();
        let mut counts: HashMap<DefaultEdge, usize> = HashMap::new();
        let mut degree_sum = 0;
        for v in graph.vertices() {
            let around = embedding.edges_around(v).unwrap();
            for e in around {
                assert!(graph.contains_edge(e));
                *counts.entry(*e).or_default() += 1;
            }
            degree_sum += around.len();
        }
        for e in graph.edges() {
            assert_eq!(counts.get(e), Some(&2), "edge {e:?}");
        }
        assert_eq!(degree_sum, 2 * graph.edge_count());
    } else {
        let subdivision = inspector.kuratowski_subdivision().unwrap();
        assert!(props::is_kuratowski_subdivision(&subdivision));
    }
}

#[test]
fn test_non_planar_graph_no_embedding() {
    let k5: Vec<(i32, i32)> = (0..5)
        .flat_map(|i| (i + 1..5).map(move |j| (i, j)))
        .collect();
    let graph = undirected(&k5);
    assert!(
        BoyerMyrvoldPlanarityInspector::new(&graph)
            .embedding()
            .is_err()
    );
}

#[test]
fn test_planar_graph_no_kuratowski_subdivision() {
    let graph = undirected(&[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]);
    assert!(
        BoyerMyrvoldPlanarityInspector::new(&graph)
            .kuratowski_subdivision()
            .is_err()
    );
}

#[test]
fn test_planarity_cases() {
    for (i, case) in CASES.iter().enumerate() {
        std::panic::catch_unwind(|| test_on_graph(case))
            .unwrap_or_else(|_| panic!("testPlanarity{i} failed"));
    }
}

/// Number of faces of a rotation system: orbits of darts `(edge, direction)` under
/// "reverse the dart, then take the next edge in the rotation at its head".
fn face_count(
    graph: &UGraph<i32>,
    emb: &jgrapht::alg::interfaces::Embedding<i32, DefaultEdge>,
) -> usize {
    use std::collections::HashSet;
    let mut pos: HashMap<(i32, DefaultEdge), usize> = HashMap::new();
    for v in graph.vertices() {
        for (i, e) in emb.edges_around(v).unwrap().iter().enumerate() {
            pos.insert((*v, *e), i);
        }
    }
    let mut seen: HashSet<(DefaultEdge, i32)> = HashSet::new(); // (edge, tail)
    let mut faces = 0;
    for e in graph.edges() {
        let (a, b) = graph.edge_endpoints(e).unwrap();
        for tail in [*a, *b] {
            if seen.contains(&(*e, tail)) {
                continue;
            }
            faces += 1;
            let (mut edge, mut t) = (*e, tail);
            while seen.insert((edge, t)) {
                let head = graph.opposite_vertex(&edge, &t).copied().unwrap();
                let around = emb.edges_around(&head).unwrap();
                let i = pos[&(head, edge)];
                edge = around[(i + 1) % around.len()];
                t = head;
            }
        }
    }
    faces
}

fn components(graph: &UGraph<i32>) -> usize {
    jgrapht::alg::connectivity::connected_components(graph).len()
}

#[test]
fn embeddings_satisfy_eulers_formula() {
    for (i, case) in CASES.iter().enumerate() {
        let graph = undirected(case);
        let mut inspector = BoyerMyrvoldPlanarityInspector::new(&graph);
        if !inspector.is_planar() {
            continue;
        }
        let emb = inspector.embedding().unwrap().clone();
        // isolated vertices add one face each to the dart count, they have no darts
        let isolated = graph.vertices().filter(|v| graph.degree_of(v) == 0).count();
        let faces = face_count(&graph, &emb) + isolated;
        assert_eq!(
            graph.vertex_count() + faces,
            graph.edge_count() + 2 * components(&graph),
            "testPlanarity{i}"
        );
    }
}

#[test]
fn agrees_with_the_left_right_test_on_random_graphs() {
    use jgrapht::generate::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    let mut planar = 0;
    for seed in 0..300u64 {
        let n = 5 + (seed % 9) as usize;
        let p = 0.1 + (seed % 7) as f64 * 0.07;
        let mut g = UGraph::<i32>::simple().with_vertex_factory(|i| i as i32);
        GnpRandomGraphGenerator::new(n, p, StdRng::seed_from_u64(seed), false)
            .unwrap()
            .generate_default(&mut g)
            .unwrap();
        let mut inspector = BoyerMyrvoldPlanarityInspector::new(&g);
        let is_planar = inspector.is_planar();
        assert_eq!(is_planar, props::is_planar(&g), "seed {seed}");
        if is_planar {
            planar += 1;
            let emb = inspector.embedding().unwrap().clone();
            let isolated = g.vertices().filter(|v| g.degree_of(v) == 0).count();
            assert_eq!(
                g.vertex_count() + face_count(&g, &emb) + isolated,
                g.edge_count() + 2 * components(&g),
                "seed {seed}"
            );
        } else {
            let sub = inspector.kuratowski_subdivision().unwrap();
            assert!(props::is_kuratowski_subdivision(&sub), "seed {seed}");
        }
    }
    assert!(
        planar > 50 && planar < 250,
        "mixed instances expected, got {planar} planar"
    );
}
