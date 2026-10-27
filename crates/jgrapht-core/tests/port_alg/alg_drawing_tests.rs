//! Ports of the `org.jgrapht.alg.drawing` tests (model and layouts).

use jgrapht::alg::drawing::model::*;
use jgrapht::*;
use std::cell::RefCell;
use std::rc::Rc;

const EPS: f64 = 1e-9;

fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= EPS, "{a} vs {b}");
}

mod box2d {
    use super::*;

    fn check(b: Box2D, x: f64, y: f64, w: f64, h: f64) {
        near(b.width(), w);
        near(b.height(), h);
        near(b.min_x(), x);
        near(b.min_y(), y);
        near(b.max_x(), x + w);
        near(b.max_y(), y + h);
    }

    #[test]
    fn test_constructors_and_factories() {
        check(Box2D::of(5.0, 10.0), 0.0, 0.0, 5.0, 10.0);
        check(Box2D::at(5.0, 4.0, 7.0, 8.0), 5.0, 4.0, 7.0, 8.0);
    }
}

mod boxes_test {
    use super::*;

    #[test]
    fn test_contains_point_2d() {
        let region = Box2D::at(0.0, 0.0, 10.0, 10.0);
        for (x, y) in [
            (5., 5.),
            (10., 5.),
            (10., 10.),
            (0., 0.),
            (0., 10.),
            (10., 0.),
        ] {
            assert!(boxes::contains_point(&region, Point2D::of(x, y)));
        }
        assert!(!boxes::contains_point(&region, Point2D::of(11.0, 0.0)));
        assert!(!boxes::contains_point(&region, Point2D::of(0.0, 11.0)));
    }

    #[test]
    fn test_split_along_x_axis() {
        let (west, east) = boxes::split_along_x_axis(&Box2D::at(5.0, 5.0, 10.0, 10.0));
        assert_eq!(west, Box2D::at(5.0, 5.0, 5.0, 10.0));
        assert_eq!(east, Box2D::at(10.0, 5.0, 5.0, 10.0));
    }

    #[test]
    fn test_split_along_y_axis() {
        let (south, north) = boxes::split_along_y_axis(&Box2D::at(5.0, 5.0, 10.0, 10.0));
        assert_eq!(south, Box2D::at(5.0, 5.0, 10.0, 5.0));
        assert_eq!(north, Box2D::at(5.0, 10.0, 10.0, 5.0));
    }
}

mod point2d {
    use super::*;

    #[test]
    fn test_constructors_and_getters() {
        let p = Point2D::of(0.0, 0.0);
        assert_eq!((p.x, p.y), (0.0, 0.0));
        let p = Point2D::of(3.0, 2.0);
        assert_eq!((p.x, p.y), (3.0, 2.0));
    }

    #[test]
    fn test_equality_follows_bit_patterns() {
        assert_eq!(Point2D::of(f64::NAN, 1.0), Point2D::of(f64::NAN, 1.0));
        assert_ne!(Point2D::of(0.0, 1.0), Point2D::of(-0.0, 1.0));
    }
}

mod points_test {
    use super::*;

    #[test]
    fn test_arithmetic() {
        near(points::length(Point2D::of(5.0, 5.0)), 50f64.sqrt());
        let s = points::add(Point2D::of(5.0, 5.0), Point2D::of(3.0, 4.0));
        assert_eq!((s.x, s.y), (8.0, 9.0));
        let d = points::subtract(Point2D::of(5.0, 5.0), Point2D::of(3.0, 4.0));
        assert_eq!((d.x, d.y), (2.0, 1.0));
        let n = points::negate(Point2D::of(5.0, 3.0));
        assert_eq!((n.x, n.y), (-5.0, -3.0));
        let m = points::scalar_multiply(Point2D::of(5.0, 3.0), 2.0);
        assert_eq!((m.x, m.y), (10.0, 6.0));
        assert!(points::equals(
            Point2D::of(1.0, 1.0),
            Point2D::of(1.0 + 1e-12, 1.0)
        ));
    }
}

mod map_layout_model {
    use super::*;

