//! Ports of the iterator tests in `org.jgrapht.traverse`: `AbstractGraphIteratorTest`,
//! `CrossComponentIteratorTest` and its subclasses (`BreadthFirstIteratorTest`,
//! `DepthFirstIteratorTest`, `ClosestFirstIteratorTest`), `IgnoreDirectionTest`, `EdgeSelectionTest`,
//! `TopologicalOrderIteratorTest`, `LexBreadthFirstIteratorTest`, `MaximumCardinalityIteratorTest`,
//! `DegeneracyOrderingIteratorTest` and `RandomWalkVertexIteratorTest`.

use jgrapht::generate::{GraphGenerator, Linear, Ring};
use jgrapht::traverse::*;
use jgrapht::util::java::JavaRandom;
use jgrapht::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

type Gr = DiGraph<&'static str>;

/// `createDirectedGraph()` of `AbstractGraphIteratorTest`.
fn create_directed_graph() -> Gr {
    let mut g = Gr::with_loops().weighted();
    g.add_vertices(["1", "2", "3", "4", "5", "6", "7", "8", "9", "orphan"]);
    for (a, b, w) in [
        ("1", "2", 1.0),
        ("1", "3", 100.0),
        ("2", "4", 1000.0),
        ("3", "5", 1.0),
        ("3", "6", 100.0),
        ("5", "6", 1.0),
        ("5", "7", 200.0),
        ("6", "1", 1.0),
        ("7", "8", 100.0),
        ("7", "9", 1.0),
        ("8", "2", 1.0),
        ("9", "4", 1.0),
    ] {
        g.add_weighted_edge(&a, &b, w).unwrap();
    }
    g
}

fn collect_result<'g, I: Iterator<Item = &'g &'static str>>(it: I) -> String {
    it.copied().collect::<Vec<_>>().join(",")
}

/// `MyTraversalListener`: checks the component expectations while recording the finish string.
struct Checker {
    result: Rc<RefCell<String>>,
    finish: Rc<RefCell<String>>,
    expected: Vec<(String, usize)>,
    component: usize,
    vertices_in_component: usize,
}

impl Checker {
    fn new(
        result: &Rc<RefCell<String>>,
        finish: &Rc<RefCell<String>>,
        expected: Vec<(String, usize)>,
    ) -> Self {
        Checker {
            result: result.clone(),
            finish: finish.clone(),
            expected,
            component: 0,
            vertices_in_component: 0,
        }
    }
}

impl TraversalListener<&'static str, DefaultEdge> for Checker {
    fn connected_component_started(&mut self) {
        self.component += 1;
    }
    fn connected_component_finished(&mut self) {
        let (want_str, want_count) = self
            .expected
            .get(self.component - 1)
            .unwrap_or_else(|| panic!("Should not get here."));
        assert_eq!(&*self.result.borrow(), want_str);
        assert_eq!(self.vertices_in_component, *want_count);
        self.vertices_in_component = 0;
    }
    fn vertex_traversed(&mut self, _: &&'static str) {
        self.vertices_in_component += 1;
    }
    fn vertex_finished(&mut self, v: &&'static str) {
        self.finish.borrow_mut().push_str(&format!("{v}:"));
    }
}

/// Runs a listener-instrumented traversal and compares the result and finish strings.
macro_rules! run_checked {
    ($it:expr, $expected_components:expr, $expected_result:expr, $expected_finish:expr) => {{
        let result = Rc::new(RefCell::new(String::new()));
        let finish = Rc::new(RefCell::new(String::new()));
        let mut it = $it;
        it.add_traversal_listener(Checker::new(&result, &finish, $expected_components));
        // like `collectResult`: append each vertex, comma separated, as it is produced
        let mut first = true;
        while it.has_next() {
            let v = it.next().unwrap();
            if !first {
                result.borrow_mut().push(',');
            }
            first = false;
            result.borrow_mut().push_str(v);
        }
        assert_eq!(&*result.borrow(), $expected_result);
        assert_eq!(&*finish.borrow(), $expected_finish);
    }};
}

fn comps(items: &[(&str, usize)]) -> Vec<(String, usize)> {
    items.iter().map(|(s, n)| (s.to_string(), *n)).collect()
}

// ---------------------------------------------------------------- Breadth-first

mod breadth_first {
    use super::*;

