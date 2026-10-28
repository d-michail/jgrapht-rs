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

//! Ports of `org.jgrapht.alg.util` tests: AliasMethodSamplerTest, FixedSizeIntegerQueueTest,
//! NeighborCacheTest, PairTest, UnionFindTest and VertexDegreeComparatorTest.
//!
//! Differences: Java's `null` elements are `Option`s; `UnorderedPair` has one element type, so the
//! cross-type equality checks of `PairTest` use an enum; `NeighborCache` results are snapshots
//! rather than live views, so the tests query again after each change.

use jgrapht::alg::util::*;
use jgrapht::event::Listenable;
use jgrapht::generate::{GnmRandomGraphGenerator, GraphGenerator};
use jgrapht::util::sampler::AliasMethodSampler;
use jgrapht::*;
use rand::{SeedableRng, rngs::StdRng};
use std::collections::HashSet;

mod alias_method_sampler_test {
    use super::*;

    fn counts(prob: &[f64], seed: i64) -> Vec<i32> {
        let sampler = AliasMethodSampler::new(prob).unwrap();
        let mut rng = StdRng::seed_from_u64((seed) as u64);
        let mut counts = vec![0; prob.len()];
        for _ in 0..1_000_000 {
            counts[sampler.next(&mut rng)] += 1;
        }
        counts
    }

    /// The sample frequencies must match the probabilities: 1,000,000 draws leave a standard
    /// deviation below 500 per outcome, so a deviation of 3,000 (six sigma) means a wrong table.
    fn assert_close(counts: &[i32], prob: &[f64]) {
        for (c, p) in counts.iter().zip(prob) {
            assert!((*c as f64 - p * 1_000_000.0).abs() < 3000.0, "{counts:?}");
        }
    }

