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

//! Ports of `org.jgrapht.alg.decomposition` tests: DulmageMendelsohnDecompositionTest and
//! HeavyPathDecompositionTest.
//!
//! The Java tests of `null` graph / `null` root are not applicable. The forest generators get a
//! fresh `StdRng` seeded from the test's random stream (Java shares one `Random`).

use jgrapht::alg::connectivity::connected_component_of;
use jgrapht::alg::decomposition::{
    DmDecomposition, DulmageMendelsohnDecomposition, HeavyPathDecomposition,
};
use jgrapht::alg::matching::{HopcroftKarpMaximumCardinalityBipartiteMatching, MatchingAlgorithm};
use jgrapht::generate::*;
use jgrapht::traverse::BreadthFirstIterator;
use jgrapht::view::AsSubgraph;
use jgrapht::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

mod dulmage_mendelsohn_decomposition_test {
    use super::*;

    fn max_edges(n: i32) -> i32 {
        if n % 2 == 0 {
            n / 2 * (n - 1)
        } else {
            n * ((n - 1) / 2)
        }
    }

    fn random_imbalance(random: &mut StdRng, n: i32) -> i32 {
        let max = n.div_euclid(4);
        random.random_range(0..max * 2) - max
    }

    fn assert_valid_decomposition(
        graph: &UGraph<i32>,
        decomposition: &DmDecomposition<i32>,
        partition1: &HashSet<i32>,
        partition2: &HashSet<i32>,
    ) {
        let mut all_perfect = HashSet::new();
        let (mut p1_perfect, mut p2_perfect) = (HashSet::new(), HashSet::new());
        for set in &decomposition.perfect_matched_sets {
            all_perfect.extend(set.iter().copied());
            for v in set {
                if partition1.contains(v) {
                    p1_perfect.insert(*v);
                }
                if partition2.contains(v) {
                    p2_perfect.insert(*v);
                }
            }
        }
        let sub = AsSubgraph::induced(graph, all_perfect.iter().copied());
        let perfect =
            HopcroftKarpMaximumCardinalityBipartiteMatching::new(&sub, p1_perfect, p2_perfect)
                .unwrap()
                .get_matching()
                .unwrap();
        assert!(
            perfect.is_perfect(),
            "Core of decomposition must perfectly match"
        );

        for v in graph.vertices() {
            if all_perfect.contains(v) {
                assert!(
                    !decomposition.partition1_dominated.contains(v),
                    "Vertex appears in multiple sets in decomposition"
                );
                assert!(
                    !decomposition.partition2_dominated.contains(v),
                    "Vertex appears in multiple sets in decomposition"
                );
            } else if decomposition.partition1_dominated.contains(v) {
                assert!(
                    !decomposition.partition2_dominated.contains(v),
                    "Vertex appears in multiple sets in decomposition"
                );
            } else {
                assert!(
                    decomposition.partition2_dominated.contains(v),
                    "Vertex appears in multiple sets in decomposition"
                );
            }
        }
        let count = |set: &HashSet<i32>| {
            let n1 = set.iter().filter(|v| partition1.contains(*v)).count();
            (n1, set.len() - n1)
        };
        let (n1, n2) = count(&decomposition.partition1_dominated);
        assert!(
            n1 > n2 || (n1 == 0 && n2 == 0),
            "Partition 1 dominated set is not dominated by partition 1"
        );
        let (n1, n2) = count(&decomposition.partition2_dominated);
        assert!(
            n1 < n2 || (n1 == 0 && n2 == 0),
            "Partition 2 dominated set is not dominated by partition 2"
        );
    }

    #[test]
    fn test_generated_graph() {
        let mut random = StdRng::seed_from_u64(1_u64);
        for vertices in 20..120 {
            let edges = random.random_range(0..max_edges(vertices) / 2);
            let imbalance = random_imbalance(&mut random, vertices);
            let mut generator = GnmRandomBipartiteGraphGenerator::new(
                (vertices - imbalance) as usize,
                (vertices + imbalance) as usize,
                edges as usize,
                StdRng::seed_from_u64(0_u64),
            );
            let mut graph = UGraph::<i32>::simple().with_vertex_factory(|i| i as i32);
            let (a, b) = generator.generate_partitions(&mut graph).unwrap();
            let (p1, p2): (HashSet<i32>, HashSet<i32>) =
                (a.into_iter().collect(), b.into_iter().collect());
            let dm = DulmageMendelsohnDecomposition::new(&graph, p1.clone(), p2.clone());
            assert_valid_decomposition(&graph, &dm.decomposition(true).unwrap(), &p1, &p2);
            assert_valid_decomposition(&graph, &dm.decomposition(false).unwrap(), &p1, &p2);
        }
    }
}

