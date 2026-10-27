//! Ports of the `org.jgrapht.alg.scoring` tests.

use jgrapht::alg::scoring::*;
use jgrapht::generate::*;
use jgrapht::*;

fn s(x: &str) -> String {
    x.to_string()
}

fn close(a: f64, b: f64) {
    if a.is_nan() || b.is_nan() {
        assert!(a.is_nan() && b.is_nan(), "{a} vs {b}");
    } else if a.is_infinite() || b.is_infinite() {
        assert_eq!(a, b);
    } else {
        assert!((a - b).abs() < 1e-9, "{a} vs {b}");
    }
}

/// The directed pseudograph shared by the closeness and harmonic tests.
fn instance1() -> DiGraph<String> {
    let mut g = DiGraph::<String>::pseudo();
    g.add_vertices((1..=5).map(|i| i.to_string()));
    for (a, b) in [
        ("1", "2"),
        ("1", "3"),
        ("2", "3"),
        ("3", "4"),
        ("4", "1"),
        ("4", "5"),
        ("5", "3"),
    ] {
        g.add_edge(&s(a), &s(b)).unwrap();
    }
    g
}

fn negative_weights_instance() -> DiGraph<String> {
    let mut g = DiGraph::<String>::pseudo().weighted();
    g.add_vertices((1..=5).map(|i| i.to_string()));
    let mut e13 = None;
    for (a, b) in [
        ("1", "2"),
        ("1", "3"),
        ("2", "3"),
        ("3", "4"),
        ("4", "1"),
        ("4", "5"),
        ("5", "3"),
    ] {
        let e = g.add_edge(&s(a), &s(b)).unwrap().unwrap();
        if (a, b) == ("1", "3") {
            e13 = Some(e);
        }
    }
    g.set_edge_weight(&e13.unwrap(), -1.0).unwrap();
    g
}

fn check<A: VertexScoringAlgorithm<String>>(alg: &A, expected: &[(&str, f64)]) {
    for (v, x) in expected {
        close(alg.vertex_score(&s(v)).unwrap(), *x);
    }
}

mod closeness_centrality {
    use super::*;

    #[test]
    fn test_outgoing() {
        let g = instance1();
        let pr = ClosenessCentrality::with_options(&g, false, true);
        check(
            &pr,
            &[
                ("1", 4. / 7.),
                ("2", 4. / 9.),
                ("3", 4. / 8.),
                ("4", 4. / 6.),
                ("5", 4. / 10.),
            ],
        );
    }

    #[test]
    fn test_incoming() {
        let g = instance1();
        let pr = ClosenessCentrality::with_options(&g, true, true);
        check(
            &pr,
            &[
                ("1", 4. / 9.),
                ("2", 4. / 10.),
                ("3", 4. / 5.),
                ("4", 4. / 7.),
                ("5", 4. / 9.),
            ],
        );
    }

    #[test]
    fn test_incoming_no_normalization() {
        let g = instance1();
        let pr = ClosenessCentrality::with_options(&g, true, false);
        check(
            &pr,
            &[
                ("1", 1. / 9.),
                ("2", 1. / 10.),
                ("3", 1. / 5.),
                ("4", 1. / 7.),
                ("5", 1. / 9.),
            ],
        );
    }

    #[test]
    fn test_undirected() {
        let d = instance1();
        let g = AsUndirected(&d);
        let want = [
            ("1", 4. / 5.),
            ("2", 4. / 6.),
            ("3", 4. / 4.),
            ("4", 4. / 5.),
            ("5", 4. / 6.),
        ];
        check(&ClosenessCentrality::with_options(&g, true, true), &want);
        check(&ClosenessCentrality::with_options(&g, false, true), &want);
    }

    #[test]
    fn test_negative_weights() {
        let g = negative_weights_instance();
        let pr = ClosenessCentrality::with_options(&g, false, true);
        check(
            &pr,
            &[
                ("1", 4. / 1.),
                ("2", 4. / 9.),
                ("3", 4. / 8.),
                ("4", 4. / 4.),
                ("5", 4. / 10.),
            ],
        );
    }

    #[test]
    fn test_disconnected_outgoing() {
        let mut g = instance1();
        g.add_vertex(s("6"));
        let pr = ClosenessCentrality::with_options(&g, false, true);
        check(
            &pr,
            &[
                ("1", 0.),
                ("2", 0.),
                ("3", 0.),
                ("4", 0.),
                ("5", 0.),
                ("6", 0.),
            ],
        );
    }

    #[test]
    fn test_singleton_with_normalize() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("1"));
        close(
            ClosenessCentrality::with_options(&g, false, true)
                .vertex_score(&s("1"))
                .unwrap(),
            f64::NAN,
        );
    }

    #[test]
    fn test_singleton_without_normalize() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("1"));
        close(
            ClosenessCentrality::with_options(&g, false, false)
                .vertex_score(&s("1"))
                .unwrap(),
            f64::INFINITY,
        );
    }
}

mod harmonic_centrality {
    use super::*;

    fn h(parts: &[f64]) -> f64 {
        parts.iter().map(|d| 1.0 / d).sum()
    }

    #[test]
    fn test_outgoing() {
        let g = instance1();
        let pr = HarmonicCentrality::with_options(&g, false, true);
        check(
            &pr,
            &[
                ("1", h(&[1., 1., 2., 3.]) / 4.),
                ("2", h(&[3., 1., 2., 3.]) / 4.),
                ("3", h(&[2., 3., 1., 2.]) / 4.),
                ("4", h(&[1., 2., 2., 1.]) / 4.),
                ("5", h(&[3., 4., 1., 2.]) / 4.),
            ],
        );
    }

    #[test]
    fn test_incoming() {
        let g = instance1();
        let pr = HarmonicCentrality::with_options(&g, true, true);
        check(
            &pr,
            &[
                ("1", h(&[3., 2., 1., 3.]) / 4.),
                ("2", h(&[1., 3., 2., 4.]) / 4.),
                ("3", h(&[1., 1., 2., 1.]) / 4.),
                ("4", h(&[2., 2., 1., 2.]) / 4.),
                ("5", h(&[3., 3., 2., 1.]) / 4.),
            ],
        );
    }

    #[test]
    fn test_incoming_no_normalization() {
        let g = instance1();
        let pr = HarmonicCentrality::with_options(&g, true, false);
        check(
            &pr,
            &[
                ("1", h(&[3., 2., 1., 3.])),
                ("2", h(&[1., 3., 2., 4.])),
                ("3", h(&[1., 1., 2., 1.])),
                ("4", h(&[2., 2., 1., 2.])),
                ("5", h(&[3., 3., 2., 1.])),
            ],
        );
    }

    #[test]
    fn test_undirected() {
        let d = instance1();
        let g = AsUndirected(&d);
        let want = [
            ("1", h(&[1., 1., 1., 2.]) / 4.),
            ("2", h(&[1., 1., 2., 2.]) / 4.),
            ("3", h(&[1., 1., 1., 1.]) / 4.),
            ("4", h(&[1., 2., 1., 1.]) / 4.),
            ("5", h(&[2., 2., 1., 1.]) / 4.),
        ];
        check(&HarmonicCentrality::with_options(&g, true, true), &want);
        check(&HarmonicCentrality::with_options(&g, false, true), &want);
    }

    #[test]
    fn test_negative_weights() {
        let g = negative_weights_instance();
        let pr = HarmonicCentrality::with_options(&g, false, true);
        check(
            &pr,
            &[
                ("1", f64::INFINITY),
                ("2", h(&[3., 1., 2., 3.]) / 4.),
                ("3", h(&[2., 3., 1., 2.]) / 4.),
                ("4", f64::INFINITY),
                ("5", h(&[3., 4., 1., 2.]) / 4.),
            ],
        );
    }