    #[test]
    fn test_general() {
        let (v1, v2) = ("1".to_string(), "2".to_string());
        let mut model = MapLayoutModel2D::new(Box2D::at(0.0, 0.0, 2.0, 2.0));
        assert_eq!(model.drawable_area(), Box2D::at(0.0, 0.0, 2.0, 2.0));
        assert_eq!(model.get(&v1), None);
        model.put(v1.clone(), Point2D::of(3.0, 5.0));
        assert_eq!(model.get(&v1), Some(Point2D::of(3.0, 5.0)));
        assert!(!model.is_fixed(&v1));
        model.set_fixed(v1.clone(), true);
        assert!(model.is_fixed(&v1));
        model.put(v1.clone(), Point2D::of(10.0, 20.0));
        assert_eq!(model.get(&v1), Some(Point2D::of(3.0, 5.0)));
        model.set_fixed(v1.clone(), false);
        assert!(!model.is_fixed(&v1));
        model.put(v1.clone(), Point2D::of(10.0, 20.0));
        assert_eq!(model.get(&v1), Some(Point2D::of(10.0, 20.0)));
        model.put(v2.clone(), Point2D::of(5.0, 7.0));
        let all = model.collect();
        assert_eq!(all[&v1], Point2D::of(10.0, 20.0));
        assert_eq!(all[&v2], Point2D::of(5.0, 7.0));
    }
}

mod listenable_layout_model {
    use super::*;

    #[test]
    fn test_general() {
        let (v1, v2) = ("1".to_string(), "2".to_string());
        let mut model =
            ListenableLayoutModel2D::new(MapLayoutModel2D::new(Box2D::at(0.0, 0.0, 2.0, 2.0)));
        let seen: Rc<RefCell<Vec<(String, Point2D)>>> = Rc::default();
        let sink = seen.clone();
        model.add_listener(move |v: &String, p| sink.borrow_mut().push((v.clone(), p)));
        assert_eq!(model.drawable_area(), Box2D::at(0.0, 0.0, 2.0, 2.0));
        assert_eq!(model.get(&v1), None);
        model.put(v1.clone(), Point2D::of(3.0, 5.0));
        assert_eq!(model.get(&v1), Some(Point2D::of(3.0, 5.0)));
        assert!(!model.is_fixed(&v1));
        assert_eq!(seen.borrow().len(), 1);
        assert_eq!(seen.borrow()[0], (v1.clone(), Point2D::of(3.0, 5.0)));

        model.set_fixed(v1.clone(), true);
        assert!(model.is_fixed(&v1));
        model.put(v1.clone(), Point2D::of(10.0, 20.0));
        assert_eq!(model.get(&v1), Some(Point2D::of(3.0, 5.0)));
        assert_eq!(seen.borrow().len(), 1);

        model.set_fixed(v1.clone(), false);
        model.put(v1.clone(), Point2D::of(10.0, 20.0));
        assert_eq!(model.get(&v1), Some(Point2D::of(10.0, 20.0)));
        assert_eq!(seen.borrow().len(), 2);
        assert_eq!(seen.borrow()[1], (v1.clone(), Point2D::of(10.0, 20.0)));

        model.put(v2.clone(), Point2D::of(5.0, 7.0));
        assert_eq!(seen.borrow().len(), 3);
        assert_eq!(seen.borrow()[2], (v2.clone(), Point2D::of(5.0, 7.0)));
        let all = model.collect();
        assert_eq!(all[&v1], Point2D::of(10.0, 20.0));
        assert_eq!(all[&v2], Point2D::of(5.0, 7.0));
    }
}

mod layouts {
    use super::*;
    use jgrapht::alg::drawing::layouts::*;
    use jgrapht::util::java::JavaRandom;
    use std::collections::HashSet;

    fn vertices(n: usize) -> UGraph<String> {
        let mut g = UGraph::<String>::simple();
        g.add_vertices((1..=n).map(|i| i.to_string()));
        g
    }

    fn v(i: usize) -> String {
        i.to_string()
    }

    fn at(model: &MapLayoutModel2D<String>, i: usize, x: f64, y: f64) {
        assert!(
            points::equals(Point2D::of(x, y), model.get(&v(i)).unwrap()),
            "vertex {i}"
        );
    }

    #[test]
    fn circular_simple() {
        let g = vertices(4);
        let mut model = MapLayoutModel2D::new(Box2D::of(2.0, 2.0));
        CircularLayoutAlgorithm2D::new(1.0)
            .unwrap()
            .layout(&g, &mut model)
            .unwrap();
        at(&model, 1, 2.0, 1.0);
        at(&model, 2, 1.0, 2.0);
        at(&model, 3, 0.0, 1.0);
        at(&model, 4, 1.0, 0.0);
    }