mod heavy_path_decomposition_test {
    use super::*;

    type G = UGraph<i32>;
    type SG = UGraph<String>;

    fn log2(n: usize) -> usize {
        n.checked_ilog2().unwrap_or(0) as usize
    }

    fn count_max_path<Gr: IncidenceGraph>(graph: &Gr, d: &HeavyPathDecomposition<Gr>) -> usize {
        let paths = d.path_decomposition().unwrap();
        let in_path: HashSet<&Gr::Vertex> = paths
            .paths()
            .iter()
            .flat_map(|p| p.vertex_list().iter())
            .collect();
        let mut maxim = 0;
        for v in graph.vertices() {
            if in_path.contains(v) {
                let (mut cnt, mut cur) = (0, v.clone());
                while let Some(u) = d.parent_of(&cur) {
                    let edge = graph.get_edge(u, &cur).unwrap();
                    if d.light_edges().contains(edge) {
                        cnt += 1;
                    }
                    cur = u.clone();
                }
                maxim = maxim.max(cnt);
            }
        }
        maxim
    }

    fn is_valid_decomposition<Gr: IncidenceGraph>(
        graph: &Gr,
        roots: &[Gr::Vertex],
        d: &HeavyPathDecomposition<Gr>,
    ) -> bool {
        let heavy = d.heavy_edges();
        let light = d.light_edges();
        let all: HashSet<&Gr::Edge> = heavy.iter().chain(light.iter()).collect();
        if all != graph.edges().collect::<HashSet<_>>() {
            return false;
        }
        let paths = d.path_decomposition().unwrap();
        let mut which_path: HashMap<&Gr::Vertex, usize> = HashMap::new();
        for (i, path) in paths.paths().iter().enumerate() {
            for (j, v) in path.vertex_list().iter().enumerate() {
                if which_path.insert(v, i).is_some() {
                    return false;
                }
                if j > 0 && !graph.contains_edge_between(&path.vertex_list()[j - 1], v) {
                    return false;
                }
            }
        }
        for root in roots {
            for v in connected_component_of(graph, root).unwrap() {
                if !which_path.contains_key(&v) {
                    return false;
                }
            }
        }
        for root in roots {
            let root_ref = graph.vertices().find(|v| *v == root).unwrap();
            let mut bfs = BreadthFirstIterator::from_start(graph, root_ref).unwrap();
            let mut order = Vec::new();
            for v in bfs.by_ref() {
                order.push(v);
            }
            order.reverse();
            let mut size_subtree: HashMap<&Gr::Vertex, usize> = HashMap::new();
            for v in order {
                size_subtree.insert(v, 1);
                for edge in graph.edges_of(v) {
                    let u = graph.opposite_vertex(edge, v).unwrap();
                    if Some(u) != bfs.parent(v) {
                        let size_u = size_subtree[u];
                        *size_subtree.get_mut(v).unwrap() += size_u;
                    }
                }
                for edge in graph.edges_of(v) {
                    let u = graph.opposite_vertex(edge, v).unwrap();
                    if Some(u) == bfs.parent(v) {
                        continue;
                    }
                    if light.contains(edge) {
                        if 2 * size_subtree[u] > size_subtree[v] {
                            return false;
                        }
                    } else if 2 * size_subtree[u] <= size_subtree[v] {
                        return false;
                    }
                }
            }
        }
        count_max_path(graph, d) <= log2(graph.vertex_count())
    }

    fn path_graph(n: i32) -> G {
        let mut g = G::simple();
        g.add_vertices(1..=n);
        for i in 1..n {
            g.add_edge(&i, &(i + 1)).unwrap();
        }
        g
    }

    #[test]
    fn test_root_not_in_tree() {
        let mut graph = SG::simple();
        graph.add_vertex("a".into());
        assert!(HeavyPathDecomposition::new(&graph, "b".to_string()).is_err());
    }

    #[test]
    fn test_no_heavy_edges() {
        let mut graph = SG::simple();
        graph.add_vertices(["1", "2", "3", "4"].map(String::from));
        for t in ["2", "3", "4"] {
            graph.add_edge(&"1".to_string(), &t.to_string()).unwrap();
        }
        let d = HeavyPathDecomposition::new(&graph, "1".to_string()).unwrap();
        assert!(d.heavy_edges().is_empty());
        assert!(is_valid_decomposition(&graph, &["1".to_string()], &d));
    }