    #[test]
    fn test_disconnected_outgoing() {
        let mut g = instance1();
        g.add_vertex(s("6"));
        let pr = HarmonicCentrality::with_options(&g, false, true);
        check(
            &pr,
            &[
                ("1", h(&[1., 1., 2., 3.]) / 5.),
                ("2", h(&[3., 1., 2., 3.]) / 5.),
                ("3", h(&[2., 3., 1., 2.]) / 5.),
                ("4", h(&[1., 2., 2., 1.]) / 5.),
                ("5", h(&[3., 4., 1., 2.]) / 5.),
                ("6", 0.),
            ],
        );
    }

    #[test]
    fn test_singleton() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("1"));
        for normalize in [true, false] {
            close(
                HarmonicCentrality::with_options(&g, false, normalize)
                    .vertex_score(&s("1"))
                    .unwrap(),
                0.0,
            );
        }
    }
}

mod coreness {
    use super::*;

    fn ugraph(vertices: &[&str], edges: &[(&str, &str)]) -> UGraph<String> {
        let mut g = UGraph::<String>::simple();
        g.add_vertices(vertices.iter().map(|v| s(v)));
        for (a, b) in edges {
            g.add_edge(&s(a), &s(b)).unwrap();
        }
        g
    }

    fn check_scores(pr: &Coreness<String>, expected: &[(&str, usize)]) {
        for (v, c) in expected {
            assert_eq!(pr.vertex_score(&s(v)).unwrap(), *c);
        }
    }

    #[test]
    fn test_graph() {
        let g = ugraph(
            &["a", "b", "c", "d", "e", "f", "g", "h"],
            &[
                ("b", "c"),
                ("c", "d"),
                ("c", "e"),
                ("e", "f"),
                ("e", "g"),
                ("e", "h"),
                ("f", "g"),
                ("f", "h"),
                ("g", "h"),
            ],
        );
        let pr = Coreness::new(&g).unwrap();
        check_scores(
            &pr,
            &[
                ("a", 0),
                ("b", 1),
                ("c", 1),
                ("d", 1),
                ("e", 3),
                ("f", 3),
                ("g", 3),
                ("h", 3),
            ],
        );
        assert_eq!(pr.degeneracy(), 3);
    }

    #[test]
    fn test_another_graph() {
        let g = ugraph(
            &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"],
            &[
                ("b", "c"),
                ("c", "d"),
                ("c", "e"),
                ("e", "f"),
                ("e", "g"),
                ("e", "h"),
                ("f", "g"),
                ("f", "h"),
                ("f", "i"),
                ("g", "h"),
                ("i", "j"),
                ("i", "k"),
                ("j", "k"),
            ],
        );
        let pr = Coreness::new(&g).unwrap();
        check_scores(
            &pr,
            &[
                ("a", 0),
                ("b", 1),
                ("c", 1),
                ("d", 1),
                ("e", 3),
                ("f", 3),
                ("g", 3),
                ("h", 3),
                ("i", 2),
                ("j", 2),
                ("k", 2),
            ],
        );
        assert_eq!(pr.degeneracy(), 3);
    }

    #[test]
    fn test_singleton_graph() {
        let pr = Coreness::new(&ugraph(&["a"], &[])).unwrap();
        assert_eq!(pr.vertex_score(&s("a")).unwrap(), 0);
        assert_eq!(pr.degeneracy(), 0);
    }

    #[test]
    fn test_empty_graph() {
        assert!(
            Coreness::new(&ugraph(&[], &[]))
                .unwrap()
                .scores()
                .is_empty()
        );
    }

    #[test]
    fn test_non_existant_vertex() {
        let pr = Coreness::new(&ugraph(&["a"], &[])).unwrap();
        assert!(pr.vertex_score(&s("unknown")).is_err());
    }

    #[test]
    fn test_rejects_directed_and_non_simple() {
        assert!(Coreness::new(&DiGraph::<String>::simple()).is_err());
        let mut g = UGraph::<String>::multi();
        g.add_vertices([s("a"), s("b")]);
        g.add_edge(&s("a"), &s("b")).unwrap();
        g.add_edge(&s("a"), &s("b")).unwrap();
        assert!(Coreness::new(&g).is_err());
    }
}

mod clustering_coefficient {
    use super::*;

    fn undirected(n: usize, edges: &[(usize, usize)]) -> UGraph<String> {
        let mut g = UGraph::<String>::simple();
        g.add_vertices((0..n).map(|i| i.to_string()));
        for (a, b) in edges {
            g.add_edge(&a.to_string(), &b.to_string()).unwrap();
        }
        g
    }

    fn directed(n: usize, edges: &[(usize, usize)]) -> DiGraph<String> {
        let mut g = DiGraph::<String>::simple();
        g.add_vertices((0..n).map(|i| i.to_string()));
        for (a, b) in edges {
            g.add_edge(&a.to_string(), &b.to_string()).unwrap();
        }
        g
    }

    fn near(a: f64, b: f64, eps: f64) {
        assert!((a - b).abs() <= eps, "{a} vs {b}");
    }

    #[test]
    fn test_undirected_clustering_coefficient() {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices(1..=8);
        for (a, b) in [
            (1, 2),
            (1, 3),
            (2, 3),
            (2, 4),
            (3, 4),
            (4, 5),
            (4, 6),
            (5, 7),
            (6, 7),
            (2, 8),
            (8, 5),
        ] {
            g.add_edge(&a, &b).unwrap();
        }
        let cc = ClusteringCoefficient::new(&g);
        assert_eq!(cc.vertex_score(&1).unwrap(), 1.0);
        near(cc.vertex_score(&2).unwrap(), 0.333333333, 1e-4);
        near(cc.vertex_score(&3).unwrap(), 0.666666666, 1e-4);
        near(cc.vertex_score(&4).unwrap(), 0.166666666, 1e-4);
        for v in 5..=8 {
            assert_eq!(cc.vertex_score(&v).unwrap(), 0.0);
        }
    }

    #[test]
    fn test_undirected_2() {
        let mut g = UGraph::<String>::simple();
        g.add_vertices(["A", "B", "C", "D"].map(s));
        for (a, b) in [("A", "B"), ("A", "C"), ("A", "D"), ("B", "C")] {
            g.add_edge(&s(a), &s(b)).unwrap();
        }
        near(
            ClusteringCoefficient::new(&g)
                .vertex_score(&s("A"))
                .unwrap(),
            1.0 / 3.0,
            0.001,
        );
    }

    #[test]
    fn test_one_node() {
        let mut g = UGraph::<String>::simple();
        g.add_vertex(s("A"));
        assert_eq!(
            ClusteringCoefficient::new(&g).average_clustering_coefficient(),
            0.0
        );
    }

    #[test]
    fn test_two_connected_nodes() {
        let g = undirected(2, &[(0, 1)]);
        assert_eq!(
            ClusteringCoefficient::new(&g).average_clustering_coefficient(),
            0.0
        );
    }

    #[test]
    fn test_complete_graph() {
        let mut g = UGraph::<usize>::simple().with_vertex_factory(|i| i as usize);
        Complete(100).generate_default(&mut g).unwrap();
        assert_eq!(
            ClusteringCoefficient::new(&g).average_clustering_coefficient(),
            1.0
        );
    }

    #[test]
    fn test_star_graph() {
        let mut g = UGraph::<usize>::simple().with_vertex_factory(|i| i as usize);
        Star(100).generate_default(&mut g).unwrap();
        assert_eq!(
            ClusteringCoefficient::new(&g).average_clustering_coefficient(),
            0.0
        );
    }