    fn from_start<'g>(g: &'g Gr, v: &'g &'static str) -> BreadthFirstIterator<'g, Gr> {
        let mut i = BreadthFirstIterator::from_start(g, v).unwrap();
        i.set_cross_component_traversal(true);
        i
    }

    #[test]
    fn test_directed_graph() {
        let g = create_directed_graph();
        let start = g.vertices().find(|v| **v == "1").unwrap();
        run_checked!(
            from_start(&g, start),
            comps(&[("1,2,3,4,5,6,7,8,9", 9), ("1,2,3,4,5,6,7,8,9,orphan", 1)]),
            "1,2,3,4,5,6,7,8,9,orphan",
            ""
        );
    }

    #[test]
    fn test_directed_graph_via_cci() {
        let g = create_directed_graph();
        let starts: Vec<&&str> = ["orphan", "7", "3"]
            .iter()
            .map(|s| g.vertices().find(|v| *v == s).unwrap())
            .collect();
        run_checked!(
            BreadthFirstIterator::from_starts(&g, starts).unwrap(),
            comps(&[
                ("orphan", 1),
                ("orphan,7,8,9,2,4", 5),
                ("orphan,7,8,9,2,4,3,5,6,1", 4)
            ]),
            "orphan,7,8,9,2,4,3,5,6,1",
            ""
        );
    }

    #[test]
    fn test_directed_graph_null_constructors() {
        let g = create_directed_graph();
        for _ in 0..2 {
            run_checked!(
                BreadthFirstIterator::new(&g),
                comps(&[("1,2,3,4,5,6,7,8,9", 9), ("1,2,3,4,5,6,7,8,9,orphan", 1)]),
                "1,2,3,4,5,6,7,8,9,orphan",
                ""
            );
        }
    }

    #[test]
    fn test_non_cross_component_traversal() {
        let g = create_directed_graph();
        let orphan = g.vertices().find(|v| **v == "orphan").unwrap();
        let it = BreadthFirstIterator::from_starts(&g, [orphan]).unwrap();
        assert!(!it.is_cross_component_traversal());
        assert_eq!(collect_result(it), "orphan");
    }

    #[test]
    fn search_tree_test() {
        let mut g = DiGraph::<&str>::with_loops();
        g.add_vertices(["a", "b", "c", "d", "e", "z"]);
        let e1 = g.add_edge(&"a", &"b").unwrap().unwrap();
        let e2 = g.add_edge(&"b", &"c").unwrap().unwrap();
        let e3 = g.add_edge(&"b", &"z").unwrap().unwrap();
        let e4 = g.add_edge(&"b", &"d").unwrap().unwrap();
        let e5 = g.add_edge(&"d", &"e").unwrap().unwrap();
        let a = g.vertices().find(|v| **v == "a").unwrap();
        let mut bfs = BreadthFirstIterator::from_start(&g, a).unwrap();
        while bfs.next().is_some() {}
        assert_eq!(
            ["a", "b", "c", "d", "e", "z"].map(|v| bfs.depth(&v)),
            [Some(0), Some(1), Some(2), Some(2), Some(3), Some(2)]
        );
        assert_eq!(bfs.spanning_tree_edge(&"a"), None);
        for (v, e) in [("b", e1), ("c", e2), ("d", e4), ("e", e5), ("z", e3)] {
            assert_eq!(bfs.spanning_tree_edge(&v), Some(&e));
        }
        assert_eq!(bfs.parent(&"a"), None);
        for (v, p) in [("b", "a"), ("c", "b"), ("d", "b"), ("e", "d"), ("z", "b")] {
            assert_eq!(bfs.parent(&v), Some(&p));
        }
    }

    #[test]
    fn search_tree_directed_cycle_test() {
        let mut g = DiGraph::<i32>::simple();
        let e1 = g.add_edge_with_vertices(0, 1).unwrap().unwrap();
        let e2 = g.add_edge_with_vertices(1, 2).unwrap().unwrap();
        let e3 = g.add_edge_with_vertices(2, 3).unwrap().unwrap();
        g.add_edge_with_vertices(3, 0).unwrap();
        let zero = g.vertices().next().unwrap();
        let mut bfs = BreadthFirstIterator::from_start(&g, zero).unwrap();
        while bfs.next().is_some() {}
        assert_eq!(
            [0, 1, 2, 3].map(|v| bfs.depth(&v)),
            [Some(0), Some(1), Some(2), Some(3)]
        );
        assert_eq!(bfs.spanning_tree_edge(&0), None);
        assert_eq!(
            [1, 2, 3].map(|v| bfs.spanning_tree_edge(&v).copied()),
            [Some(e1), Some(e2), Some(e3)]
        );
        assert_eq!(bfs.parent(&0), None);
        assert_eq!(
            [1, 2, 3].map(|v| bfs.parent(&v).copied()),
            [Some(0), Some(1), Some(2)]
        );
    }
}

// ---------------------------------------------------------------- Depth-first

mod depth_first {
    use super::*;

