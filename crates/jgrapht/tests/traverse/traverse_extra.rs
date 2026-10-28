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

use jgrapht::traverse::*;
use jgrapht::*;
use rand::{SeedableRng, rngs::StdRng};

fn dg(edges: &[(u32, u32)]) -> DiGraph<u32> {
    let mut g = DiGraph::<u32>::simple();
    for &(a, b) in edges {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    g
}

#[test]
fn bfs_dfs_orders() {
    let g = dg(&[(1, 2), (1, 3), (2, 4), (3, 4), (4, 5)]);
    let mut bfs = Bfs::new(&g, &1).unwrap();
    let order: Vec<_> = bfs.by_ref().collect();
    assert_eq!(order, [1, 2, 3, 4, 5]);
    assert_eq!(bfs.depth(&5), Some(3));
    let dfs: Vec<_> = Dfs::new(&g, &1).unwrap().collect();
    assert_eq!(dfs, [1, 2, 4, 5, 3]);
    let post: Vec<_> = DfsPostOrder::new(&g, &1).unwrap().collect();
    assert_eq!(post, [5, 4, 2, 3, 1]);
}

#[test]
fn all_components_and_direction() {
    let g = dg(&[(1, 2), (3, 4)]);
    assert_eq!(Bfs::new(&g, &2).unwrap().count(), 1); // directed: 2 has no out-edges
    assert_eq!(Bfs::all(&g).count(), 4);
    assert_eq!(Dfs::all(&g).count(), 4);
    assert_eq!(DfsPostOrder::all(&g).count(), 4);
}

#[test]
fn topo() {
    let g = dg(&[(1, 2), (1, 3), (2, 4), (3, 4)]);
    let t = topological_sort(&g).unwrap();
    let pos = |x: u32| t.iter().position(|v| *v == x).unwrap();
    assert!(pos(1) < pos(2) && pos(2) < pos(4) && pos(3) < pos(4));
    assert!(topological_sort(&dg(&[(1, 2), (2, 1)])).is_none());
}

#[test]
fn closest_first_and_radius() {
    let mut g = UGraph::<u32>::simple().weighted();
    for (a, b, w) in [(1, 2, 1.0), (2, 3, 1.0), (1, 3, 5.0), (3, 4, 1.0)] {
        g.add_edge_with_vertices(a, b).unwrap();
        let e = *g.edge_between(&a, &b).unwrap();
        g.set_edge_weight(&e, w).unwrap();
    }
    let r: Vec<_> = ClosestFirst::new(&g, &1)
        .unwrap()
        .map(|item| item.map(|(v, d)| (*v, d)))
        .collect::<jgrapht::Result<_>>()
        .unwrap();
    assert_eq!(r, [(1, 0.0), (2, 1.0), (3, 2.0), (4, 3.0)]);
    assert_eq!(ClosestFirst::with_radius(&g, &1, 2.0).unwrap().count(), 3);
}

#[test]
fn random_walk() {
    let g = dg(&[(1, 2), (2, 3)]);
    let w: Vec<_> = RandomWalk::new(&g, &1, 10, false, StdRng::seed_from_u64(1))
        .unwrap()
        .map(|v| *v.unwrap())
        .collect();
    assert_eq!(w, [1, 2, 3]);
}

#[test]
fn orderings() {
    // path 1-2-3 plus pendant 4 on 3
    let mut g = UGraph::<u32>::simple();
    for (a, b) in [(1, 2), (2, 3), (3, 4)] {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    assert_eq!(LexBfs::new(&g).count(), 4);
    assert_eq!(MaximumCardinality::new(&g).count(), 4);
    let mut d = DegeneracyOrdering::new(&g);
    assert_eq!(d.by_ref().count(), 4);
    assert_eq!(d.degeneracy(), 1);
    // triangle has degeneracy 2
    let mut t = UGraph::<u32>::simple();
    for (a, b) in [(1, 2), (2, 3), (1, 3)] {
        t.add_edge_with_vertices(a, b).unwrap();
    }
    let mut d = DegeneracyOrdering::new(&t);
    d.by_ref().for_each(drop);
    assert_eq!(d.degeneracy(), 2);
    // LexBFS on a path from the first vertex visits it in path order
    let order: Vec<_> = LexBfs::new(&g).collect();
    assert_eq!(order, [1, 2, 3, 4]);
}