    #[test]
    fn test_triangle_directed() {
        let g = directed(3, &[(0, 1), (1, 0), (1, 2), (2, 1), (2, 0), (0, 2)]);
        assert_eq!(
            ClusteringCoefficient::new(&g).average_clustering_coefficient(),
            1.0
        );
    }

    #[test]
    fn test_special_1_directed() {
        let g = directed(4, &[(0, 1), (1, 2), (1, 3), (2, 0), (2, 3), (3, 0)]);
        assert_eq!(
            ClusteringCoefficient::new(&g).average_clustering_coefficient(),
            0.5
        );
    }

    #[test]
    fn test_special_2_directed() {
        let g = directed(4, &[(1, 0), (1, 3), (2, 0), (2, 1), (3, 2)]);
        near(
            ClusteringCoefficient::new(&g).average_clustering_coefficient(),
            0.4167,
            0.01,
        );
    }

    #[test]
    fn test_triangle_non_complete_directed() {
        let g = directed(3, &[(0, 1), (1, 0), (1, 2), (2, 1), (0, 2)]);
        near(
            ClusteringCoefficient::new(&g).average_clustering_coefficient(),
            0.833,
            0.01,
        );
    }

    #[test]
    fn test_triangle_graph() {
        let g = undirected(3, &[(0, 1), (1, 2), (2, 0)]);
        assert_eq!(
            ClusteringCoefficient::new(&g).average_clustering_coefficient(),
            1.0
        );
    }

    #[test]
    fn test_special_1_undirected() {
        let g = undirected(
            7,
            &[
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (0, 6),
                (1, 2),
                (2, 3),
                (3, 4),
                (4, 5),
                (5, 6),
                (6, 1),
            ],
        );
        let cc = ClusteringCoefficient::new(&g);
        assert_eq!(cc.vertex_score(&s("0")).unwrap(), 0.4);
        near(cc.vertex_score(&s("2")).unwrap(), 0.667, 0.001);
    }

    #[test]
    fn test_special_2_undirected() {
        let g = undirected(
            7,
            &[
                (0, 1),
                (1, 2),
                (2, 0),
                (0, 3),
                (3, 4),
                (4, 0),
                (0, 5),
                (5, 6),
                (6, 0),
            ],
        );
        near(
            ClusteringCoefficient::new(&g).average_clustering_coefficient(),
            0.8857,
            0.01,
        );
    }

    #[test]
    fn test_special_3_undirected() {
        let g = undirected(6, &[(0, 1), (1, 2), (2, 0), (0, 3), (1, 4), (2, 5)]);
        near(
            ClusteringCoefficient::new(&g)
                .vertex_score(&s("0"))
                .unwrap(),
            0.333,
            0.01,
        );
    }
}

mod page_rank {
    use super::*;

    #[test]
    fn test_graph_2_nodes() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("1"));
        g.add_vertex(s("2"));
        g.add_edge(&s("1"), &s("2")).unwrap();
        g.add_edge(&s("2"), &s("1")).unwrap();
        let pr = PageRank::new(&g);
        near(
            pr.vertex_score(&s("1")).unwrap(),
            pr.vertex_score(&s("2")).unwrap(),
            0.0001,
        );
    }

    #[test]
    fn test_graph_3_nodes() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("1"));
        g.add_vertex(s("2"));
        g.add_vertex(s("3"));
        g.add_edge(&s("1"), &s("2")).unwrap();
        g.add_edge(&s("2"), &s("3")).unwrap();
        g.add_edge(&s("3"), &s("1")).unwrap();
        let pr = PageRank::new(&g);
        near(
            pr.vertex_score(&s("1")).unwrap(),
            pr.vertex_score(&s("2")).unwrap(),
            0.0001,
        );
        near(
            pr.vertex_score(&s("1")).unwrap(),
            pr.vertex_score(&s("3")).unwrap(),
            0.0001,
        );
    }

    #[test]
    fn test_graph_wikipedia() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("A"));
        g.add_vertex(s("B"));
        g.add_vertex(s("C"));
        g.add_vertex(s("D"));
        g.add_vertex(s("E"));
        g.add_vertex(s("F"));
        g.add_vertex(s("1"));
        g.add_vertex(s("2"));
        g.add_vertex(s("3"));
        g.add_vertex(s("4"));
        g.add_vertex(s("5"));
        g.add_edge(&s("B"), &s("C")).unwrap();
        g.add_edge(&s("C"), &s("B")).unwrap();
        g.add_edge(&s("D"), &s("A")).unwrap();
        g.add_edge(&s("D"), &s("B")).unwrap();
        g.add_edge(&s("E"), &s("D")).unwrap();
        g.add_edge(&s("E"), &s("B")).unwrap();
        g.add_edge(&s("E"), &s("F")).unwrap();
        g.add_edge(&s("F"), &s("B")).unwrap();
        g.add_edge(&s("F"), &s("E")).unwrap();
        g.add_edge(&s("1"), &s("B")).unwrap();
        g.add_edge(&s("1"), &s("E")).unwrap();
        g.add_edge(&s("2"), &s("B")).unwrap();
        g.add_edge(&s("2"), &s("E")).unwrap();
        g.add_edge(&s("3"), &s("B")).unwrap();
        g.add_edge(&s("3"), &s("E")).unwrap();
        g.add_edge(&s("4"), &s("E")).unwrap();
        g.add_edge(&s("5"), &s("E")).unwrap();
        let pr = PageRank::new(&g);
        near(pr.vertex_score(&s("A")).unwrap(), 0.03278, 0.0001);
        near(pr.vertex_score(&s("B")).unwrap(), 0.38435, 0.0001);
        near(pr.vertex_score(&s("C")).unwrap(), 0.34295, 0.0001);
        near(pr.vertex_score(&s("D")).unwrap(), 0.03908, 0.0001);
        near(pr.vertex_score(&s("E")).unwrap(), 0.08088, 0.0001);
        near(pr.vertex_score(&s("F")).unwrap(), 0.03908, 0.0001);
        near(pr.vertex_score(&s("1")).unwrap(), 0.01616, 0.0001);
        near(pr.vertex_score(&s("2")).unwrap(), 0.01616, 0.0001);
        near(pr.vertex_score(&s("3")).unwrap(), 0.01616, 0.0001);
        near(pr.vertex_score(&s("4")).unwrap(), 0.01616, 0.0001);
        near(pr.vertex_score(&s("5")).unwrap(), 0.01616, 0.0001);
    }

    #[test]
    fn test_undirected_graph_wikipedia() {
        let mut g = UGraph::<String>::pseudo();
        g.add_vertex(s("A"));
        g.add_vertex(s("B"));
        g.add_vertex(s("C"));
        g.add_vertex(s("D"));
        g.add_vertex(s("E"));
        g.add_vertex(s("F"));
        g.add_vertex(s("1"));
        g.add_vertex(s("2"));
        g.add_vertex(s("3"));
        g.add_vertex(s("4"));
        g.add_vertex(s("5"));
        g.add_edge(&s("B"), &s("C")).unwrap();
        g.add_edge(&s("C"), &s("B")).unwrap();
        g.add_edge(&s("D"), &s("A")).unwrap();
        g.add_edge(&s("D"), &s("B")).unwrap();
        g.add_edge(&s("E"), &s("D")).unwrap();
        g.add_edge(&s("E"), &s("B")).unwrap();
        g.add_edge(&s("E"), &s("F")).unwrap();
        g.add_edge(&s("F"), &s("B")).unwrap();
        g.add_edge(&s("F"), &s("E")).unwrap();
        g.add_edge(&s("1"), &s("B")).unwrap();
        g.add_edge(&s("1"), &s("E")).unwrap();
        g.add_edge(&s("2"), &s("B")).unwrap();
        g.add_edge(&s("2"), &s("E")).unwrap();
        g.add_edge(&s("3"), &s("B")).unwrap();
        g.add_edge(&s("3"), &s("E")).unwrap();
        g.add_edge(&s("4"), &s("E")).unwrap();
        g.add_edge(&s("5"), &s("E")).unwrap();
        let pr = PageRank::new(&g);
        near(pr.vertex_score(&s("A")).unwrap(), 0.0404, 0.0001);
        near(pr.vertex_score(&s("B")).unwrap(), 0.2152, 0.0001);
        near(pr.vertex_score(&s("C")).unwrap(), 0.0593, 0.0001);
        near(pr.vertex_score(&s("D")).unwrap(), 0.0945, 0.0001);
        near(pr.vertex_score(&s("E")).unwrap(), 0.2511, 0.0001);
        near(pr.vertex_score(&s("F")).unwrap(), 0.0839, 0.0001);
        near(pr.vertex_score(&s("1")).unwrap(), 0.0602, 0.0001);
        near(pr.vertex_score(&s("2")).unwrap(), 0.0602, 0.0001);
        near(pr.vertex_score(&s("3")).unwrap(), 0.0602, 0.0001);
        near(pr.vertex_score(&s("4")).unwrap(), 0.0373, 0.0001);
        near(pr.vertex_score(&s("5")).unwrap(), 0.0373, 0.0001);
    }

    #[test]
    fn test_weighted_graph_1() {
        let mut g = DiGraph::<String>::pseudo().weighted();
        g.add_vertex(s("center"));
        g.add_vertex(s("a"));
        g.add_vertex(s("b"));
        g.add_vertex(s("c"));
        add_weighted(&mut g, "center", "a", 75.0);
        add_weighted(&mut g, "center", "b", 20.0);
        add_weighted(&mut g, "center", "c", 5.0);
        let pr = PageRank::with_options(&g, 0.85, 100, 0.0001).unwrap();
        near(pr.vertex_score(&s("center")).unwrap(), 0.2061, 0.0001);
        near(pr.vertex_score(&s("a")).unwrap(), 0.3376, 0.0001);
        near(pr.vertex_score(&s("b")).unwrap(), 0.2412, 0.0001);
        near(pr.vertex_score(&s("c")).unwrap(), 0.2149, 0.0001);
    }

    #[test]
    fn test_empty_graph() {
        let mut g = DiGraph::<String>::pseudo();
        let pr = PageRank::with_options(&g, 0.85, 100, 0.0001).unwrap();
        assert!(pr.scores().is_empty());
    }

    #[test]
    fn test_non_existant_vertex() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertices(["center", "a", "b", "c", "d"].map(s));
        let pr = PageRank::new(&g);
        assert!(pr.vertex_score(&s("unknown")).is_err());
    }

    #[test]
    fn test_bad_parameters() {
        let g = DiGraph::<String>::pseudo();
        assert!(PageRank::with_options(&g, 1.1, 100, 0.0001).is_err());
        assert!(PageRank::with_options(&g, 0.85, 0, 0.0001).is_err());
        assert!(PageRank::with_options(&g, 0.85, 100, 0.0).is_err());
    }
}

