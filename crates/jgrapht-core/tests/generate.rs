use jgrapht::generate::*;
use jgrapht::graph::props;
use jgrapht::*;
use rand::{SeedableRng, rngs::StdRng};

fn rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

fn counts<G: Graph>(g: &G) -> (usize, usize) {
    (g.vertex_count(), g.edge_count())
}

#[test]
fn deterministic_undirected() {
    let mut g = UGraph::<usize>::simple();
    Complete(5).generate_indexed(&mut g).unwrap();
    assert_eq!(counts(&g), (5, 10));
    assert!(props::is_complete(&g));

    let mut g = UGraph::<usize>::simple();
    Linear(6).generate_indexed(&mut g).unwrap();
    assert_eq!(counts(&g), (6, 5));
}

#[test]
fn shapes() {
    let mut g = UGraph::<usize>::simple();
    Ring(6).generate_indexed(&mut g).unwrap();
    assert_eq!(counts(&g), (6, 6));
    let mut g = UGraph::<usize>::simple();
    Star(5).generate_indexed(&mut g).unwrap();
    assert_eq!(counts(&g), (5, 4));
    assert!(props::is_tree(&g));
    let mut g = UGraph::<usize>::simple();
    Wheel {
        n: 6,
        inward_spokes: false,
    }
    .generate_indexed(&mut g)
    .unwrap();
    assert_eq!(counts(&g), (6, 10));
    let mut g = UGraph::<usize>::simple();
    Grid { rows: 3, cols: 4 }.generate_indexed(&mut g).unwrap();
    assert_eq!(counts(&g), (12, 17));
    let mut g = UGraph::<usize>::simple();
    Hypercube(4).generate_indexed(&mut g).unwrap();
    assert_eq!(counts(&g), (16, 32));
    let mut g = UGraph::<usize>::simple();
    CompleteBipartite(3, 4).generate_indexed(&mut g).unwrap();
    assert_eq!(counts(&g), (7, 12));
    let mut g = UGraph::<usize>::simple();
    Empty(3).generate_indexed(&mut g).unwrap();
    assert_eq!(counts(&g), (3, 0));
}

#[test]
fn directed_variants() {
    let mut g = DiGraph::<usize>::simple();
    Complete(4).generate_indexed(&mut g).unwrap();
    assert_eq!(counts(&g), (4, 12));
    let mut g = DiGraph::<usize>::simple();
    Grid { rows: 2, cols: 2 }.generate_indexed(&mut g).unwrap();
    assert_eq!(counts(&g), (4, 8));
    let mut g = DiGraph::<usize>::dag();
    Ring(4).generate_indexed(&mut g).unwrap(); // closing edge refused as it would form a cycle
    assert_eq!(counts(&g), (4, 3));
}

#[test]
fn random_generators() {
    let mut g = UGraph::<usize>::simple();
    Gnm {
        n: 20,
        m: 50,
        loops: false,
        rng: rng(),
    }
    .generate_indexed(&mut g)
    .unwrap();
    assert_eq!(counts(&g), (20, 50));
    let mut g = UGraph::<usize>::simple();
    Gnp {
        n: 30,
        p: 1.0,
        loops: false,
        rng: rng(),
    }
    .generate_indexed(&mut g)
    .unwrap();
    assert_eq!(g.edge_count(), 30 * 29 / 2);
    let mut g = UGraph::<usize>::simple();
    BarabasiAlbert {
        m0: 3,
        m: 2,
        n: 50,
        rng: rng(),
    }
    .generate_indexed(&mut g)
    .unwrap();
    assert_eq!(counts(&g), (50, 3 + 2 * 47));
    assert!(props::is_connected(&g));
    let mut g = UGraph::<usize>::simple();
    WattsStrogatz {
        n: 30,
        k: 4,
        p: 0.3,
        rng: rng(),
    }
    .generate_indexed(&mut g)
    .unwrap();
    assert_eq!(counts(&g), (30, 60));
    let mut g = UGraph::<usize>::simple();
    Prufer {
        n: 25,
        sequence: None,
        rng: rng(),
    }
    .generate_indexed(&mut g)
    .unwrap();
    assert!(props::is_tree(&g));
}

#[test]
fn prufer_known_sequence_and_errors() {
    let mut g = UGraph::<usize>::simple();
    Prufer {
        n: 5,
        sequence: Some(vec![3, 3, 3]),
        rng: rng(),
    }
    .generate_indexed(&mut g)
    .unwrap();
    assert_eq!(g.degree_of(&3), 4);
    assert!(
        Prufer {
            n: 5,
            sequence: Some(vec![9]),
            rng: rng()
        }
        .generate_indexed(&mut g)
        .is_err()
    );
    assert!(
        Gnp {
            n: 3,
            p: 2.0,
            loops: false,
            rng: rng()
        }
        .generate_indexed(&mut g)
        .is_err()
    );
    assert!(
        Gnm {
            n: 3,
            m: 4,
            loops: false,
            rng: rng()
        }
        .generate_indexed(&mut g)
        .is_err()
    );
}

#[test]
fn custom_vertices() {
    let mut g = UGraph::<String>::simple();
    let vs = Linear(3).generate(&mut g, |i| format!("v{i}")).unwrap();
    assert_eq!(vs, ["v0", "v1", "v2"]);
    assert!(g.contains_edge_between(&"v1".to_string(), &"v2".to_string()));
}
