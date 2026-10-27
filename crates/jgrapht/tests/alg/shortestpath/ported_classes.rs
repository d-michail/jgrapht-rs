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

use super::*;
use dashu_int::UBig;
use std::collections::HashMap;

#[test]
fn yen_iterator_and_class_are_stably_sorted() {
    let graph = weighted_di(&[
        (1, 2, 1.0),
        (2, 4, 2.0),
        (1, 3, 2.0),
        (3, 4, 1.0),
        (1, 4, 10.0),
    ]);
    let paths = YenKShortestPath::new(&graph).get_paths(&1, &4, 10).unwrap();
    assert_eq!(
        paths.iter().map(|path| path.weight).collect::<Vec<_>>(),
        [3.0, 3.0, 10.0]
    );
    let iterator = YenShortestPathIterator::new(&graph, &1, &4).unwrap();
    assert_eq!(
        iterator.map(|path| path.weight).collect::<Vec<_>>(),
        [3.0, 3.0, 10.0]
    );
}

#[test]
fn dijkstra_many_to_many_handles_reverse_search() {
    let graph = weighted_di(&[(0, 2, 1.0), (1, 2, 2.0), (2, 3, 4.0)]);
    let paths = DijkstraManyToManyShortestPaths::new(&graph)
        .get_many_to_many_paths([0, 1], [3])
        .unwrap();
    assert_eq!(paths.weight(&0, &3), 5.0);
    assert_eq!(paths.weight(&1, &3), 6.0);
    assert_eq!(paths.path(&1, &3).unwrap().vertices, [1, 2, 3]);
}

#[test]
fn multi_objective_algorithms_return_the_pareto_front() {
    let mut graph = DiGraph::<u32>::multi().weighted();
    for vertex in 1..=3 {
        graph.add_vertex(vertex);
    }
    let direct = graph.add_edge(&1, &2).unwrap().unwrap();
    let first = graph.add_edge(&1, &3).unwrap().unwrap();
    let second = graph.add_edge(&3, &2).unwrap().unwrap();
    let values = HashMap::from([
        (direct, vec![5.0, 2.0]),
        (first, vec![2.0, 8.0]),
        (second, vec![1.0, 4.0]),
    ]);
    let martin = MartinShortestPath::new(&graph, |edge| values[edge].clone())
        .get_paths(&1)
        .unwrap();
    let correcting =
        LabelCorrectingMultiObjectiveShortestPath::new(&graph, |edge| values[edge].clone())
            .get_paths(&1)
            .unwrap();
    assert_eq!(martin.paths(&2).len(), 2);
    assert_eq!(correcting.paths(&2).len(), 2);
}

#[test]
fn dag_counter_uses_arbitrary_precision() {
    let mut graph = DiGraph::<u32>::simple();
    for vertex in 0..6 {
        graph.add_vertex(vertex);
    }
    for left in [1, 2] {
        graph.add_edge(&0, &left).unwrap();
        for right in [3, 4] {
            graph.add_edge(&left, &right).unwrap();
        }
    }
    graph.add_edge(&3, &5).unwrap();
    graph.add_edge(&4, &5).unwrap();
    assert_eq!(
        DagAllPathsCounter::count_all_paths(&graph, &0, &5).unwrap(),
        UBig::from(4u8)
    );
}

#[test]
fn integer_dijkstra_supports_dense_and_remapped_ids() {
    let graph = weighted_di(&[(100, 1, 2.0), (100, 2, 3.0), (1, 3, 5.0), (3, 4, 5.0)]);
    let paths = IntVertexDijkstraShortestPath::new(&graph)
        .single_source(&100)
        .unwrap();
    assert_eq!(paths.weight(&4), 12.0);
}