    #[test]
    fn circular_with_order() {
        let mut g = UGraph::<String>::simple();
        g.add_vertices(["4", "3", "2", "1"].map(String::from));
        let mut model = MapLayoutModel2D::new(Box2D::of(2.0, 2.0));
        CircularLayoutAlgorithm2D::new(1.0)
            .unwrap()
            .with_vertex_comparator(|a: &String, b: &String| a.cmp(b))
            .layout(&g, &mut model)
            .unwrap();
        at(&model, 1, 2.0, 1.0);
        at(&model, 2, 1.0, 2.0);
        at(&model, 3, 0.0, 1.0);
        at(&model, 4, 1.0, 0.0);
    }

    #[test]
    fn circular_rejects_bad_radius_and_small_area() {
        assert!(CircularLayoutAlgorithm2D::<String>::new(0.0).is_err());
        let g = vertices(2);
        let mut model = MapLayoutModel2D::new(Box2D::of(1.0, 1.0));
        assert!(
            CircularLayoutAlgorithm2D::new(1.0)
                .unwrap()
                .layout(&g, &mut model)
                .is_err()
        );
    }

    #[test]
    fn random() {
        let g = vertices(4);
        let mut model = MapLayoutModel2D::new(Box2D::of(10.0, 20.0));
        RandomLayoutAlgorithm2D::with_seed(5)
            .layout(&g, &mut model)
            .unwrap();
        let mut rng = JavaRandom::new(5);
        for i in 1..=4 {
            let (x, y) = (10.0 * rng.next_double(), 20.0 * rng.next_double());
            assert_eq!(model.get(&v(i)), Some(Point2D::of(x, y)));
        }
    }

    #[test]
    fn rescale_simple() {
        let g = vertices(4);
        let mut model = MapLayoutModel2D::new(Box2D::of(2.0, 2.0));
        CircularLayoutAlgorithm2D::new(1.0)
            .unwrap()
            .layout(&g, &mut model)
            .unwrap();
        at(&model, 1, 2.0, 1.0);
        RescaleLayoutAlgorithm2D::new(4.0)
            .unwrap()
            .layout(&g, &mut model)
            .unwrap();
        at(&model, 1, 5.0, 1.0);
        at(&model, 2, 1.0, 5.0);
        at(&model, 3, -3.0, 1.0);
        at(&model, 4, 1.0, -3.0);
    }

    #[test]
    fn rescale_with_different_aspect_ratio() {
        let g = vertices(4);
        let mut model = MapLayoutModel2D::new(Box2D::at(-1.0, -1.0, 2.0, 2.0));
        for (i, x, y) in [(1, 1.0, 0.0), (2, -1.0, 0.0), (3, 0.0, 0.5), (4, 0.0, -0.5)] {
            model.put(v(i), Point2D::of(x, y));
        }
        RescaleLayoutAlgorithm2D::new(3.0)
            .unwrap()
            .layout(&g, &mut model)
            .unwrap();
        at(&model, 1, 3.0, 0.0);
        at(&model, 2, -3.0, 0.0);
        at(&model, 3, 0.0, 1.5);
        at(&model, 4, 0.0, -1.5);
    }

    #[test]
    fn rescale_bad_input() {
        let g = vertices(1);
        let mut model = MapLayoutModel2D::new(Box2D::at(-1.0, -1.0, 2.0, 2.0));
        model.put(v(1), Point2D::of(0.0, 0.0));
        RescaleLayoutAlgorithm2D::new(3.0)
            .unwrap()
            .layout(&g, &mut model)
            .unwrap();
        at(&model, 1, 0.0, 0.0);
        let empty = vertices(0);
        let mut model = MapLayoutModel2D::new(Box2D::at(-1.0, -1.0, 2.0, 2.0));
        RescaleLayoutAlgorithm2D::new(3.0)
            .unwrap()
            .layout(&empty, &mut model)
            .unwrap();
        assert!(RescaleLayoutAlgorithm2D::new(0.0).is_err());
    }

    fn bipartite() -> UGraph<String> {
        let mut g = vertices(6);
        for (a, b) in [(1, 4), (1, 5), (1, 6), (2, 4), (2, 5), (3, 5), (3, 6)] {
            g.add_edge(&v(a), &v(b)).unwrap();
        }
        g
    }

    fn pt(model: &MapLayoutModel2D<String>, i: usize) -> Point2D {
        model.get(&v(i)).unwrap()
    }

    #[test]
    fn two_layered_vertical() {
        let g = bipartite();
        let mut model = MapLayoutModel2D::new(Box2D::of(3.0, 10.0));
        TwoLayeredBipartiteLayout2D::new()
            .layout(&g, &mut model)
            .unwrap();
        for (i, x, y) in [
            (1, 0., 0.),
            (2, 0., 5.),
            (3, 0., 10.),
            (4, 3., 0.),
            (5, 3., 5.),
            (6, 3., 10.),
        ] {
            assert_eq!(pt(&model, i), Point2D::of(x, y));
        }
    }