    fn from_start<'g>(g: &'g Gr, v: &'g &'static str) -> DepthFirstIterator<'g, Gr> {
        let mut i = DepthFirstIterator::from_start(g, v).unwrap();
        i.set_cross_component_traversal(true);
        i
    }

    #[test]
    fn test_directed_graph() {
        let g = create_directed_graph();
        let start = g.vertices().find(|v| **v == "1").unwrap();
        run_checked!(
            from_start(&g, start),
            comps(&[("1,3,6,5,7,9,4,8,2", 9), ("1,3,6,5,7,9,4,8,2,orphan", 1)]),
            "1,3,6,5,7,9,4,8,2,orphan",
            "6:4:9:2:8:7:5:3:1:orphan:"
        );
    }

    #[test]
    fn test_directed_graph_via_cci() {
        let g = create_directed_graph();
        let starts: Vec<&&str> = ["orphan", "7", "3"]
            .iter()
            .map(|s| g.vertices().find(|v| *v == s).unwrap())
            .collect();
        run_checked!(
            DepthFirstIterator::from_starts(&g, starts).unwrap(),
            comps(&[
                ("orphan", 1),
                ("orphan,7,9,4,8,2", 5),
                ("orphan,7,9,4,8,2,3,6,1,5", 4)
            ]),
            "orphan,7,9,4,8,2,3,6,1,5",
            "orphan:4:9:2:8:7:1:6:5:3:"
        );
    }

    #[test]
    fn test_directed_graph_null_constructors() {
        let g = create_directed_graph();
        for _ in 0..2 {
            run_checked!(
                DepthFirstIterator::new(&g),
                comps(&[("1,3,6,5,7,9,4,8,2", 9), ("1,3,6,5,7,9,4,8,2,orphan", 1)]),
                "1,3,6,5,7,9,4,8,2,orphan",
                "6:4:9:2:8:7:5:3:1:orphan:"
            );
        }
    }

    #[test]
    fn test_bug_1169182() {
        let mut dg = DiGraph::<&str>::with_loops();
        dg.add_vertices(["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L"]);
        for (a, b) in [
            ("A", "B"),
            ("B", "C"),
            ("C", "J"),
            ("C", "D"),
            ("C", "E"),
            ("C", "F"),
            ("C", "G"),
            ("D", "H"),
            ("E", "H"),
            ("F", "I"),
            ("G", "I"),
            ("H", "J"),
            ("I", "C"),
            ("J", "K"),
            ("K", "L"),
        ] {
            dg.add_edge(&a, &b).unwrap();
        }
        let actual: String = DepthFirstIterator::new(&dg).copied().collect();
        assert_eq!(actual, "ABCGIFEHJKLD");
    }
}

// ---------------------------------------------------------------- Closest-first

mod closest_first {
    use super::*;

    fn from_start<'g>(g: &'g Gr, v: &'g &'static str) -> ClosestFirstIterator<'g, Gr> {
        let mut i = ClosestFirstIterator::from_start(g, v).unwrap();
        i.set_cross_component_traversal(true);
        i
    }

    #[test]
    fn test_directed_graph() {
        let g = create_directed_graph();
        let start = g.vertices().find(|v| **v == "1").unwrap();
        run_checked!(
            from_start(&g, start),
            comps(&[("1,2,3,5,6,7,9,4,8", 9), ("1,2,3,5,6,7,9,4,8,orphan", 1)]),
            "1,2,3,5,6,7,9,4,8,orphan",
            ""
        );
    }

    #[test]
    fn test_directed_graph_via_cci() {
        let g = create_directed_graph();
        let starts: Vec<&&str> = ["orphan", "7", "3"]
            .iter()
            .map(|s| g.vertices().find(|v| *v == s).unwrap())
            .collect();
        run_checked!(
            ClosestFirstIterator::from_starts(&g, starts).unwrap(),
            comps(&[("orphan,3,7,5,9,6,4,1,2,8", 10)]),
            "orphan,3,7,5,9,6,4,1,2,8",
            ""
        );
    }

    #[test]
    fn test_directed_graph_null_constructors() {
        let g = create_directed_graph();
        for _ in 0..2 {
            run_checked!(
                ClosestFirstIterator::new(&g),
                comps(&[("1,2,3,5,6,7,9,4,8", 9), ("1,2,3,5,6,7,9,4,8,orphan", 1)]),
                "1,2,3,5,6,7,9,4,8,orphan",
                ""
            );
        }
    }

    #[test]
    fn test_radius() {
        let g = create_directed_graph();
        let start = g.vertices().find(|v| **v == "1").unwrap();
        let it = ClosestFirstIterator::from_start_with_radius(&g, start, 301.0).unwrap();
        assert_eq!(collect_result(it), "1,2,3,5,6,7");
    }

    #[test]
    fn test_radius_with_cross_component_is_rejected() {
        let g = create_directed_graph();
        let r =
            ClosestFirstIterator::build(&g, None, 5.0, jgrapht::util::heap::JavaPairingHeap::new());
        assert!(r.is_err());
    }

    #[test]
    fn test_multiple_starts() {
        let mut graph = DiGraph::<&str>::pseudo();
        graph.add_vertices(["1", "2", "3", "4", "5", "6", "7", "8", "9"]);
        for (a, b) in [
            ("1", "7"),
            ("2", "4"),
            ("2", "5"),
            ("2", "6"),
            ("2", "7"),
            ("2", "9"),
            ("3", "8"),
            ("4", "2"),
            ("4", "6"),
            ("4", "7"),
            ("6", "2"),
            ("6", "4"),
            ("6", "7"),
            ("8", "2"),
            ("8", "4"),
            ("8", "6"),
            ("8", "9"),
        ] {
            graph.add_edge(&a, &b).unwrap();
        }
        let starts: Vec<&&str> = ["3", "1"]
            .iter()
            .map(|s| graph.vertices().find(|v| *v == s).unwrap())
            .collect();
        let it = ClosestFirstIterator::from_starts_with_radius(&graph, starts, 2.0).unwrap();
        assert_eq!(collect_result(it), "3,1,8,7,4,9,2,6");
    }
}

