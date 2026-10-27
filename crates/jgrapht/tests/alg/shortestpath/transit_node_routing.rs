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

use jgrapht::alg::shortestpath::{
    ContractionHierarchy, ContractionHierarchyPrecomputation, Dijkstra, ShortestPathAlgorithm,
    TransitNodeRouting, TransitNodeRoutingPrecomputation, TransitNodeRoutingShortestPath,
    BidirectionalDijkstraShortestPath,
};
use jgrapht::generate::{GnmRandomGraphGenerator, GraphGenerator};
use jgrapht::util::java::JavaRandom;
use jgrapht::{DiGraph, DefaultEdge, Graph, GraphError, GraphMut, GraphMutExt};
use rand::{SeedableRng, rngs::StdRng};
use std::collections::HashSet;

const SEED: i64 = 19;

fn hierarchy(graph: &DiGraph<usize>) -> ContractionHierarchy<'_, DiGraph<usize>> {
    ContractionHierarchyPrecomputation::with_random_supplier(graph, 4, || JavaRandom::new(SEED))
        .compute_contraction_hierarchy()
        .unwrap()
}

fn random_graph(n: usize, m: usize, random: &mut JavaRandom) -> DiGraph<usize> {
    let mut graph = DiGraph::<usize>::pseudo().weighted();
    GnmRandomGraphGenerator::new(n, m - n + 1, StdRng::seed_from_u64(SEED as u64), true, true)
        .unwrap()
        .generate_indexed(&mut graph)
        .unwrap();
    // make the graph connected
    for i in 0..n - 1 {
        graph.add_edge(&i, &(i + 1)).unwrap();
        graph.add_edge(&(i + 1), &i).unwrap();
    }
    let edges: Vec<_> = graph.edges().copied().collect();
    for e in &edges {
        graph.set_edge_weight(e, random.next_double()).unwrap();
    }
    graph
}

