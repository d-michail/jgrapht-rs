use jgrapht::alg::metrics::*;
use jgrapht::alg::scoring::*;
use jgrapht::generate::*;
use jgrapht::*;

fn build<G: GraphMut<Vertex = usize>>(mut g: G, mut f: impl GraphGenerator) -> G {
    f.generate_indexed(&mut g).unwrap();
    g
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn centralities_on_star_and_path() {
    let star = build(UGraph::<usize>::simple(), Star(6)); // hub 0, 5 leaves
    let b = betweenness_centrality(&star, false, false);
    assert!(close(b.vertices[&0], 10.0)); // C(5,2)
    assert!(close(b.vertices[&1], 0.0));
    assert!(close(
        betweenness_centrality(&star, false, true).vertices[&0],
        1.0
    ));
    assert!(close(
        b.edges.values().copied().sum::<f64>(),
        25.0
    )); // each leaf pair path + direct
    let wb = betweenness_centrality(&star, true, false);
    assert!(close(wb.vertices[&0], 10.0));
    let d = degree_centrality(&star, true);
    assert!(close(d[&0], 1.0) && close(d[&2], 0.2));
    let c = closeness_centrality(&star, false);
    assert!(c[&0] > c[&1]);
    let h = harmonic_centrality(&star, false);
    assert!(close(h[&0], 5.0) && close(h[&1], 1.0 + 4.0 * 0.5));
    let path = build(UGraph::<usize>::simple(), Linear(3));
    assert!(close(
        betweenness_centrality(&path, false, false).vertices[&1],
        1.0
    ));
}

#[test]
fn pagerank_and_eigenvector() {
    let mut g = DiGraph::<u32>::simple();
    for (a, b) in [(1, 2), (2, 3), (3, 1), (4, 3)] {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    let pr = pagerank(&g, 0.85, 200, 1e-12);
    assert!(close(pr.values().sum::<f64>(), 1.0));
    assert!(pr[&3] > pr[&4]);
    let ring = build(UGraph::<usize>::simple(), Ring(6));
    let ev = eigenvector_centrality(&ring, 200, 1e-12);
    assert!(close(ev[&0], ev[&3]));
    let k = katz_centrality(&ring, 0.1, 1.0, 200, 1e-12);
    assert!(close(k[&0], 1.0 / (1.0 - 0.2)));
}

#[test]
fn core_and_clustering() {
    let mut g = UGraph::<u32>::simple();
    for (a, b) in [(1, 2), (2, 3), (3, 1), (3, 4)] {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    let core = coreness(&g);
    assert_eq!((core[&1], core[&4]), (2, 1));
    assert_eq!(count_triangles(&g), 1);
    let lc = local_clustering_coefficients(&g);
    assert!(close(lc[&1], 1.0) && close(lc[&3], 1.0 / 3.0) && close(lc[&4], 0.0));
    assert!(close(global_clustering_coefficient(&g), 3.0 / 5.0));
    assert!(close(
        average_clustering_coefficient(&g),
        (1.0 + 1.0 + 1.0 / 3.0) / 4.0
    ));
}

#[test]
fn metrics() {
    let path = build(UGraph::<usize>::simple(), Linear(5));
    assert_eq!((diameter(&path, false), radius(&path, false)), (4.0, 2.0));
    assert_eq!(center(&path, false), [2]);
    let mut p = periphery(&path, false);
    p.sort();
    assert_eq!(p, [0, 4]);
    assert_eq!(girth(&path), None);
    assert_eq!(girth(&build(UGraph::<usize>::simple(), Ring(7))), Some(7));
    assert_eq!(
        girth(&build(UGraph::<usize>::simple(), Complete(5))),
        Some(3)
    );
    let disc = build(UGraph::<usize>::simple(), Empty(2));
    assert!(diameter(&disc, false).is_infinite() || diameter(&disc, false) == 0.0);
}
