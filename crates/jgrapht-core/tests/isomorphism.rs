use jgrapht::alg::isomorphism::*;
use jgrapht::generate::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

fn build<G: GraphMut<Vertex = usize>>(mut g: G, mut f: impl GraphGenerator) -> G {
    f.generate_indexed(&mut g).unwrap();
    g
}
fn relabel(g: &UGraph<usize>, perm: &[usize]) -> UGraph<char> {
    let mut h = UGraph::<char>::simple();
    let c = |i: usize| (b'a' + perm[i] as u8) as char;
    g.vertices().for_each(|&v| {
        h.add_vertex(c(v));
    });
    for e in g.edges() {
        h.add_edge(
            &c(*g.edge_source(e).unwrap()),
            &c(*g.edge_target(e).unwrap()),
        )
        .unwrap();
    }
    h
}

#[test]
fn isomorphic_after_relabeling() {
    let mut rng = StdRng::seed_from_u64(8);
    for _ in 0..20 {
        let g = {
            let mut g = UGraph::<usize>::simple();
            GnpRandomGraphGenerator::new(9, 0.35, StdRng::seed_from_u64(rng.random()), false)
                .unwrap()
                .generate_indexed(&mut g)
                .unwrap();
            g
        };
        let mut perm: Vec<usize> = (0..9).collect();
        for i in (1..9).rev() {
            perm.swap(i, rng.random_range(0..=i));
        }
        let h = relabel(&g, &perm);
        let m = find_isomorphism(&g, &h).expect("isomorphic");
        for e in g.edges() {
            let (s, t) = (g.edge_source(e).unwrap(), g.edge_target(e).unwrap());
            assert!(h.contains_edge_between(m.vertex(s).unwrap(), m.vertex(t).unwrap()));
            assert!(m.edge(e).is_some());
        }
        assert_eq!(m.vertex_forward.len(), 9);
        // removing an edge breaks it
        let mut h2 = h.clone();
        let first = h2.edges().next().cloned();
        if let Some(e) = first {
            h2.remove_edge(&e);
            assert!(!is_isomorphic(&g, &h2));
        }
    }
}

#[test]
fn automorphism_counts() {
    assert_eq!(
        count_isomorphisms(
            &build(UGraph::<usize>::simple(), Ring(5)),
            &build(UGraph::<usize>::simple(), Ring(5))
        ),
        10
    );
    let k4 = build(UGraph::<usize>::simple(), Complete(4));
    assert_eq!(count_isomorphisms(&k4, &k4), 24);
    let star = build(UGraph::<usize>::simple(), Star(5));
    let path = build(UGraph::<usize>::simple(), Linear(5));
    assert!(!is_isomorphic(&star, &path));
    // directed cycle vs directed path orientation
    let c3 = build(DiGraph::<usize>::simple(), Ring(3));
    assert_eq!(count_isomorphisms(&c3, &c3), 3);
    assert!(!is_isomorphic(
        &c3,
        &build(UGraph::<usize>::simple(), Ring(3))
    ));
}

#[test]
fn subgraphs() {
    let c5 = build(UGraph::<usize>::simple(), Ring(5));
    let p3 = build(UGraph::<usize>::simple(), Linear(3));
    let k4 = build(UGraph::<usize>::simple(), Complete(4));
    let tri = build(UGraph::<usize>::simple(), Complete(3));
    assert_eq!(count_induced_subgraphs(&p3, &c5), 10);
    assert_eq!(count_subgraphs(&p3, &c5), 10);
    assert_eq!(count_induced_subgraphs(&p3, &k4), 0);
    assert_eq!(count_subgraphs(&p3, &k4), 24);
    assert!(find_induced_subgraph(&tri, &k4).is_some());
    assert!(find_subgraph(&tri, &c5).is_none());
}