// ---------------------------------------------------------------- IgnoreDirectionTest / EdgeSelectionTest

mod ignore_direction_test {
    use super::*;

    #[test]
    fn test_directed_graph() {
        let g = create_directed_graph();
        let view = AsUndirected(&g);
        let start = view.vertices().find(|v| **v == "4").unwrap();
        let mut it = DepthFirstIterator::from_start(&view, start).unwrap();
        it.set_cross_component_traversal(true);
        run_checked!(
            it,
            comps(&[("4,9,7,8,2,1,3,6,5", 9), ("4,9,7,8,2,1,3,6,5,orphan", 1)]),
            "4,9,7,8,2,1,3,6,5,orphan",
            "5:6:3:1:2:8:7:9:4:orphan:"
        );
    }
}

mod edge_selection_test {
    use super::*;

    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    struct StatefulVertex(i32);
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    struct StatefulEdge(&'static str, u32);

    struct Tracker {
        traversed: Rc<RefCell<HashSet<i32>>>,
        finished: Rc<RefCell<HashSet<i32>>>,
    }
    impl TraversalListener<StatefulVertex, StatefulEdge> for Tracker {
        fn vertex_traversed(&mut self, v: &StatefulVertex) {
            self.traversed.borrow_mut().insert(v.0);
        }
        fn vertex_finished(&mut self, v: &StatefulVertex) {
            self.finished.borrow_mut().insert(v.0);
        }
    }

    #[test]
    fn test_edge_selection_override() {
        let mut graph = BaseGraph::<StatefulVertex, StatefulEdge, jgrapht::graph::Directed>::without_edge_factory(GraphType { self_loops: true, ..GraphType::simple() });
        for i in 1..=7 {
            graph.add_vertex(StatefulVertex(i));
        }
        let mut n = 0;
        let mut add =
            |g: &mut BaseGraph<StatefulVertex, StatefulEdge, jgrapht::graph::Directed>,
             a: i32,
             b: i32,
             color: &'static str| {
                n += 1;
                // the simple graph refuses a second parallel edge; JGraphT's DefaultDirectedGraph does too
                let _ = g.add_edge_with(
                    &StatefulVertex(a),
                    &StatefulVertex(b),
                    StatefulEdge(color, n),
                );
            };
        add(&mut graph, 1, 2, "BLUE");
        add(&mut graph, 1, 3, "RED");
        add(&mut graph, 1, 3, "BLUE");
        add(&mut graph, 2, 4, "BLUE");
        add(&mut graph, 2, 5, "BLUE");
        add(&mut graph, 2, 5, "RED");
        add(&mut graph, 3, 4, "RED");
        add(&mut graph, 3, 7, "BLUE");
        add(&mut graph, 4, 6, "RED");
        add(&mut graph, 6, 2, "BLUE");
        add(&mut graph, 7, 5, "RED");

        let mut iterator = DepthFirstIterator::new(&graph);
        iterator.set_edge_selector(|g, v| {
            g.out_edges_of(v)
                .filter(|e| {
                    if v.0 % 2 == 0 {
                        e.0 == "BLUE"
                    } else {
                        e.0 == "RED"
                    }
                })
                .collect()
        });
        let (traversed, finished) = (
            Rc::new(RefCell::new(HashSet::new())),
            Rc::new(RefCell::new(HashSet::new())),
        );
        iterator.add_traversal_listener(Tracker {
            traversed: traversed.clone(),
            finished: finished.clone(),
        });
        let order: String = iterator.map(|v| v.0.to_string()).collect();
        let all: HashSet<i32> = (1..=7).collect();
        assert_eq!(*traversed.borrow(), all);
        assert_eq!(*finished.borrow(), all);
        assert_eq!(order, "1342567");
    }
}