mod precomputation {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let graph = DiGraph::<usize>::pseudo().weighted();
        let h = hierarchy(&graph);
        let routing = TransitNodeRoutingPrecomputation::new(&h)
            .with_transit_vertex_count(0)
            .unwrap()
            .compute_transit_node_routing()
            .unwrap();
        assert!(routing.transit_vertices().is_empty());
    }

    #[test]
    fn test_too_many_transit_vertices() {
        let graph = DiGraph::<usize>::pseudo().weighted();
        let h = hierarchy(&graph);
        assert!(matches!(
            TransitNodeRoutingPrecomputation::new(&h).with_transit_vertex_count(1),
            Err(GraphError::InvalidParameter { name: "transit_vertex_count", .. })
        ));
    }

    #[test]
    fn test_one_vertex() {
        let mut graph = DiGraph::<usize>::pseudo().weighted();
        graph.add_vertex(1);
        let h = hierarchy(&graph);
        let routing = TransitNodeRoutingPrecomputation::new(&h)
            .with_transit_vertex_count(1)
            .unwrap()
            .compute_transit_node_routing()
            .unwrap();
        let id = h.contraction_mapping()[&1];
        // transit vertices
        assert!(routing.transit_vertices().contains(&1));
        assert_eq!(routing.transit_vertex_ids(), [id]);
        // access vertices
        let forward = routing.access_vertices().forward(id);
        let backward = routing.access_vertices().backward(id);
        assert_eq!(forward.len(), 1);
        assert_eq!(forward[0].vertex, 1);
        assert_eq!(forward[0].path.vertex_list(), [1]);
        assert!(forward[0].path.edge_list().is_empty());
        assert_eq!(forward[0].path.weight(), 0.0);
        assert_eq!(backward.len(), 1);
        assert_eq!(backward[0].vertex, 1);
        assert_eq!(backward[0].path.vertex_list(), [1]);
        // locality filter
        assert!(!routing.locality_filter().is_local(&1, &1));
    }

    #[test]
    fn test_three_vertices() {
        let mut graph = DiGraph::<usize>::pseudo().weighted();
        graph.add_vertices([1, 2, 3]);
        let edge1 = graph.add_weighted_edge(&1, &2, 1.0).unwrap().unwrap();
        let edge2 = graph.add_weighted_edge(&2, &1, 1.0).unwrap().unwrap();
        // to ensure Voronoi diagram correctness
        let edge3 = graph.add_weighted_edge(&2, &3, 2.0).unwrap().unwrap();
        let edge4 = graph.add_weighted_edge(&3, &2, 2.0).unwrap().unwrap();
        let h = hierarchy(&graph);
        let routing = TransitNodeRoutingPrecomputation::new(&h)
            .with_transit_vertex_count(1)
            .unwrap()
            .compute_transit_node_routing()
            .unwrap();
        let m = h.contraction_mapping();
        let (cv1, cv2, cv3) = (m[&1], m[&2], m[&3]);
        // transit vertices
        assert!(routing.transit_vertices().contains(&2));
        // access vertices
        let av = routing.access_vertices();
        assert_eq!(av.forward(cv1).len(), 1);
        assert_eq!(av.forward(cv1)[0].vertex, 2);
        assert_eq!(av.forward(cv1)[0].path.vertex_list(), [1, 2]);
        assert_eq!(av.forward(cv1)[0].path.edge_list(), [edge1]);
        assert_eq!(av.forward(cv1)[0].path.weight(), 1.0);
        assert_eq!(av.backward(cv1).len(), 1);
        assert_eq!(av.backward(cv1)[0].vertex, 2);
        assert_eq!(av.backward(cv1)[0].path.vertex_list(), [2, 1]);
        assert_eq!(av.backward(cv1)[0].path.edge_list(), [edge2]);
        assert_eq!(av.forward(cv2).len(), 1);
        assert_eq!(av.backward(cv2).len(), 1);
        assert_eq!(av.forward(cv3).len(), 1);
        assert_eq!(av.forward(cv3)[0].vertex, 2);
        assert_eq!(av.forward(cv3)[0].path.vertex_list(), [3, 2]);
        assert_eq!(av.forward(cv3)[0].path.edge_list(), [edge4]);
        assert_eq!(av.forward(cv3)[0].path.weight(), 2.0);
        assert_eq!(av.backward(cv3).len(), 1);
        assert_eq!(av.backward(cv3)[0].path.vertex_list(), [2, 3]);
        assert_eq!(av.backward(cv3)[0].path.edge_list(), [edge3]);
        assert_eq!(av.backward(cv3)[0].path.weight(), 2.0);
        // locality filter
        let lf = routing.locality_filter();
        assert!(lf.is_local(&1, &1));
        assert!(!lf.is_local(&1, &2));
        assert!(lf.is_local(&1, &3));
        assert!(!lf.is_local(&2, &2));
        assert!(!lf.is_local(&2, &3));
        assert!(lf.is_local(&3, &3));
        // Voronoi diagram: the transit vertex is the only center
        let voronoi = routing.voronoi_diagram();
        for v in [cv1, cv2, cv3] {
            assert_eq!(voronoi.voronoi_cell_id(v), cv2 as i64);
        }
    }

    #[test]
    fn test_on_random_graphs() {
        let mut random = JavaRandom::new(SEED);
        for _ in 0..50 {
            let graph = random_graph(30, 5 * 30, &mut random);
            let h = ContractionHierarchyPrecomputation::with_random_supplier(&graph, 4, || {
                JavaRandom::new(SEED)
            })
            .compute_contraction_hierarchy()
            .unwrap();
            let routing = TransitNodeRoutingPrecomputation::new(&h)
                .compute_transit_node_routing()
                .unwrap();
            assert_correct_tnr(&graph, &routing);
        }
    }

    /// Checks the number and level of the transit vertices, the sets of the transit paths, the
    /// range of the Voronoi cell ids and the paths of the access vertices.
    fn assert_correct_tnr(graph: &DiGraph<usize>, routing: &TransitNodeRouting<'_, DiGraph<usize>>) {
        let n = graph.vertex_count();
        let h = routing.contraction_hierarchy();
        // transit vertices
        let transit = routing.transit_vertices();
        assert_eq!(transit.len(), (n as f64).sqrt() as usize);
        for v in transit {
            assert!(h.contraction_vertex(v).unwrap().contraction_level >= n - transit.len());
        }
        // many-to-many shortest paths
        assert_eq!(routing.transit_paths().sources(), transit);
        assert_eq!(routing.transit_paths().targets(), transit);
        // Voronoi diagram
        for v in 0..n {
            let cell = routing.voronoi_diagram().voronoi_cell_id(v);
            assert!((-1..n as i64).contains(&cell));
        }
        // access vertices
        let sp = BidirectionalDijkstraShortestPath::new(graph);
        for v in h.vertices() {
            for access in routing.access_vertices().forward(v.vertex_id) {
                assert_eq!(
                    sp.path(&v.vertex, &access.vertex).unwrap().as_ref(),
                    Some(&access.path)
                );
            }
            for access in routing.access_vertices().backward(v.vertex_id) {
                assert_eq!(
                    sp.path(&access.vertex, &v.vertex).unwrap().as_ref(),
                    Some(&access.path)
                );
            }
        }
    }
}