mod eigenvector_centrality {
    use super::*;

    #[test]
    fn test_graph_2_nodes() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("1"));
        g.add_vertex(s("2"));
        g.add_edge(&s("1"), &s("2")).unwrap();
        g.add_edge(&s("2"), &s("1")).unwrap();
        let pr = EigenvectorCentrality::new(&g);
        near(
            pr.vertex_score(&s("1")).unwrap(),
            1.0 / f64::sqrt(2.0),
            0.0001,
        );
        near(
            pr.vertex_score(&s("2")).unwrap(),
            1.0 / f64::sqrt(2.0),
            0.0001,
        );
    }

    #[test]
    fn test_graph_3_nodes() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("1"));
        g.add_vertex(s("2"));
        g.add_vertex(s("3"));
        g.add_edge(&s("1"), &s("2")).unwrap();
        g.add_edge(&s("2"), &s("3")).unwrap();
        g.add_edge(&s("3"), &s("1")).unwrap();
        let pr = EigenvectorCentrality::new(&g);
        near(
            pr.vertex_score(&s("1")).unwrap(),
            1.0 / f64::sqrt(3.0),
            0.0001,
        );
        near(
            pr.vertex_score(&s("2")).unwrap(),
            1.0 / f64::sqrt(3.0),
            0.0001,
        );
        near(
            pr.vertex_score(&s("3")).unwrap(),
            1.0 / f64::sqrt(3.0),
            0.0001,
        );
    }

    #[test]
    fn test_graph_1() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("0"));
        g.add_vertex(s("1"));
        g.add_vertex(s("2"));
        g.add_vertex(s("3"));
        g.add_vertex(s("4"));
        g.add_edge(&s("0"), &s("1")).unwrap();
        g.add_edge(&s("0"), &s("2")).unwrap();
        g.add_edge(&s("1"), &s("3")).unwrap();
        g.add_edge(&s("1"), &s("2")).unwrap();
        g.add_edge(&s("2"), &s("1")).unwrap();
        g.add_edge(&s("2"), &s("4")).unwrap();
        g.add_edge(&s("2"), &s("3")).unwrap();
        g.add_edge(&s("3"), &s("1")).unwrap();
        g.add_edge(&s("3"), &s("2")).unwrap();
        g.add_edge(&s("3"), &s("3")).unwrap();
        g.add_edge(&s("4"), &s("2")).unwrap();
        g.add_edge(&s("4"), &s("0")).unwrap();
        let pr = EigenvectorCentrality::new(&g);
        near(
            pr.vertex_score(&s("0")).unwrap(),
            0.08032022089204849,
            0.001,
        );
        near(
            pr.vertex_score(&s("1")).unwrap(),
            0.48765632797141506,
            0.001,
        );
        near(pr.vertex_score(&s("2")).unwrap(), 0.5453987490787013, 0.001);
        near(pr.vertex_score(&s("3")).unwrap(), 0.6437087676602127, 0.001);
        near(
            pr.vertex_score(&s("4")).unwrap(),
            0.20956906939251885,
            0.001,
        );
    }

    #[test]
    fn test_weighted_graph_1() {
        let mut g = DiGraph::<String>::pseudo().weighted();
        g.add_vertex(s("a"));
        g.add_vertex(s("b"));
        g.add_vertex(s("c"));
        g.add_vertex(s("d"));
        add_weighted(&mut g, "a", "b", 1. / 2.0);
        add_weighted(&mut g, "a", "c", 1. / 3.0);
        add_weighted(&mut g, "b", "a", 1.0);
        add_weighted(&mut g, "b", "b", 2.0);
        add_weighted(&mut g, "b", "d", 1. / 4.0);
        add_weighted(&mut g, "c", "a", 1.0);
        add_weighted(&mut g, "c", "d", 3.0);
        add_weighted(&mut g, "d", "b", 1. / 5.0);
        add_weighted(&mut g, "d", "d", 1.0);
        let pr = EigenvectorCentrality::new(&g);
        near(pr.vertex_score(&s("a")).unwrap(), 0.400610775759173, 0.0001);
        near(pr.vertex_score(&s("b")).unwrap(), 0.863882834704165, 0.0001);
        near(
            pr.vertex_score(&s("c")).unwrap(),
            0.0580276877361552,
            0.0001,
        );
        near(pr.vertex_score(&s("d")).unwrap(), 0.299750298600000, 0.0001);
    }

    #[test]
    fn test_non_existant_vertex() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertices(["center", "a", "b", "c", "d"].map(s));
        let pr = EigenvectorCentrality::new(&g);
        assert!(pr.vertex_score(&s("unknown")).is_err());
    }

    #[test]
    fn test_bad_parameters_1() {
        assert!(
            EigenvectorCentrality::with_options(&DiGraph::<String>::pseudo(), 0, 0.0001).is_err()
        );
    }

    #[test]
    fn test_bad_parameters_2() {
        assert!(EigenvectorCentrality::with_options(&DiGraph::<String>::pseudo(), 1, 0.0).is_err());
    }
}