    #[test]
    fn two_layered_horizontal() {
        let g = bipartite();
        let mut model = MapLayoutModel2D::new(Box2D::of(10.0, 3.0));
        TwoLayeredBipartiteLayout2D::new()
            .with_vertical(false)
            .layout(&g, &mut model)
            .unwrap();
        for (i, x, y) in [
            (1, 0., 0.),
            (2, 5., 0.),
            (3, 10., 0.),
            (4, 0., 3.),
            (5, 5., 3.),
            (6, 10., 3.),
        ] {
            assert_eq!(pt(&model, i), Point2D::of(x, y));
        }
    }

    #[test]
    fn two_layered_with_partition() {
        let mut g = vertices(7);
        for (a, b) in [(1, 4), (1, 5), (1, 6), (2, 4), (2, 5), (3, 5), (3, 6)] {
            g.add_edge(&v(a), &v(b)).unwrap();
        }
        let mut model = MapLayoutModel2D::new(Box2D::of(3.0, 10.0));
        TwoLayeredBipartiteLayout2D::new()
            .with_first_partition(HashSet::from([v(1), v(2), v(3), v(7)]))
            .layout(&g, &mut model)
            .unwrap();
        for (i, x, y) in [
            (1, 0., 0.),
            (2, 0., 3.3333333333333335),
            (3, 0., 6.666666666666667),
            (7, 0., 10.),
            (4, 3., 0.),
            (5, 3., 5.),
            (6, 3., 10.),
        ] {
            assert_eq!(pt(&model, i), Point2D::of(x, y));
        }
    }

    fn greedy_graph() -> UGraph<String> {
        let mut g = vertices(9);
        for (a, b) in [
            (1, 4),
            (1, 5),
            (1, 6),
            (1, 7),
            (1, 8),
            (2, 4),
            (2, 5),
            (2, 7),
            (3, 5),
            (3, 6),
            (3, 8),
        ] {
            g.add_edge(&v(a), &v(b)).unwrap();
        }
        g
    }

    fn check_greedy(mut alg: TwoLayeredBipartiteLayout2D<String>) {
        let g = greedy_graph();
        alg = alg.with_first_partition(HashSet::from([v(1), v(2), v(3)]));
        let mut model = MapLayoutModel2D::new(Box2D::of(3.0, 10.0));
        alg.layout(&g, &mut model).unwrap();
        for (i, x, y) in [
            (1, 0., 0.),
            (2, 0., 5.),
            (3, 0., 10.),
            (9, 3., 0.),
            (5, 3., 2.),
            (4, 3., 4.),
            (6, 3., 6.),
            (7, 3., 8.),
            (8, 3., 10.),
        ] {
            assert_eq!(pt(&model, i), Point2D::of(x, y), "vertex {i}");
        }
    }

    #[test]
    fn median_greedy() {
        check_greedy(TwoLayeredBipartiteLayout2D::median_greedy());
    }

    #[test]
    fn barycenter_greedy() {
        check_greedy(TwoLayeredBipartiteLayout2D::barycenter_greedy());
    }
}

mod fr {
    use super::*;
    use jgrapht::alg::drawing::fr::*;
    use jgrapht::alg::drawing::layouts::LayoutAlgorithm2D;
    use jgrapht::util::java::JavaRandom;

    fn v(i: usize) -> String {
        i.to_string()
    }

    fn star_pair(extra: usize) -> UGraph<String> {
        let mut g = UGraph::<String>::simple();
        g.add_vertices((1..=6 + extra).map(v));
        for (a, b) in [(1, 2), (3, 1), (4, 1), (5, 2), (6, 2)] {
            g.add_edge(&v(a), &v(b)).unwrap();
        }
        g
    }

    /// The layout is oriented as in the picture of the Java test.
    fn assert_orientation(result: &indexmap::IndexMap<String, Point2D>) {
        let p = |i: usize| result[&v(i)];
        assert!(p(1).x > p(2).x && p(1).y > p(2).y);
        assert!(p(3).x > p(1).x && p(3).y < p(1).y);
        assert!(p(4).x > p(1).x && p(4).y > p(1).y);
        assert!(p(5).x < p(2).x && p(5).y < p(2).y);
        assert!(p(6).x < p(2).x && p(6).y > p(2).y);
    }