// ---------------------------------------------------------------- TopologicalOrderIterator

mod topological_order {
    use super::*;

    fn order<'g, G: DirectedGraph>(it: impl Iterator<Item = &'g G::Vertex>) -> Vec<G::Vertex>
    where
        G::Vertex: 'g,
    {
        it.cloned().collect()
    }

    #[test]
    fn test_recipe() {
        let v = [
            "preheat oven",
            "sift dry ingredients",
            "stir wet ingredients",
            "mix wet and dry ingredients",
            "spoon onto pan",
            "bake",
            "cool",
            "frost",
            "eat",
        ];
        let mut graph = DiGraph::<&str>::with_loops();
        for i in [4, 8, 1, 3, 7, 6, 0, 2, 5] {
            graph.add_vertex(v[i]);
        }
        for (a, b) in [
            (0, 1),
            (1, 2),
            (0, 2),
            (1, 3),
            (2, 3),
            (3, 4),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 8),
            (6, 8),
        ] {
            graph.add_edge(&v[a], &v[b]).unwrap();
        }
        let got = order::<Gr2>(TopologicalOrderIterator::new(&graph).unwrap());
        assert_eq!(got, v);
        let reversed = EdgeReversed(&graph);
        let got = order::<EdgeReversed<&DiGraph<&str>>>(
            TopologicalOrderIterator::new(&reversed).unwrap(),
        );
        let mut rev = v.to_vec();
        rev.reverse();
        assert_eq!(got, rev);
    }
    type Gr2 = DiGraph<&'static str>;

    #[test]
    fn test_empty_graph() {
        let graph = DiGraph::<&str>::with_loops();
        assert!(!TopologicalOrderIterator::new(&graph).unwrap().has_next());
    }

    fn graph1() -> DiGraph<&'static str> {
        let mut graph = DiGraph::<&str>::with_loops();
        graph.add_vertices(["v0", "v1", "v2", "v3", "v4", "v5"]);
        for (a, b) in [
            ("v0", "v1"),
            ("v0", "v2"),
            ("v1", "v4"),
            ("v2", "v4"),
            ("v3", "v2"),
            ("v3", "v4"),
            ("v4", "v5"),
        ] {
            graph.add_edge(&a, &b).unwrap();
        }
        graph
    }

    #[test]
    fn test_graph_1() {
        let g = graph1();
        assert_eq!(
            order::<Gr2>(TopologicalOrderIterator::new(&g).unwrap()),
            ["v0", "v3", "v1", "v2", "v4", "v5"]
        );
    }

    #[test]
    fn test_graph_with_partial_order() {
        let g = graph1();
        let cf = |a: &&str, b: &&str| -> i32 {
            if *a == "v0" && *b == "v3" {
                return 1;
            } else if *a == "v3" && *b == "v0" {
                return -1;
            }
            if *a == "v1" && *b == "v2" {
                return 1;
            } else if *a == "v2" && *b == "v1" {
                return -1;
            }
            -1
        };
        let it = TopologicalOrderIterator::with_comparator(&g, cf).unwrap();
        assert_eq!(order::<Gr2>(it), ["v3", "v0", "v2", "v1", "v4", "v5"]);
    }

    #[test]
    fn test_graph_with_parallel_edges() {
        let mut graph = DiGraph::<&str>::pseudo();
        graph.add_vertices(["v0", "v1", "v2", "v3", "v4", "v5"]);
        for (a, b) in [
            ("v0", "v1"),
            ("v0", "v1"),
            ("v0", "v2"),
            ("v1", "v4"),
            ("v2", "v4"),
            ("v2", "v4"),
            ("v3", "v2"),
            ("v3", "v4"),
            ("v4", "v5"),
        ] {
            graph.add_edge(&a, &b).unwrap();
        }
        let got: Vec<&str> = TopologicalOrderIterator::new(&graph)
            .unwrap()
            .copied()
            .collect();
        assert_eq!(got, ["v0", "v3", "v1", "v2", "v4", "v5"]);
    }

    #[test]
    fn test_with_self_loops() {
        let mut graph = DiGraph::<&str>::with_loops();
        graph.add_vertices(["v0", "v1", "v2"]);
        for (a, b) in [("v0", "v1"), ("v0", "v2"), ("v1", "v2"), ("v2", "v2")] {
            graph.add_edge(&a, &b).unwrap();
        }
        assert!(TopologicalOrderIterator::new(&graph).is_err());
    }

    #[test]
    fn test_graph_with_cycle() {
        let mut graph = DiGraph::<&str>::pseudo();
        graph.add_vertices(["v0", "v1", "v2", "v3", "v4", "v5"]);
        for (a, b) in [
            ("v0", "v1"),
            ("v0", "v1"),
            ("v0", "v2"),
            ("v1", "v4"),
            ("v2", "v4"),
            ("v2", "v4"),
            ("v3", "v2"),
            ("v3", "v4"),
            ("v4", "v5"),
            ("v5", "v2"),
        ] {
            graph.add_edge(&a, &b).unwrap();
        }
        let mut it = TopologicalOrderIterator::new(&graph).unwrap();
        while it.next().is_some() {}
        assert!(it.inner().is_cyclic());
    }

    #[test]
    fn test_disconnected() {
        let mut graph = DiGraph::<i32>::pseudo();
        graph.add_vertices([0, 1, 2, 3]);
        graph.add_edge(&0, &1).unwrap();
        graph.add_edge(&2, &3).unwrap();
        let cf = |a: &i32, b: &i32| -> i32 {
            match a.cmp(b) {
                std::cmp::Ordering::Less => 1,
                std::cmp::Ordering::Greater => -1,
                std::cmp::Ordering::Equal => 0,
            }
        };
        let got: Vec<i32> = TopologicalOrderIterator::with_comparator(&graph, cf)
            .unwrap()
            .copied()
            .collect();
        assert_eq!(got, [2, 3, 0, 1]);
    }

    #[test]
    fn test_try_to_disable_cross_component() {
        let graph = DiGraph::<i32>::pseudo();
        let mut it = TopologicalOrderIterator::new(&graph).unwrap();
        assert!(it.set_cross_component_traversal(false).is_err());
        assert!(it.set_cross_component_traversal(true).is_ok());
    }
}