mod shortest_path {
    use super::*;

    #[test]
    fn test_one_vertex() {
        let mut graph = DiGraph::<usize>::pseudo().weighted();
        graph.add_vertex(1);
        let shortest_path = TransitNodeRoutingShortestPath::with_parallelism(&graph, 4);
        let path = shortest_path.path(&1, &1).unwrap().unwrap();
        assert_eq!(path.vertex_list(), [1]);
        assert!(path.edge_list().is_empty());
        assert_eq!(path.weight(), 0.0);
    }

    fn routing_for<'g>(
        graph: &'g DiGraph<usize>,
        h: &ContractionHierarchy<'g, DiGraph<usize>>,
        k: usize,
    ) -> TransitNodeRoutingShortestPath<'g, DiGraph<usize>> {
        let _ = graph;
        TransitNodeRoutingShortestPath::from_routing(
            TransitNodeRoutingPrecomputation::new(h)
                .with_transit_vertex_count(k)
                .unwrap()
                .compute_transit_node_routing()
                .unwrap(),
        )
    }

    #[test]
    fn test_two_vertices() {
        let mut graph = DiGraph::<usize>::pseudo().weighted();
        graph.add_vertices([1, 2]);
        let edge1 = graph.add_weighted_edge(&1, &2, 1.0).unwrap().unwrap();
        let _edge2 = graph.add_weighted_edge(&1, &2, 2.0).unwrap().unwrap();
        let edge3 = graph.add_weighted_edge(&2, &1, 1.0).unwrap().unwrap();
        let h = hierarchy(&graph);
        let shortest_path = routing_for(&graph, &h, 1);
        let p = shortest_path.path(&1, &2).unwrap().unwrap();
        assert_eq!((p.vertex_list().to_vec(), p.edge_list().to_vec(), p.weight()), (vec![1, 2], vec![edge1], 1.0));
        let p = shortest_path.path(&2, &1).unwrap().unwrap();
        assert_eq!((p.vertex_list().to_vec(), p.edge_list().to_vec(), p.weight()), (vec![2, 1], vec![edge3], 1.0));
    }

    #[test]
    fn test_three_vertices() {
        let mut graph = DiGraph::<usize>::pseudo().weighted();
        graph.add_vertices([1, 2, 3]);
        let edge1 = graph.add_weighted_edge(&1, &2, 1.0).unwrap().unwrap();
        let edge2 = graph.add_weighted_edge(&2, &3, 2.0).unwrap().unwrap();
        let edge3 = graph.add_weighted_edge(&3, &2, 1.0).unwrap().unwrap();
        let h = hierarchy(&graph);
        let sp = routing_for(&graph, &h, 1);
        let walk = |s, t| sp.path(&s, &t).unwrap().map(|p| (p.vertex_list().to_vec(), p.edge_list().to_vec(), p.weight()));
        assert_eq!(walk(1, 2), Some((vec![1, 2], vec![edge1], 1.0)));
        assert_eq!(walk(2, 1), None);
        assert_eq!(walk(2, 3), Some((vec![2, 3], vec![edge2], 2.0)));
        assert_eq!(walk(3, 2), Some((vec![3, 2], vec![edge3], 1.0)));
        assert_eq!(walk(1, 3), Some((vec![1, 2, 3], vec![edge1, edge2], 3.0)));
        assert_eq!(walk(3, 1), None);
        assert_eq!(sp.path_weight(&1, &3).unwrap(), Some(3.0));
        assert_eq!(sp.path_weight(&3, &1).unwrap(), None);
    }

    #[test]
    fn test_on_random_graphs() {
        let mut random = JavaRandom::new(SEED);
        for _ in 0..20 {
            let graph = random_graph(30, 5 * 30, &mut random);
            let expected = Dijkstra::new(&graph).single_source(&0).unwrap();
            let h = hierarchy(&graph);
            let routing = TransitNodeRoutingPrecomputation::new(&h)
                .compute_transit_node_routing()
                .unwrap();
            let actual = TransitNodeRoutingShortestPath::from_routing(routing)
                .single_source(&0)
                .unwrap();
            let vertices: HashSet<_> = graph.vertices().copied().collect();
            for sink in vertices {
                assert_eq!(expected.path(&sink), actual.path(&sink));
            }
        }
    }

    #[test]
    fn test_unknown_vertex() {
        let mut graph = DiGraph::<usize>::pseudo().weighted();
        graph.add_vertex(1);
        let shortest_path = TransitNodeRoutingShortestPath::with_parallelism(&graph, 2);
        assert!(matches!(
            shortest_path.path(&1, &2),
            Err(GraphError::VertexNotFound)
        ));
    }

    #[allow(dead_code)]
    fn _unused(_: DefaultEdge) {}
}