    #[test]
    fn test_one_vertex() {
        let mut graph = SG::simple();
        graph.add_vertex("a".into());
        let d = HeavyPathDecomposition::new(&graph, "a".to_string()).unwrap();
        assert_eq!(d.path_decomposition().unwrap().number_of_paths(), 1);
        assert!(is_valid_decomposition(&graph, &["a".to_string()], &d));
    }

    #[test]
    fn test_line_graph() {
        let graph = path_graph(11);
        let d = HeavyPathDecomposition::new(&graph, 1).unwrap();
        assert_eq!(d.path_decomposition().unwrap().number_of_paths(), 1);
        assert!(is_valid_decomposition(&graph, &[1], &d));
    }

    #[test]
    fn test_line_graph_2() {
        let graph = path_graph(11);
        let d = HeavyPathDecomposition::new(&graph, 5).unwrap();
        assert_eq!(d.path_decomposition().unwrap().number_of_paths(), 2);
        assert!(is_valid_decomposition(&graph, &[5], &d));
    }

    #[test]
    fn test_small_tree() {
        let mut graph = G::simple();
        graph.add_vertices(1..=11);
        for (a, b) in [
            (1, 2),
            (2, 4),
            (2, 5),
            (2, 6),
            (4, 7),
            (4, 8),
            (6, 9),
            (1, 3),
            (3, 10),
            (3, 11),
        ] {
            graph.add_edge(&a, &b).unwrap();
        }
        let d = HeavyPathDecomposition::new(&graph, 1).unwrap();
        assert!(is_valid_decomposition(&graph, &[1], &d));
    }

    #[test]
    fn test_disconnected_small_graph() {
        let mut graph = G::simple();
        graph.add_vertices([1, 2]);
        graph.add_edge(&1, &2).unwrap();
        graph.add_vertices([3, 4]);
        graph.add_edge(&3, &4).unwrap();
        let d = HeavyPathDecomposition::new(&graph, 1).unwrap();
        assert_eq!(d.path_decomposition().unwrap().number_of_paths(), 1);
        assert!(d.heavy_edges().is_empty());
        assert_eq!(d.light_edges().len(), 1);
    }

    fn forest(trees: usize, n: usize, seed: i64) -> G {
        let mut graph = G::simple().with_vertex_factory(|i| i as i32);
        BarabasiAlbertForestGenerator::new(trees, n, StdRng::seed_from_u64((seed) as u64))
            .unwrap()
            .generate_default(&mut graph)
            .unwrap();
        graph
    }

    #[test]
    fn test_random_trees() {
        let mut random = StdRng::seed_from_u64(0x2882_u64);
        for _ in 0..100 {
            let n = 1024 + random.random_range(0..1 << 12) as usize;
            let graph = forest(1, n, random.random::<i64>());
            let roots = vec![*graph.vertices().next().unwrap()];
            let d = HeavyPathDecomposition::with_roots(&graph, roots.clone()).unwrap();
            assert!(is_valid_decomposition(&graph, &roots, &d));
        }
    }

    #[test]
    fn test_random_forests() {
        let mut random = StdRng::seed_from_u64(0x1881_u64);
        for _ in 0..1000 {
            let trees = 1 + random.random_range(0..20) as usize;
            let n = 50 + random.random_range(0..1 << 11) as usize;
            let graph = forest(trees, n, random.random::<i64>());
            let roots: Vec<i32> = jgrapht::alg::connectivity::connected_components(&graph)
                .into_iter()
                .map(|c| c[0])
                .collect();
            let d = HeavyPathDecomposition::with_roots(&graph, roots.clone()).unwrap();
            assert!(is_valid_decomposition(&graph, &roots, &d));
        }
    }

    #[test]
    fn test_huge_tree() {
        let graph = forest(
            1,
            1 << 19,
            StdRng::seed_from_u64(0x118811_u64).random::<i64>(),
        );
        let roots = vec![*graph.vertices().next().unwrap()];
        let d = HeavyPathDecomposition::with_roots(&graph, roots.clone()).unwrap();
        assert!(is_valid_decomposition(&graph, &roots, &d));
    }

    #[test]
    fn multiple_roots_in_one_tree_is_an_error() {
        let graph = path_graph(5);
        assert!(HeavyPathDecomposition::with_roots(&graph, [1, 3]).is_err());
    }
}