// ---------------------------------------------------------------- tracking listener shared by the ordering iterators

struct Vertices {
    traversed: Rc<RefCell<HashSet<String>>>,
    finished: Rc<RefCell<HashSet<String>>>,
}
impl<V: ToString, E> TraversalListener<V, E> for Vertices {
    fn vertex_traversed(&mut self, v: &V) {
        self.traversed.borrow_mut().insert(v.to_string());
    }
    fn vertex_finished(&mut self, v: &V) {
        self.finished.borrow_mut().insert(v.to_string());
    }
}

fn tracked() -> (
    Vertices,
    Rc<RefCell<HashSet<String>>>,
    Rc<RefCell<HashSet<String>>>,
) {
    let (t, f) = (
        Rc::new(RefCell::new(HashSet::new())),
        Rc::new(RefCell::new(HashSet::new())),
    );
    (
        Vertices {
            traversed: t.clone(),
            finished: f.clone(),
        },
        t,
        f,
    )
}

fn names<V: ToString>(g: &impl Graph<Vertex = V>) -> HashSet<String> {
    g.vertices().map(|v| v.to_string()).collect()
}

// ---------------------------------------------------------------- LexBreadthFirstIterator

mod lex_bfs {
    use super::*;

    #[test]
    fn test_events() {
        let mut graph = UGraph::<i32>::with_loops();
        for (a, b) in [(1, 2), (1, 3), (1, 4), (2, 4), (3, 4)] {
            graph.add_edge_with_vertices(a, b).unwrap();
        }
        let (l, t, f) = tracked();
        let mut it = LexBreadthFirstIterator::new(&graph);
        it.add_traversal_listener(l);
        for _ in 0..4 {
            it.next();
        }
        assert_eq!(*t.borrow(), names(&graph));
        assert_eq!(*f.borrow(), names(&graph));
    }

    #[test]
    fn test_lexicographical_bfs_iterator_1() {
        let graph = UGraph::<i32>::with_loops();
        let mut it = LexBreadthFirstIterator::new(&graph);
        assert!(!it.has_next());
        assert!(it.next().is_none());
    }

    fn visits_all<G: UndirectedGraph>(graph: &G)
    where
        G::Vertex: std::fmt::Debug,
    {
        let mut it = LexBreadthFirstIterator::new(graph);
        let mut returned = HashSet::new();
        for _ in 0..graph.vertex_count() {
            assert!(it.has_next());
            let v = it.next().unwrap();
            assert!(graph.contains_vertex(v));
            returned.insert(v.clone());
        }
        assert_eq!(returned, graph.vertices().cloned().collect());
        assert!(!it.has_next());
    }