mod katz_centrality {
    use super::*;

    #[test]
    fn test_graph_2_nodes() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("1"));
        g.add_vertex(s("2"));
        g.add_edge(&s("1"), &s("2")).unwrap();
        g.add_edge(&s("2"), &s("1")).unwrap();
        let pr = KatzCentrality::new(&g);
        near(
            pr.vertex_score(&s("1")).unwrap(),
            pr.vertex_score(&s("2")).unwrap(),
            0.0001,
        );
    }

    #[test]
    fn test_graph_3_nodes() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("1"));
        g.add_vertex(s("2"));
        g.add_vertex(s("3"));
        g.add_edge(&s("1"), &s("2")).unwrap();
        g.add_edge(&s("2"), &s("3")).unwrap();
        g.add_edge(&s("3"), &s("1")).unwrap();
        let pr = KatzCentrality::new(&g);
        near(
            pr.vertex_score(&s("1")).unwrap(),
            pr.vertex_score(&s("2")).unwrap(),
            0.0001,
        );
        near(
            pr.vertex_score(&s("1")).unwrap(),
            pr.vertex_score(&s("3")).unwrap(),
            0.0001,
        );
    }

    #[test]
    fn test_graph_1() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("A"));
        g.add_vertex(s("B"));
        g.add_vertex(s("C"));
        g.add_vertex(s("D"));
        g.add_vertex(s("E"));
        g.add_vertex(s("F"));
        g.add_vertex(s("1"));
        g.add_vertex(s("2"));
        g.add_vertex(s("3"));
        g.add_vertex(s("4"));
        g.add_vertex(s("5"));
        g.add_edge(&s("A"), &s("E")).unwrap();
        g.add_edge(&s("A"), &s("F")).unwrap();
        g.add_edge(&s("B"), &s("E")).unwrap();
        g.add_edge(&s("B"), &s("F")).unwrap();
        g.add_edge(&s("C"), &s("E")).unwrap();
        g.add_edge(&s("D"), &s("E")).unwrap();
        g.add_edge(&s("1"), &s("E")).unwrap();
        g.add_edge(&s("1"), &s("F")).unwrap();
        g.add_edge(&s("E"), &s("1")).unwrap();
        g.add_edge(&s("2"), &s("E")).unwrap();
        g.add_edge(&s("2"), &s("F")).unwrap();
        g.add_edge(&s("3"), &s("F")).unwrap();
        g.add_edge(&s("F"), &s("3")).unwrap();
        g.add_edge(&s("4"), &s("F")).unwrap();
        g.add_edge(&s("E"), &s("4")).unwrap();
        g.add_edge(&s("4"), &s("5")).unwrap();
        g.add_edge(&s("E"), &s("F")).unwrap();
        let pr = KatzCentrality::with_damping(&g, 0.85).unwrap();
        near(pr.vertex_score(&s("A")).unwrap(), 1.0000, 0.5);
        near(pr.vertex_score(&s("B")).unwrap(), 1.0000, 0.5);
        near(pr.vertex_score(&s("C")).unwrap(), 1.0000, 0.5);
        near(pr.vertex_score(&s("D")).unwrap(), 1.0000, 0.5);
        near(pr.vertex_score(&s("E")).unwrap(), 22.000, 0.5);
        near(pr.vertex_score(&s("F")).unwrap(), 204.00, 0.5);
        near(pr.vertex_score(&s("1")).unwrap(), 20.000, 0.5);
        near(pr.vertex_score(&s("2")).unwrap(), 1.0000, 0.5);
        near(pr.vertex_score(&s("3")).unwrap(), 174.00, 0.5);
        near(pr.vertex_score(&s("4")).unwrap(), 20.000, 0.5);
        near(pr.vertex_score(&s("5")).unwrap(), 18.000, 0.5);
    }

    #[test]
    fn test_graph_2() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("A"));
        g.add_vertex(s("B"));
        g.add_vertex(s("C"));
        g.add_vertex(s("D"));
        g.add_vertex(s("E"));
        g.add_vertex(s("F"));
        g.add_vertex(s("1"));
        g.add_vertex(s("2"));
        g.add_vertex(s("3"));
        g.add_vertex(s("4"));
        g.add_vertex(s("5"));
        g.add_edge(&s("A"), &s("E")).unwrap();
        g.add_edge(&s("A"), &s("F")).unwrap();
        g.add_edge(&s("B"), &s("E")).unwrap();
        g.add_edge(&s("B"), &s("F")).unwrap();
        g.add_edge(&s("C"), &s("E")).unwrap();
        g.add_edge(&s("D"), &s("E")).unwrap();
        g.add_edge(&s("1"), &s("E")).unwrap();
        g.add_edge(&s("1"), &s("F")).unwrap();
        g.add_edge(&s("E"), &s("1")).unwrap();
        g.add_edge(&s("2"), &s("E")).unwrap();
        g.add_edge(&s("2"), &s("F")).unwrap();
        g.add_edge(&s("3"), &s("F")).unwrap();
        g.add_edge(&s("F"), &s("3")).unwrap();
        g.add_edge(&s("4"), &s("F")).unwrap();
        g.add_edge(&s("E"), &s("4")).unwrap();
        g.add_edge(&s("4"), &s("5")).unwrap();
        g.add_edge(&s("E"), &s("F")).unwrap();
        let pr = KatzCentrality::with_damping(&g, 0.15).unwrap();
        near(pr.vertex_score(&s("A")).unwrap(), 1.0000, 0.05);
        near(pr.vertex_score(&s("B")).unwrap(), 1.0000, 0.05);
        near(pr.vertex_score(&s("C")).unwrap(), 1.0000, 0.05);
        near(pr.vertex_score(&s("D")).unwrap(), 1.0000, 0.05);
        near(pr.vertex_score(&s("E")).unwrap(), 1.9400, 0.05);
        near(pr.vertex_score(&s("F")).unwrap(), 2.3300, 0.05);
        near(pr.vertex_score(&s("1")).unwrap(), 1.2900, 0.05);
        near(pr.vertex_score(&s("2")).unwrap(), 1.0000, 0.05);
        near(pr.vertex_score(&s("3")).unwrap(), 1.3500, 0.05);
        near(pr.vertex_score(&s("4")).unwrap(), 1.2900, 0.05);
        near(pr.vertex_score(&s("5")).unwrap(), 1.1900, 0.05);
    }

    #[test]
    fn test_weighted_graph_1() {
        let mut g = DiGraph::<String>::pseudo().weighted();
        g.add_vertex(s("center"));
        g.add_vertex(s("a"));
        g.add_vertex(s("b"));
        g.add_vertex(s("c"));
        add_weighted(&mut g, "center", "a", 75.0);
        add_weighted(&mut g, "center", "b", 20.0);
        add_weighted(&mut g, "center", "c", 5.0);
        let pr = KatzCentrality::with_options(&g, 0.85, |_| 1.0, 100, 0.0001).unwrap();
        near(pr.vertex_score(&s("center")).unwrap(), 1.0000, 0.0001);
        near(pr.vertex_score(&s("a")).unwrap(), 64.7500, 0.0001);
        near(pr.vertex_score(&s("b")).unwrap(), 18.0000, 0.0001);
        near(pr.vertex_score(&s("c")).unwrap(), 5.2500, 0.0001);
    }

    #[test]
    fn testweighted_graph_2() {
        let mut g = DiGraph::<String>::pseudo().weighted();
        g.add_vertex(s("center"));
        g.add_vertex(s("a"));
        g.add_vertex(s("b"));
        g.add_vertex(s("c"));
        add_weighted(&mut g, "center", "a", 1.0);
        add_weighted(&mut g, "center", "b", 1.0);
        add_weighted(&mut g, "center", "c", 1.0);
        let pr = KatzCentrality::with_options(&g, 0.85, |_| 1.0, 100, 0.0001).unwrap();
        near(pr.vertex_score(&s("center")).unwrap(), 1.0000, 0.0001);
        near(pr.vertex_score(&s("a")).unwrap(), 1.8500, 0.0001);
        near(pr.vertex_score(&s("b")).unwrap(), 1.8500, 0.0001);
        near(pr.vertex_score(&s("c")).unwrap(), 1.8500, 0.0001);
    }

    #[test]
    fn test_unweighted_graph_2() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertex(s("center"));
        g.add_vertex(s("a"));
        g.add_vertex(s("b"));
        g.add_vertex(s("c"));
        g.add_vertex(s("d"));
        g.add_edge(&s("center"), &s("a")).unwrap();
        g.add_edge(&s("center"), &s("b")).unwrap();
        g.add_edge(&s("center"), &s("c")).unwrap();
        let pr = KatzCentrality::with_options(&g, 0.85, |_| 1.0, 100, 0.0001).unwrap();
        near(pr.vertex_score(&s("center")).unwrap(), 1.0000, 0.0001);
        near(pr.vertex_score(&s("a")).unwrap(), 1.8500, 0.0001);
        near(pr.vertex_score(&s("b")).unwrap(), 1.8500, 0.0001);
        near(pr.vertex_score(&s("c")).unwrap(), 1.8500, 0.0001);
        near(pr.vertex_score(&s("d")).unwrap(), 1.0000, 0.0001);
    }

    #[test]
    fn test_empty_graph() {
        let mut g = DiGraph::<String>::pseudo();
        let pr = KatzCentrality::with_options(&g, 0.85, |_| 1.0, 100, 0.0001).unwrap();
        assert!(pr.scores().is_empty());
    }

    #[test]
    fn test_exogenous_factor() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertices([s("1"), s("2")]);
        g.add_edge(&s("1"), &s("2")).unwrap();
        g.add_edge(&s("2"), &s("1")).unwrap();
        let pr = KatzCentrality::with_options(
            &g,
            0.5,
            |x| if x == "1" { 0.5 } else { 1.0 },
            100,
            0.0001,
        )
        .unwrap();
        near(pr.vertex_score(&s("1")).unwrap(), 4.0 / 3.0, 0.0001);
        near(pr.vertex_score(&s("2")).unwrap(), 5.0 / 3.0, 0.0001);
    }

    #[test]
    fn test_non_existant_vertex() {
        let pr =
            KatzCentrality::with_options(&DiGraph::<String>::pseudo(), 0.85, |_| 1.0, 100, 0.0001)
                .unwrap();
        assert!(pr.vertex_score(&s("unknown")).is_err());
    }

    #[test]
    fn test_bad_parameters_1() {
        assert!(
            KatzCentrality::with_options(&DiGraph::<String>::pseudo(), -1.0, |_| 1.0, 100, 0.0001)
                .is_err()
        );
    }

    #[test]
    fn test_bad_parameters_2() {
        assert!(
            KatzCentrality::with_options(&DiGraph::<String>::pseudo(), 0.85, |_| 1.0, 0, 0.0001)
                .is_err()
        );
    }

    #[test]
    fn test_bad_parameters_3() {
        assert!(
            KatzCentrality::with_options(&DiGraph::<String>::pseudo(), 0.85, |_| 1.0, 100, 0.0)
                .is_err()
        );
    }
}