    #[test]
    fn test_graph_1() {
        let g = star_pair(0);
        let mut alg = FRLayoutAlgorithm2D::new(100, 0.5, JavaRandom::new(17));
        let mut model = MapLayoutModel2D::new(Box2D::of(100.0, 100.0));
        alg.layout(&g, &mut model).unwrap();
        assert_orientation(&model.collect());
    }

    #[test]
    fn test_initializer_same_position() {
        let g = star_pair(0);
        let mut alg = FRLayoutAlgorithm2D::new(100, 0.5, JavaRandom::new(17));
        let rng = RefCell::new(JavaRandom::new(17));
        alg.initializer = Some(Box::new(move |u: &String| {
            if u == "1" || u == "2" {
                // two points with the same position
                return Some(Point2D::of(1.0, 1.0));
            }
            let mut r = rng.borrow_mut();
            Some(Point2D::of(r.next_double(), r.next_double()))
        }));
        let mut model = MapLayoutModel2D::new(Box2D::of(100.0, 100.0));
        alg.layout(&g, &mut model).unwrap();
        model.collect();
    }

    #[test]
    fn test_graph_with_isolated_vertex() {
        let g = star_pair(1);
        let mut alg = FRLayoutAlgorithm2D::new(100, 0.5, JavaRandom::new(17));
        let mut model = MapLayoutModel2D::new(Box2D::of(100.0, 100.0));
        alg.layout(&g, &mut model).unwrap();
        let p = model.collect()[&v(7)];
        near(p.x, 100.0);
        near(p.y, 100.0);
    }

    #[test]
    fn indexed_test_graph_1() {
        let g = star_pair(0);
        let mut alg = FRLayoutAlgorithm2D::indexed(100, 0.5, 0.5, JavaRandom::new(17)).unwrap();
        let mut model = MapLayoutModel2D::new(Box2D::of(100.0, 100.0));
        alg.layout(&g, &mut model).unwrap();
        assert_orientation(&model.collect());
        assert_eq!(alg.saved_comparisons(), 80);
    }

    #[test]
    fn indexed_test_graph_zero_theta() {
        let g = star_pair(0);
        let mut alg = FRLayoutAlgorithm2D::indexed(100, 0.0, 0.5, JavaRandom::new(17)).unwrap();
        let mut model = MapLayoutModel2D::new(Box2D::of(100.0, 100.0));
        alg.layout(&g, &mut model).unwrap();
        assert_orientation(&model.collect());
        assert_eq!(alg.saved_comparisons(), 0);
    }

    #[test]
    fn indexed_test_graph_with_isolated_vertex() {
        let g = star_pair(1);
        let mut alg = FRLayoutAlgorithm2D::indexed(100, 0.0, 0.5, JavaRandom::new(17)).unwrap();
        let mut model = MapLayoutModel2D::new(Box2D::of(100.0, 100.0));
        alg.layout(&g, &mut model).unwrap();
        let p = model.collect()[&v(7)];
        near(p.x, 100.0);
        near(p.y, 100.0);
    }

    #[test]
    fn indexed_rejects_bad_theta() {
        assert!(
            FRLayoutAlgorithm2D::<String, _>::indexed(10, 1.5, 0.5, JavaRandom::new(1)).is_err()
        );
    }

    #[test]
    fn test_quad_tree() {
        let (width, height, count) = (100.0, 100.0, 10000);
        let mut tree = FRQuadTree::new(Box2D::of(width, height));
        let mut rng = JavaRandom::new(17);
        for _ in 0..count {
            tree.insert(Point2D::of(
                rng.next_double() * width,
                rng.next_double() * height,
            ))
            .unwrap();
        }
        let root = tree.root();
        assert_eq!(tree.number_of_points(root), count);
        let mut queue = std::collections::VecDeque::from([root]);
        while let Some(cur) = queue.pop_front() {
            if tree.has_points(cur) {
                let pts = tree.points(cur);
                let (x, y) = pts.iter().fold((0.0, 0.0), |(x, y), p| (x + p.x, y + p.y));
                let n = pts.len() as f64;
                assert!(points::equals(
                    tree.centroid(cur).unwrap(),
                    Point2D::of(x / n, y / n)
                ));
            }
            if !tree.is_leaf(cur) {
                let mut children_points = 0;
                for c in tree.children(cur) {
                    queue.push_back(c);
                    children_points += tree.number_of_points(c);
                }
                assert_eq!(tree.number_of_points(cur), children_points);
            }
        }
    }
}