    #[test]
    fn test_lexicographical_bfs_iterator_2() {
        let mut graph = UGraph::<i32>::with_loops();
        graph.add_vertices([1, 2, 3, 4]);
        graph.add_edge_with_vertices(1, 2).unwrap();
        graph.add_edge_with_vertices(2, 3).unwrap();
        graph.add_edge_with_vertices(3, 4).unwrap();
        visits_all(&graph);
    }

    #[test]
    fn test_lexicographical_bfs_iterator_3() {
        let mut graph = UGraph::<&str>::with_loops();
        graph.add_vertices(["a", "b", "c", "d"]);
        visits_all(&graph);
    }

    #[test]
    fn test_lexicographical_bfs_iterator_4() {
        let mut graph = UGraph::<i32>::pseudo();
        graph.add_vertices([1, 2, 3]);
        for (a, b) in [
            (1, 1),
            (1, 2),
            (1, 2),
            (1, 3),
            (1, 3),
            (2, 3),
            (2, 3),
            (3, 3),
            (3, 3),
        ] {
            graph.add_edge_with_vertices(a, b).unwrap();
        }
        visits_all(&graph);
    }
}

// ---------------------------------------------------------------- MaximumCardinalityIterator

mod max_cardinality {
    use super::*;

    #[test]
    fn test_events() {
        let mut graph = UGraph::<&str>::with_loops();
        for (a, b) in [("a", "b"), ("b", "c"), ("c", "a"), ("b", "d")] {
            graph.add_edge_with_vertices(a, b).unwrap();
        }
        let (l, t, f) = tracked();
        let mut it = MaximumCardinalityIterator::new(&graph);
        it.add_traversal_listener(l);
        for _ in 0..4 {
            it.next();
        }
        assert_eq!(*t.borrow(), names(&graph));
        assert_eq!(*f.borrow(), names(&graph));
    }

    #[test]
    fn test_maximum_cardinality_iterator_1() {
        let graph = UGraph::<i32>::with_loops();
        let mut it = MaximumCardinalityIterator::new(&graph);
        assert!(!it.has_next());
        assert!(it.next().is_none());
    }

    fn visits_all<G: UndirectedGraph>(graph: &G)
    where
        G::Vertex: std::fmt::Debug,
    {
        let mut it = MaximumCardinalityIterator::new(graph);
        let mut returned = HashSet::new();
        for _ in 0..graph.vertex_count() {
            assert!(it.has_next());
            let v = it.next().unwrap();
            assert!(graph.contains_vertex(v));
            returned.insert(v.clone());
        }
        assert_eq!(returned, graph.vertices().cloned().collect());
        assert!(!it.has_next());
    }

    #[test]
    fn test_maximum_cardinality_iterator_2() {
        let mut graph = UGraph::<&str>::with_loops();
        for (a, b) in [("a", "b"), ("b", "c"), ("b", "d"), ("c", "d")] {
            graph.add_edge_with_vertices(a, b).unwrap();
        }
        visits_all(&graph);
    }

    #[test]
    fn test_maximum_cardinality_iterator_3() {
        let mut graph = UGraph::<i32>::with_loops();
        graph.add_vertices([1, 2, 3, 4]);
        visits_all(&graph);
    }

    #[test]
    fn test_maximum_cardinality_iterator_4() {
        let mut graph = UGraph::<&str>::pseudo();
        for (a, b) in [
            ("a", "a"),
            ("a", "b"),
            ("a", "b"),
            ("a", "c"),
            ("a", "c"),
            ("b", "c"),
            ("b", "c"),
            ("c", "c"),
            ("c", "c"),
        ] {
            graph.add_edge_with_vertices(a, b).unwrap();
        }
        visits_all(&graph);
    }
}

// ---------------------------------------------------------------- DegeneracyOrderingIterator

mod degeneracy_ordering {
    use super::*;

    #[test]
    fn test_graph_1() {
        let mut g = UGraph::<&str>::simple();
        g.add_vertices(["v1", "v2", "v3", "v4", "v5", "v6", "v7", "v8", "v9", "v10"]);
        for (a, b) in [
            ("v1", "v2"),
            ("v1", "v3"),
            ("v1", "v4"),
            ("v2", "v3"),
            ("v2", "v4"),
            ("v3", "v4"),
            ("v5", "v6"),
            ("v5", "v7"),
            ("v6", "v7"),
            ("v3", "v5"),
            ("v4", "v5"),
            ("v7", "v8"),
            ("v1", "v9"),
            ("v1", "v10"),
            ("v2", "v9"),
            ("v2", "v10"),
            ("v9", "v10"),
        ] {
            g.add_edge(&a, &b).unwrap();
        }
        let sb: String = DegeneracyOrderingIterator::new(&g)
            .map(|v| format!(",{v}"))
            .collect();
        assert_eq!(sb, ",v8,v6,v7,v5,v9,v10,v3,v4,v1,v2");
    }