fn near(a: f64, b: f64, eps: f64) {
    assert!((a - b).abs() <= eps, "{a} vs {b}");
}

fn add_weighted(g: &mut DiGraph<String>, a: &str, b: &str, w: f64) {
    let e = g.add_edge(&s(a), &s(b)).unwrap().unwrap();
    g.set_edge_weight(&e, w).unwrap();
}

mod page_rank_unweighted {
    use super::*;

    #[test]
    fn test_unweighted_graph_1() {
        let mut g = DiGraph::<String>::pseudo().weighted();
        g.add_vertices(["center", "a", "b", "c"].map(s));
        for v in ["a", "b", "c"] {
            add_weighted(&mut g, "center", v, 1.0);
        }
        let pr = PageRank::with_options(&g, 0.85, 100, 0.0001).unwrap();
        near(pr.vertex_score(&s("center")).unwrap(), 0.2061, 0.0001);
        for v in ["a", "b", "c"] {
            near(pr.vertex_score(&s(v)).unwrap(), 0.2646, 0.0001);
        }
    }

    #[test]
    fn test_unweighted_graph_2() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertices(["center", "a", "b", "c", "d"].map(s));
        for v in ["a", "b", "c"] {
            g.add_edge(&s("center"), &s(v)).unwrap();
        }
        let pr = PageRank::with_options(&g, 0.85, 100, 0.0001).unwrap();
        near(pr.vertex_score(&s("center")).unwrap(), 0.1709, 0.0001);
        for v in ["a", "b", "c"] {
            near(pr.vertex_score(&s(v)).unwrap(), 0.21937, 0.0001);
        }
        near(pr.vertex_score(&s("d")).unwrap(), 0.1709, 0.0001);
    }
}

mod katz_exogenous_map {
    use super::*;

    #[test]
    fn test_graph_3() {
        let mut g = DiGraph::<String>::pseudo();
        g.add_vertices(["A", "B", "C", "D", "E", "F", "1", "2", "3", "4", "5"].map(s));
        for (a, b) in [
            ("A", "E"),
            ("A", "F"),
            ("B", "E"),
            ("B", "F"),
            ("C", "E"),
            ("D", "E"),
            ("1", "E"),
            ("1", "F"),
            ("E", "1"),
            ("2", "E"),
            ("2", "F"),
            ("3", "F"),
            ("F", "3"),
            ("4", "F"),
            ("E", "4"),
            ("4", "5"),
            ("E", "F"),
        ] {
            g.add_edge(&s(a), &s(b)).unwrap();
        }
        let pr = KatzCentrality::with_options(
            &g,
            0.15,
            |v| if v == "4" { 2.0 } else { 1.0 },
            100,
            0.0001,
        )
        .unwrap();
        for (v, x) in [
            ("A", 1.0),
            ("B", 1.0),
            ("C", 1.0),
            ("D", 1.0),
            ("E", 1.94),
            ("F", 2.48),
            ("1", 1.29),
            ("2", 1.0),
            ("3", 1.37),
            ("4", 2.29),
            ("5", 1.34),
        ] {
            near(pr.vertex_score(&s(v)).unwrap(), x, 0.005);
        }
    }
}

mod betweenness_centrality {
    use super::*;
    use std::collections::HashMap;

    type Scores = HashMap<i32, f64>;

