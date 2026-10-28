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

//! The sparse graphs hold only their endpoint arrays and compressed rows: `vertices()` and
//! `edges()` yield the integers themselves, so no identity array is kept. The test builds a graph
//! under a counting allocator and checks the retained heap against the size of those arrays.

use jgrapht::Graph;
use jgrapht::graph::sparse::{FullIncoming, NoIncoming, SparseIntDirectedGraph};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};

struct Counting;
static LIVE: AtomicIsize = AtomicIsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE.fetch_add(
            new_size as isize - layout.size() as isize,
            Ordering::Relaxed,
        );
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const N: usize = 10_000;
const M: usize = 200_000;
const WORD: usize = std::mem::size_of::<usize>();

fn edges() -> Vec<(usize, usize)> {
    (0..M).map(|i| (i % N, (i * 7 + 3) % N)).collect()
}

/// Bytes still allocated after `build` returns its graph, with the graph alive.
fn retained<G>(edges: &[(usize, usize)], build: impl FnOnce(&[(usize, usize)]) -> G) -> (usize, G) {
    let before = LIVE.load(Ordering::Relaxed);
    let graph = build(edges);
    let after = LIVE.load(Ordering::Relaxed);
    ((after - before) as usize, graph)
}

/// One test only: the counter is global, so concurrent tests would measure each other.
#[test]
fn sparse_graph_retains_only_endpoints_and_rows() {
    let edges = edges();
    let (bytes, g) = retained(&edges, |e| {
        SparseIntDirectedGraph::<NoIncoming>::from_edges(N, e).unwrap()
    });
    // source + target (2m words) + outgoing rows: m edge ids and n + 1 row offsets
    let stored = (3 * M + N + 1) * WORD;
    assert!(
        bytes <= stored + stored / 20,
        "{bytes} bytes retained, the arrays need {stored}"
    );
    assert_eq!(g.vertices().count(), N);
    assert_eq!(g.edges().last(), Some(M - 1));
    drop(g);

    // the incoming index adds one more set of rows
    let (bytes, g) = retained(&edges, |e| {
        SparseIntDirectedGraph::<FullIncoming>::from_edges(N, e).unwrap()
    });
    let stored = (4 * M + 2 * (N + 1)) * WORD;
    assert!(
        bytes <= stored + stored / 20,
        "{bytes} bytes retained, the arrays need {stored}"
    );
    assert_eq!(g.edge_count(), M);
}