    #[test]
    fn test1() {
        assert_close(&counts(&[0.1, 0.2, 0.3, 0.4], 5), &[0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn test2() {
        let prob = [0.05, 0.05, 0.05, 0.05, 0.8];
        assert_close(&counts(&prob, 17), &prob);
    }

    #[test]
    fn test_non_valid() {
        assert!(AliasMethodSampler::new(&[0.5, 0.6]).is_err());
    }
}

mod fixed_size_integer_queue_test {
    use super::*;

    #[test]
    fn test_queue() {
        let mut queue = FixedSizeIntegerQueue::new(10);
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        queue.enqueue(1);
        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.poll(), Some(1));
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        queue.enqueue(2);
        assert!(!queue.is_empty());
        queue.clear();
        assert!(queue.is_empty());
    }
}

mod vertex_degree_comparator_test {
    use super::*;

    #[test]
    fn test_vertex_degree_comparator() {
        let mut generator =
            GnmRandomGraphGenerator::new(100, 1000, StdRng::seed_from_u64(0), false, false)
                .unwrap();
        for _ in 0..20 {
            let mut graph = UGraph::<i32>::simple().with_vertex_factory(|i| i as i32);
            generator.generate_default(&mut graph).unwrap();
            let mut vertices: Vec<i32> = graph.vertices_cloned().collect();
            vertices.sort_by(vertex_degree_comparator(&graph));
            for w in vertices.windows(2) {
                assert!(graph.degree_of(&w[0]) <= graph.degree_of(&w[1]));
            }
        }
    }
}

mod union_find_test {
    use super::*;

    fn union(sets: &mut Vec<Vec<&'static str>>, a: &str, b: &str) {
        let mut to_add = Vec::new();
        for key in [a, b] {
            if let Some(i) = sets.iter().position(|s| s.contains(&key)) {
                to_add.extend(sets.remove(i));
            }
        }
        sets.push(to_add);
    }

    fn same(sets: &[Vec<&str>], a: &str, b: &str) -> bool {
        sets.iter().any(|s| s.contains(&a) && s.contains(&b))
    }

    fn identical(
        universe: &[&'static str],
        sets: &[Vec<&'static str>],
        uf: &mut UnionFind<&'static str>,
    ) {
        for a in universe {
            for b in universe {
                let same1 = uf.find(a).unwrap() == uf.find(b).unwrap();
                assert_eq!(same1, same(sets, a, b));
            }
        }
    }

    #[test]
    fn test_union_find() {
        let strs = ["aaa", "bbb", "ccc", "ddd", "eee"];
        let mut sets: Vec<Vec<&'static str>> = strs.iter().map(|s| vec![*s]).collect();
        let mut uf = UnionFind::new(strs);
        assert_eq!(uf.len(), 5);
        assert_eq!(uf.number_of_sets(), 5);
        identical(&strs, &sets, &mut uf);
        assert!(uf.contains(&"aaa"));
        assert!(uf.contains(&"eee"));
        assert!(!uf.contains(&"fff"));

        uf.union(&strs[0], &strs[1]).unwrap();
        assert_eq!(uf.number_of_sets(), 4);
        union(&mut sets, strs[0], strs[1]);
        identical(&strs, &sets, &mut uf);
        assert!(uf.in_same_set(&"aaa", &"bbb").unwrap());
        assert!(!uf.in_same_set(&"bbb", &"ccc").unwrap());

        uf.union(&strs[2], &strs[3]).unwrap();
        assert_eq!(uf.number_of_sets(), 3);
        union(&mut sets, strs[2], strs[3]);
        identical(&strs, &sets, &mut uf);

        uf.union(&strs[2], &strs[4]).unwrap();
        assert_eq!(uf.number_of_sets(), 2);
        union(&mut sets, strs[2], strs[4]);
        identical(&strs, &sets, &mut uf);

        uf.union(&strs[2], &strs[4]).unwrap();
        assert_eq!(uf.number_of_sets(), 2);
        union(&mut sets, strs[2], strs[4]);
        identical(&strs, &sets, &mut uf);

        uf.union(&strs[0], &strs[4]).unwrap();
        assert_eq!(uf.number_of_sets(), 1);
        union(&mut sets, strs[0], strs[4]);
        identical(&strs, &sets, &mut uf);

        uf.add_element("fff").unwrap();
        assert_eq!(uf.number_of_sets(), 2);
        assert_eq!(uf.len(), 6);
        uf.reset();
        assert_eq!(uf.number_of_sets(), 6);
    }

    #[test]
    fn errors_and_display() {
        let mut uf = UnionFind::new([1, 2, 3]);
        assert!(uf.find(&9).is_err());
        assert!(uf.union(&1, &9).is_err());
        assert!(uf.add_element(1).is_err());
        uf.union(&1, &2).unwrap();
        assert_eq!(uf.to_string(), "{{1:1,2}, {3:3}}");
    }
}

mod neighbor_cache_test {
    use super::*;

    use std::cell::RefCell;
    use std::rc::Rc;

    const V1: &str = "v1";
    const V2: &str = "v2";
    const V3: &str = "v3";

    type Cache = Rc<RefCell<NeighborCache<String>>>;

    fn setup<D: graph::Direction>(
        g: BaseGraph<String, DefaultEdge, D>,
    ) -> (Listenable<BaseGraph<String, DefaultEdge, D>>, Cache) {
        let mut g = Listenable::new(g);
        let cache = NeighborCache::shared();
        g.add_listener(cache.clone());
        (g, cache)
    }

    fn s(x: &str) -> String {
        x.to_string()
    }

    fn remove_between<G: GraphMut>(g: &mut G, a: &G::Vertex, b: &G::Vertex) {
        let e = g.get_edge(a, b).cloned().unwrap();
        g.remove_edge(&e);
    }

    fn neighbors(
        cache: &Cache,
        g: &impl IncidenceGraph<Vertex = String>,
        v: &str,
    ) -> HashSet<String> {
        cache
            .borrow_mut()
            .neighbors_of(g, &s(v))
            .unwrap()
            .iter()
            .cloned()
            .collect()
    }

    #[test]
    fn test_neighbor_set() {
        let (mut g, cache) = setup(UGraph::<String>::simple());
        g.add_vertex(s(V1));
        g.add_vertex(s(V2));
        g.add_edge(&s(V1), &s(V2)).unwrap();
        let n1 = neighbors(&cache, g.inner(), V1);
        assert_eq!(n1.len(), 1);
        assert!(n1.contains(V2));

        g.add_vertex(s(V3));
        g.add_edge(&s(V3), &s(V1)).unwrap();
        let n1 = neighbors(&cache, g.inner(), V1);
        let n3 = neighbors(&cache, g.inner(), V3);
        assert_eq!(n1.len(), 2);
        assert!(n1.contains(V3));
        assert_eq!(n3.len(), 1);
        assert!(n3.contains(V1));

        remove_between(&mut g, &s(V3), &s(V1));
        let n1 = neighbors(&cache, g.inner(), V1);
        assert_eq!(n1.len(), 1);
        assert!(!n1.contains(V3));
        assert_eq!(neighbors(&cache, g.inner(), V3).len(), 0);

        g.remove_vertex(&s(V2));
        assert_eq!(neighbors(&cache, g.inner(), V1).len(), 0);
    }

    #[test]
    fn test_directed_neighbor_set() {
        let mut g = Listenable::new(DiGraph::<String>::simple());
        g.add_vertex(s(V1));
        g.add_vertex(s(V2));
        g.add_edge(&s(V1), &s(V2)).unwrap();
        let cache = NeighborCache::shared();
        g.add_listener(cache.clone());
        let preds = |g: &Listenable<DiGraph<String>>, v: &str| -> HashSet<String> {
            cache
                .borrow_mut()
                .predecessors_of(g.inner(), &s(v))
                .unwrap()
                .iter()
                .cloned()
                .collect()
        };
        let succs = |g: &Listenable<DiGraph<String>>, v: &str| -> HashSet<String> {
            cache
                .borrow_mut()
                .successors_of(g.inner(), &s(v))
                .unwrap()
                .iter()
                .cloned()
                .collect()
        };

        assert_eq!(preds(&g, V1).len(), 0);
        let sv1 = succs(&g, V1);
        assert_eq!(sv1.len(), 1);
        assert!(sv1.contains(V2));

        g.add_vertex(s(V3));
        g.add_edge(&s(V3), &s(V1)).unwrap();
        let q = succs(&g, V3);
        let p = preds(&g, V1);
        assert_eq!(p.len(), 1);
        assert_eq!(succs(&g, V1).len(), 1);
        assert!(p.contains(V3));
        assert_eq!(q.len(), 1);
        assert!(q.contains(V1));

        remove_between(&mut g, &s(V3), &s(V1));
        assert_eq!(succs(&g, V3).len(), 0);
        assert_eq!(preds(&g, V1).len(), 0);

        g.remove_vertex(&s(V2));
        assert_eq!(succs(&g, V1).len(), 0);
    }

    fn abcd() -> (Listenable<UGraph<String>>, Cache) {
        let (mut g, cache) = setup(UGraph::<String>::simple());
        for v in ["A", "B", "C", "D"] {
            g.add_vertex(s(v));
        }
        for v in ["A", "B", "C"] {
            g.add_edge(&s("D"), &s(v)).unwrap();
        }
        (g, cache)
    }

    #[test]
    fn test_vertex_removal() {
        let (mut g, cache) = abcd();
        let d = neighbors(&cache, g.inner(), "D");
        assert_eq!(d, HashSet::from([s("A"), s("B"), s("C")]));
        for v in ["A", "B", "C"] {
            assert_eq!(neighbors(&cache, g.inner(), v).len(), 1);
        }
        g.remove_vertex(&s("D"));
        for v in ["A", "B", "C"] {
            assert_eq!(neighbors(&cache, g.inner(), v).len(), 0);
        }
        assert!(cache.borrow_mut().neighbors_of(g.inner(), &s("D")).is_err());
    }

    #[test]
    fn test_neighbor_list_creation() {
        let (mut g, cache) = abcd();
        let list = |g: &Listenable<UGraph<String>>, v: &str| -> Vec<String> {
            cache
                .borrow_mut()
                .neighbor_list_of(g.inner(), &s(v))
                .unwrap()
        };
        assert_eq!(list(&g, "B"), [s("D")]);

        g.add_edge(&s("A"), &s("B")).unwrap();
        let l = list(&g, "B");
        assert_eq!(l.len(), 2);
        assert!(l.contains(&s("A")) && l.contains(&s("D")));

        remove_between(&mut g, &s("D"), &s("B"));
        assert_eq!(list(&g, "B"), [s("A")]);

        g.remove_vertex(&s("B"));
        assert!(
            cache
                .borrow_mut()
                .neighbor_list_of(g.inner(), &s("B"))
                .is_err()
        );
    }
}

mod pair_test {
    use super::*;

    const ANOTHER: &str = "another";
    const CUSTOM: &str = "custom";

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct Custom(i32);

    /// Heterogeneous elements for the cross-type comparisons of the Java test.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum Elem {
        Null,
        S(&'static str),
        C(Custom),
    }

    fn s(x: &'static str) -> Option<&'static str> {
        Some(x)
    }

    #[test]
    fn test_pair() {
        let p = Pair::of(CUSTOM, Custom(1));
        assert_eq!(*p.first(), CUSTOM);
        assert_ne!(*p.first(), ANOTHER);
        assert_eq!(*p.second(), Custom(1));
        assert_ne!(*p.second(), Custom(2));
        assert!(p.has_first(&CUSTOM));
        assert!(!p.has_first(&ANOTHER));
        assert!(p.has_second(&Custom(1)));
        assert!(!p.has_second(&Custom(2)));
    }

    #[test]
    fn test_unordered_pair() {
        let p = UnorderedPair::of(Elem::S(CUSTOM), Elem::C(Custom(1)));
        assert_eq!(p.first, Elem::S(CUSTOM));
        assert!(p.has_element(&Elem::S(CUSTOM)));
        assert!(!p.has_element(&Elem::S(ANOTHER)));
        assert!(p.has_element(&Elem::C(Custom(1))));
        assert!(!p.has_element(&Elem::C(Custom(2))));
        assert!(!p.has_element(&Elem::Null));
    }

    #[test]
    fn test_pair_with_null() {
        let p: Pair<Option<&str>, Option<Custom>> = Pair::of(None, Some(Custom(1)));
        assert!(p.first().is_none());
        assert_eq!(*p.second(), Some(Custom(1)));
        assert!(p.has_first(&None));
        assert!(p.has_second(&Some(Custom(1))));
    }

    #[test]
    fn test_unordered_pair_with_null() {
        let p = UnorderedPair::of(Elem::Null, Elem::C(Custom(1)));
        assert_eq!(p.first, Elem::Null);
        assert!(p.has_element(&Elem::Null));
        assert!(p.has_element(&Elem::C(Custom(1))));
    }

    #[test]
    fn test_pair_equals() {
        let p1 = Pair::of(CUSTOM, Custom(1));
        let p2 = Pair::of(ANOTHER, Custom(1));
        let p3 = Pair::of(ANOTHER, Custom(2));
        let p4 = Pair::of(CUSTOM, Custom(1));
        let p5 = Pair::of(CUSTOM, Custom(2));
        assert_ne!(p1, p2);
        assert_ne!(p1, p3);
        assert_eq!(p1, p4);
        assert_ne!(p1, p5);
        assert_ne!(p2, p3);
        assert_ne!(p2, p4);
        assert_ne!(p2, p5);
        assert_ne!(p3, p4);
        assert_ne!(p3, p5);
        assert_ne!(p4, p5);
        // (p6 = Pair<Custom, String> is a different type in Rust, so it can never be equal)
    }

    #[test]
    fn test_unordered_pair_equals() {
        let c = |x| Elem::C(Custom(x));
        let p1 = UnorderedPair::of(Elem::S(CUSTOM), c(1));
        let p2 = UnorderedPair::of(Elem::S(ANOTHER), c(1));
        let p3 = UnorderedPair::of(Elem::S(ANOTHER), c(2));
        let p4 = UnorderedPair::of(Elem::S(CUSTOM), c(1));
        let p5 = UnorderedPair::of(Elem::S(CUSTOM), c(2));
        let p6 = UnorderedPair::of(c(1), Elem::S(CUSTOM));
        assert_ne!(p1, p2);
        assert_ne!(p1, p3);
        assert_eq!(p1, p4);
        assert_ne!(p1, p5);
        assert_eq!(p1, p6);
        assert_ne!(p2, p3);
        assert_ne!(p2, p4);
        assert_ne!(p2, p5);
        assert_ne!(p2, p6);
        assert_ne!(p3, p4);
        assert_ne!(p3, p5);
        assert_ne!(p3, p6);
        assert_ne!(p4, p5);
        assert_eq!(p4, p6);
        assert_ne!(p5, p6);
    }

    #[test]
    fn test_pair_equals_with_null() {
        let p1: Pair<Option<&str>, Option<Custom>> = Pair::of(s(CUSTOM), None);
        let p3 = Pair::of(s(ANOTHER), None);
        let p4 = Pair::of(s(CUSTOM), None);
        let p5 = Pair::of(s(CUSTOM), Some(Custom(1)));
        assert_ne!(p1, p3);
        assert_eq!(p1, p4);
        assert_ne!(p1, p5);
        assert_ne!(p3, p4);
        assert_ne!(p3, p5);
        assert_ne!(p4, p5);
        let p2: Pair<Option<&str>, Option<&str>> = Pair::of(None, s(CUSTOM));
        let p2b: Pair<Option<&str>, Option<&str>> = Pair::of(s(CUSTOM), None);
        assert_ne!(p2, p2b);
    }

    #[test]
    fn test_unordered_pair_equals_with_null() {
        let p1 = UnorderedPair::of(Elem::S(CUSTOM), Elem::Null);
        let p2 = UnorderedPair::of(Elem::Null, Elem::S(CUSTOM));
        let p3 = UnorderedPair::of(Elem::S(ANOTHER), Elem::Null);
        let p4 = UnorderedPair::of(Elem::S(CUSTOM), Elem::Null);
        let p5 = UnorderedPair::of(Elem::S(CUSTOM), Elem::C(Custom(1)));
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
        assert_eq!(p1, p4);
        assert_ne!(p1, p5);
        assert_ne!(p2, p3);
        assert_eq!(p2, p4);
        assert_ne!(p2, p5);
        assert_ne!(p3, p4);
        assert_ne!(p3, p5);
        assert_ne!(p4, p5);
    }

    #[test]
    fn test_different_types_equals_with_null() {
        let p1: Pair<Option<&str>, Option<Custom>> = Pair::of(None, None);
        assert_eq!(p1, Pair::of(None, None));
        let p4 = UnorderedPair::of(Elem::Null, Elem::Null);
        assert_eq!(p4, UnorderedPair::of(Elem::Null, Elem::Null));
    }

    #[test]
    fn test_unordered_same_hash_code() {
        use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
        let b = BuildHasherDefault::<DefaultHasher>::default();
        let p1 = UnorderedPair::of(Elem::S(CUSTOM), Elem::C(Custom(1)));
        let p2 = UnorderedPair::of(Elem::C(Custom(1)), Elem::S(CUSTOM));
        assert_eq!(b.hash_one(&p1), b.hash_one(&p2));
    }

    #[test]
    fn triple() {
        let t = Triple::of(1, "a", 2.5);
        assert_eq!(
            t,
            Triple {
                first: 1,
                second: "a",
                third: 2.5
            }
        );
    }
}