    fn ugraph(n: std::ops::RangeInclusive<i32>, edges: &[(i32, i32)]) -> UGraph<i32> {
        let mut g = UGraph::<i32>::simple();
        g.add_vertices(n);
        for (a, b) in edges {
            g.add_edge(a, b).unwrap();
        }
        g
    }

    fn bc(g: &UGraph<i32>) -> Scores {
        BetweennessCentrality::new(g).unwrap().scores().clone()
    }

    fn graph1() -> UGraph<i32> {
        ugraph(
            1..=9,
            &[
                (1, 2),
                (1, 3),
                (1, 4),
                (2, 3),
                (3, 4),
                (4, 5),
                (4, 6),
                (5, 6),
                (5, 7),
                (5, 8),
                (6, 7),
                (6, 8),
                (7, 8),
                (7, 9),
            ],
        )
    }

    fn assert_scores(scores: &Scores, expected: &[(i32, f64)]) {
        for &(v, x) in expected {
            assert_eq!(scores[&v], x, "vertex {v}");
        }
    }

    fn assert_graph1(s: &Scores) {
        assert_scores(
            s,
            &[
                (1, 3.0),
                (2, 0.0),
                (3, 3.0),
                (4, 15.0),
                (5, 6.0),
                (6, 6.0),
                (7, 7.0),
                (8, 0.0),
                (9, 0.0),
            ],
        );
    }

    #[test]
    fn test_empty_graph() {
        let g = UGraph::<i32>::simple();
        assert!(BetweennessCentrality::new(&g).unwrap().scores().is_empty());
        assert!(
            BetweennessCentrality::normalized(&g)
                .unwrap()
                .scores()
                .is_empty()
        );
    }

    #[test]
    fn test_singleton_and_k2() {
        for normalized in [false, true] {
            for n in [0, 1] {
                let g = ugraph(0..=n, if n == 1 { &[(0, 1)] } else { &[] });
                let s = if normalized {
                    BetweennessCentrality::normalized(&g)
                } else {
                    BetweennessCentrality::new(&g)
                }
                .unwrap()
                .scores()
                .clone();
                assert!(s.values().all(|&x| x == 0.0));
            }
        }
    }

    #[test]
    fn test_unweighted_1() {
        assert_graph1(&bc(&graph1()));
    }

    #[test]
    fn test_as_weighted_1() {
        let base = graph1();
        let g = AsWeighted::new(&base, HashMap::new());
        assert_graph1(BetweennessCentrality::new(&g).unwrap().scores());
    }

    #[test]
    fn test_normalization() {
        let base = graph1();
        let g = AsWeighted::new(&base, HashMap::new());
        let scores = BetweennessCentrality::normalized(&g)
            .unwrap()
            .scores()
            .clone();
        let n = g.vertex_count() as f64;
        let scaled: Scores = scores
            .into_iter()
            .map(|(v, x)| (v, x * ((n - 1.0) * (n - 2.0))))
            .collect();
        for (v, want) in [
            (1, 3.0),
            (2, 0.0),
            (3, 3.0),
            (4, 15.0),
            (5, 6.0),
            (6, 6.0),
            (7, 7.0),
            (8, 0.0),
            (9, 0.0),
        ] {
            assert!((scaled[&v] - want).abs() < 1e-9);
        }
    }

    #[test]
    fn test_unweighted_2() {
        let g = ugraph(
            0..=14,
            &[
                (0, 1),
                (0, 2),
                (0, 5),
                (1, 6),
                (1, 9),
                (2, 3),
                (2, 4),
                (2, 10),
                (2, 14),
                (3, 7),
                (4, 11),
                (7, 8),
                (7, 12),
                (7, 13),
            ],
        );
        assert_scores(
            &bc(&g),
            &[
                (0, 43.0),
                (1, 25.0),
                (2, 70.0),
                (3, 40.0),
                (4, 13.0),
                (5, 0.0),
                (6, 0.0),
                (7, 36.0),
                (8, 0.0),
                (9, 0.0),
                (10, 0.0),
                (11, 0.0),
                (12, 0.0),
                (13, 0.0),
                (14, 0.0),
            ],
        );
    }

    #[test]
    fn test_unweighted_3() {
        let g = ugraph(
            1..=6,
            &[(1, 2), (1, 5), (2, 3), (2, 5), (3, 4), (4, 5), (4, 6)],
        );
        assert_scores(
            &bc(&g),
            &[(1, 0.0), (2, 1.5), (3, 1.0), (4, 4.5), (5, 3.0), (6, 0.0)],
        );
    }

    #[test]
    fn test_unweighted_4() {
        let g = ugraph(1..=5, &[(1, 2), (2, 3), (2, 4), (3, 5), (4, 5)]);
        assert_scores(&bc(&g), &[(1, 0.0), (2, 3.5), (3, 1.0), (4, 1.0), (5, 0.5)]);
    }

    #[test]
    fn test_weighted_5() {
        let mut g = DiGraph::<String>::simple().weighted();
        g.add_vertices("ABCDEFG".chars().map(|c| c.to_string()));
        for (a, b, w) in [
            ("A", "B", 0.7),
            ("A", "D", 0.3),
            ("B", "C", 0.9),
            ("C", "A", 1.3),
            ("C", "D", 0.57),
            ("D", "B", 1.0),
            ("D", "E", 0.8),
            ("D", "F", 0.2),
            ("E", "G", 0.4),
            ("F", "E", 0.6),
            ("G", "F", 0.2),
        ] {
            add_weighted(&mut g, a, b, w);
        }
        let s = BetweennessCentrality::new(&g).unwrap();
        for (v, want) in [
            ("A", 0.0),
            ("B", 3.0),
            ("C", 6.0),
            ("D", 10.0),
            ("E", 5.0),
            ("F", 5.0),
            ("G", 1.0),
        ] {
            assert_eq!(s.vertex_score(&s_(v)).unwrap(), want, "{v}");
        }
    }

    fn s_(x: &str) -> String {
        x.to_string()
    }

    #[test]
    fn test_weighted_6() {
        let mut g = DiGraph::<i32>::simple().weighted();
        g.add_vertices(0..=2);
        for (a, b, w) in [(2, 1, 1.0), (1, 0, 1.0), (2, 0, 49.0)] {
            let e = g.add_edge(&a, &b).unwrap().unwrap();
            g.set_edge_weight(&e, w).unwrap();
        }
        let s = BetweennessCentrality::new(&g).unwrap();
        assert_scores(s.scores(), &[(0, 0.0), (1, 1.0), (2, 0.0)]);
    }

    #[test]
    fn test_star() {
        for order in [5usize, 12] {
            let mut g = UGraph::<usize>::simple().with_vertex_factory(|i| i as usize);
            let mut result = GeneratorResult::new();
            Star(order)
                .generate_graph(&mut g, Some(&mut result))
                .unwrap();
            let center = result["Center Vertex"][0];
            let s = BetweennessCentrality::new(&g).unwrap();
            for (v, &x) in s.scores() {
                let want = if *v == center {
                    ((order - 2) * (order - 1) / 2) as f64
                } else {
                    0.0
                };
                assert_eq!(x, want);
            }
        }
    }

    #[test]
    fn test_linear() {
        for order in [5usize, 12, 37] {
            let mut g = UGraph::<usize>::simple().with_vertex_factory(|i| i as usize);
            Linear(order).generate_default(&mut g).unwrap();
            let s = BetweennessCentrality::new(&g).unwrap();
            let sc = s.scores();
            if order == 5 {
                for (v, &x) in sc {
                    let want = match v {
                        0 | 4 => 0.0,
                        1 | 3 => 3.0,
                        _ => 4.0,
                    };
                    assert_eq!(x, want);
                }
            } else {
                for i in 0..order / 2 {
                    assert_eq!(sc[&i], sc[&(order - i - 1)]);
                }
            }
        }
    }