    struct Seq(Rc<RefCell<String>>);
    impl TraversalListener<&'static str, DefaultEdge> for Seq {
        fn connected_component_started(&mut self) {
            panic!("Should not be called");
        }
        fn connected_component_finished(&mut self) {
            panic!("Should not be called");
        }
        fn edge_traversed(&mut self, _: &DefaultEdge) {
            panic!("Should not be called");
        }
        fn vertex_traversed(&mut self, v: &&'static str) {
            self.0.borrow_mut().push_str(&format!(",s_{v}"));
        }
        fn vertex_finished(&mut self, v: &&'static str) {
            self.0.borrow_mut().push_str(&format!(",f_{v}"));
        }
    }

    #[test]
    fn test_graph_with_listener() {
        let mut g = UGraph::<&str>::simple();
        g.add_vertices(["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"]);
        for (a, b) in [
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
        ] {
            g.add_edge(&a, &b).unwrap();
        }
        let log = Rc::new(RefCell::new(String::new()));
        let mut it = DegeneracyOrderingIterator::new(&g);
        it.add_traversal_listener(Seq(log.clone()));
        while it.next().is_some() {}
        assert_eq!(
            *log.borrow(),
            ",s_a,f_a,s_b,f_b,s_d,f_d,s_c,f_c,s_j,f_j,s_k,f_k,s_i,f_i,s_g,f_g,s_h,f_h,s_e,f_e,s_f,f_f"
        );
    }
}

// ---------------------------------------------------------------- RandomWalkVertexIterator

mod random_walk {
    use super::*;

    #[test]
    fn test_invalid_vertex() {
        let graph = DiGraph::<&str>::with_loops();
        assert!(
            RandomWalkVertexIterator::new(&graph, &"unknown", 100, false, JavaRandom::new(1))
                .is_err()
        );
    }

    #[test]
    fn test_single_node() {
        let mut graph = DiGraph::<&str>::with_loops();
        graph.add_vertex("123");
        let start = graph.vertices().next().unwrap();
        let mut it =
            RandomWalkVertexIterator::new(&graph, start, u64::MAX, false, JavaRandom::new(1))
                .unwrap();
        assert_eq!(it.next(), Some(&"123"));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_sink() {
        let mut graph = DiGraph::<String>::with_loops();
        Linear(10).generate(&mut graph, |i| i.to_string()).unwrap();
        let start = graph.vertices().next().unwrap();
        let it = RandomWalkVertexIterator::new(&graph, start, u64::MAX, false, JavaRandom::new(7))
            .unwrap();
        assert_eq!(it.count(), 10);
    }

    #[test]
    fn test_exhausted() {
        let mut graph = UGraph::<String>::simple();
        Ring(10)
            .generate(&mut graph, |i| (i + 1).to_string())
            .unwrap();
        let start = graph.vertices().find(|v| *v == "1").unwrap();
        let it =
            RandomWalkVertexIterator::new(&graph, start, 4, false, JavaRandom::new(3)).unwrap();
        assert_eq!(it.count(), 5);
    }

    #[test]
    fn test_deterministic() {
        let mut graph = DiGraph::<String>::with_loops();
        Ring(5).generate(&mut graph, |i| i.to_string()).unwrap();
        let start = graph.vertices().next().unwrap();
        let it =
            RandomWalkVertexIterator::new(&graph, start, 20, false, JavaRandom::new(11)).unwrap();
        for (step, v) in it.enumerate() {
            assert_eq!(*v, (step % 5).to_string());
        }
    }

    #[test]
    fn test_long_time() {
        let mut graph = UGraph::<String>::simple();
        graph.add_vertices(["0".to_string(), "1".to_string(), "2".to_string()]);
        graph.add_edge(&"0".to_string(), &"1".to_string()).unwrap();
        graph.add_edge(&"1".to_string(), &"2".to_string()).unwrap();
        let start = graph.vertices().next().unwrap();
        let it = RandomWalkVertexIterator::new(&graph, start, u64::MAX, false, JavaRandom::new(5))
            .unwrap();
        assert_eq!(it.take(10000).count(), 10000);
    }
}