/// Compares the transit vertices, Voronoi cells, access vertices and locality filter with the ones
/// computed by JGraphT (`scripts/java-oracle/TransitNodeRoutingOracle.java`) on the same hierarchy.
mod java_golden {
    use super::*;
    use jgrapht::UGraph;
    use std::collections::BTreeSet;

    fn check<G: Graph<Vertex = usize> + Sync>(graph: &G, expected: &[&str], name: &str, tie_free: bool)
    where
        G::Edge: Send + Sync,
    {
        let h = ContractionHierarchyPrecomputation::with_random_supplier(graph, 3, || {
            JavaRandom::new(19)
        })
        .compute_contraction_hierarchy()
        .unwrap();
        let routing = TransitNodeRoutingPrecomputation::new(&h)
            .with_parallelism(3)
            .compute_transit_node_routing()
            .unwrap();
        let set = |it: Vec<usize>| {
            let sorted: BTreeSet<_> = it.into_iter().collect();
            let items: Vec<_> = sorted.iter().map(|v| v.to_string()).collect();
            format!("[{}]", items.join(", "))
        };
        let n = graph.vertex_count();
        let transit: Vec<usize> = routing.transit_vertices().iter().copied().collect();
        assert_eq!(format!("T {}", set(transit)), expected[0], "{name}: transit");
        for v in 0..n {
            let id = h.contraction_mapping()[&v];
            let access = |list: &[jgrapht::alg::shortestpath::AccessVertex<usize, _>]| {
                set(list.iter().map(|a| a.vertex).collect())
            };
            let got = format!(
                "V {v} {} F{} B{}",
                routing.voronoi_diagram().voronoi_cell_id(id),
                access(routing.access_vertices().forward(id)),
                access(routing.access_vertices().backward(id)),
            );
            if tie_free {
                assert_eq!(got, expected[1 + v], "{name}: vertex {v}");
            } else {
                // equidistant cell centers are tried in `HashSet` order by Java, so only the
                // access vertices are comparable
                let strip = |s: &str| s.splitn(4, ' ').nth(3).unwrap().to_string();
                assert_eq!(strip(&got), strip(expected[1 + v]), "{name}: vertex {v}");
            }
        }
        if !tie_free {
            return;
        }
        let mut filter = String::from("LF ");
        for s in 0..n {
            for t in 0..n {
                filter.push(if routing.locality_filter().is_local(&s, &t) { '1' } else { '0' });
            }
        }
        assert_eq!(filter, expected[1 + n], "{name}: locality filter");
    }

    #[test]
    fn routing_matches_jgrapht() {
        let graphs = include_str!("../fixtures/ch_graphs.txt");
        let golden = include_str!("../fixtures/tnr_golden.txt");
        let mut blocks: Vec<(&str, Vec<&str>)> = Vec::new();
        for line in golden.lines() {
            if let Some(name) = line.strip_prefix("case ") {
                blocks.push((name, Vec::new()));
            } else {
                blocks.last_mut().unwrap().1.push(line);
            }
        }
        assert_eq!(blocks.len(), 4);
        let mut lines = graphs.lines();
        while let Some(header) = lines.next() {
            let h: Vec<_> = header.split(' ').collect();
            let (n, m): (usize, usize) = (h[3].parse().unwrap(), h[4].parse().unwrap());
            let edges: Vec<(usize, usize, f64)> = (0..m)
                .map(|_| {
                    let t: Vec<_> = lines.next().unwrap().split(' ').collect();
                    (t[0].parse().unwrap(), t[1].parse().unwrap(), t[2].parse().unwrap())
                })
                .collect();
            let (_, expected) = blocks.iter().find(|(name, _)| *name == h[1]).unwrap();
            if h[2] == "d" {
                let mut g = DiGraph::<usize>::pseudo().weighted();
                g.add_vertices(0..n);
                for &(s, t, w) in &edges {
                    g.add_weighted_edge(&s, &t, w).unwrap();
                }
                check(&g, expected, h[1], h[1] != "und_b");
            } else {
                let mut g = UGraph::<usize>::pseudo().weighted();
                g.add_vertices(0..n);
                for &(s, t, w) in &edges {
                    g.add_weighted_edge(&s, &t, w).unwrap();
                }
                check(&g, expected, h[1], h[1] != "und_b");
            }
        }
    }
}