    #[test]
    fn test_ring() {
        for order in [5usize, 12, 37] {
            let mut g = UGraph::<usize>::simple().with_vertex_factory(|i| i as usize);
            Ring(order).generate_default(&mut g).unwrap();
            let s = BetweennessCentrality::new(&g).unwrap();
            let sc = s.scores();
            if order == 5 {
                assert!(sc.values().all(|&x| x == 1.0));
            } else {
                for i in 0..order - 1 {
                    assert_eq!(sc[&i], sc[&(i + 1)]);
                }
            }
        }
    }

    fn overflow_graph() -> DiGraph<i32> {
        let mut g = DiGraph::<i32>::simple();
        g.add_vertices(0..3300);
        for i in 0..3290 {
            for j in 0..10 {
                g.add_edge(&i, &(i - i % 10 + 10 + j)).unwrap();
            }
        }
        g
    }

    #[test]
    fn test_overflow() {
        let g = overflow_graph();
        assert!(
            BetweennessCentrality::with_options(
                &g,
                false,
                OverflowStrategy::ThrowExceptionOnOverflow
            )
            .is_err()
        );
    }

    #[test]
    fn test_ignore_overflow() {
        let g = overflow_graph();
        let b = BetweennessCentrality::with_options(&g, false, OverflowStrategy::IgnoreOverflow)
            .unwrap();
        let s = b.scores();
        assert!(s[&9].abs() < 1e-9);
        assert!(s[&10].is_nan());
        assert!(s[&3289].is_nan());
        assert!(s[&3290].abs() < 1e-9);
    }
}

mod ap_betweenness_centrality {
    use super::*;
    use num_bigint::BigInt;
    use num_rational::BigRational;

    // exact rational arithmetic on ~330-digit path counts takes about a minute
    #[test]
    #[ignore = "slow"]
    fn test_no_overflow() {
        let mut g = DiGraph::<i32>::simple();
        g.add_vertices(0..3300);
        for i in 0..3290 {
            for j in 0..10 {
                g.add_edge(&i, &(i - i % 10 + 10 + j)).unwrap();
            }
        }
        let bc = ApBetweennessCentrality::new(&g).unwrap();
        let int = |x: i64| BigRational::from_integer(BigInt::from(x));
        assert_eq!(bc.vertex_score(&9).unwrap(), &int(0));
        assert_eq!(bc.vertex_score(&10).unwrap(), &int(3280));
        assert_eq!(bc.vertex_score(&3289).unwrap(), &int(3280));
        assert_eq!(bc.vertex_score(&3290).unwrap(), &int(0));
    }
}

mod edge_betweenness_centrality {
    use super::*;

    const FIRST_EDGES: [(&str, &str); 12] = [
        ("A", "B"),
        ("A", "C"),
        ("A", "D"),
        ("B", "C"),
        ("B", "D"),
        ("C", "D"),
        ("D", "E"),
        ("E", "F"),
        ("E", "G"),
        ("F", "G"),
        ("F", "H"),
        ("G", "H"),
    ];

    const TREE_EDGES: [(i32, i32); 17] = [
        (1, 2),
        (1, 3),
        (2, 3),
        (3, 7),
        (4, 6),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 8),
        (8, 9),
        (8, 12),
        (9, 10),
        (9, 11),
        (12, 13),
        (12, 14),
        (10, 11),
        (13, 14),
    ];

    fn int_graph<D: graph::Direction>() -> BaseGraph<i32, DefaultEdge, D> {
        let mut g = BaseGraph::<i32, DefaultEdge, D>::with_loops();
        g.add_vertices(1..15);
        g
    }

    fn check<G: Graph<Vertex = i32, Edge = DefaultEdge>>(
        g: &G,
        ebc: &mut EdgeBetweennessCentrality<'_, G>,
        expected: [(i32, i32, f64); 4],
    ) {
        for (a, b, want) in expected {
            let e = *g.get_edge(&a, &b).unwrap();
            near(ebc.edge_score(&e).unwrap(), want, 1e-9);
        }
    }

    #[test]
    fn test_undirected_graph_1() {
        let mut g = UGraph::<String>::with_loops();
        g.add_vertices("ABCDEFGH".chars().map(|c| c.to_string()));
        for (a, b) in FIRST_EDGES {
            g.add_edge(&s(a), &s(b)).unwrap();
        }
        let mut ebc = EdgeBetweennessCentrality::new(&g).unwrap();
        let de = *g.get_edge(&s("D"), &s("E")).unwrap();
        let eg = *g.get_edge(&s("E"), &s("G")).unwrap();
        near(ebc.edge_score(&de).unwrap(), 16.0, 1e-9);
        near(ebc.edge_score(&eg).unwrap(), 7.5, 1e-9);
    }

    #[test]
    fn test_undirected_graph_2() {
        let mut g = int_graph::<graph::Undirected>();
        for (a, b) in TREE_EDGES {
            g.add_edge(&a, &b).unwrap();
        }
        let mut ebc = EdgeBetweennessCentrality::new(&g).unwrap();
        check(
            &g,
            &mut ebc,
            [(3, 7, 33.0), (7, 8, 49.0), (9, 11, 12.0), (13, 14, 1.0)],
        );
    }

    #[test]
    fn test_directed_graph_3() {
        let mut g = int_graph::<graph::Directed>();
        for (a, b) in TREE_EDGES {
            g.add_edge(&a, &b).unwrap();
        }
        let mut ebc = EdgeBetweennessCentrality::new(&g).unwrap();
        check(
            &g,
            &mut ebc,
            [(3, 7, 24.0), (7, 8, 49.0), (9, 11, 9.0), (13, 14, 1.0)],
        );
    }

    #[test]
    fn test_directed_graph_3_subset() {
        let mut g = int_graph::<graph::Directed>();
        for (a, b) in TREE_EDGES {
            g.add_edge(&a, &b).unwrap();
        }
        let mut ebc = EdgeBetweennessCentrality::with_options(
            &g,
            OverflowStrategy::ThrowExceptionOnOverflow,
            Some(vec![1, 2, 4, 11]),
        )
        .unwrap();
        check(
            &g,
            &mut ebc,
            [(3, 7, 16.0), (7, 8, 21.0), (9, 11, 3.0), (13, 14, 0.0)],
        );
    }

    #[test]
    fn test_undirected_graph_with_weights() {
        let mut g = UGraph::<String>::with_loops().weighted();
        g.add_vertices("ABCDEFGH".chars().map(|c| c.to_string()));
        let edges = [
            ("A", "B"),
            ("A", "C"),
            ("A", "D"),
            ("B", "C"),
            ("B", "D"),
            ("C", "D"),
            ("D", "E"),
            ("D", "F"),
            ("E", "F"),
            ("E", "G"),
            ("F", "G"),
            ("F", "H"),
            ("G", "H"),
        ];
        for (a, b) in edges {
            let e = g.add_edge(&s(a), &s(b)).unwrap().unwrap();
            if (a, b) == ("D", "E") {
                g.set_edge_weight(&e, 1000.0).unwrap();
            }
        }
        let mut ebc = EdgeBetweennessCentrality::new(&g).unwrap();
        for (a, b, want) in [
            ("D", "E", 0.0),
            ("D", "F", 16.0),
            ("E", "G", 1.5),
            ("F", "G", 5.0),
        ] {
            let e = *g.get_edge(&s(a), &s(b)).unwrap();
            near(ebc.edge_score(&e).unwrap(), want, 1e-9);
        }
    }
}
