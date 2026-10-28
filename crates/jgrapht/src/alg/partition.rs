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

//! Bipartiteness.

use super::interfaces::{Partitioning, PartitioningAlgorithm};
use super::util::Indexed;
use crate::graph::IncidenceGraph;
use crate::graph::{Graph, GraphExt, props};
use indexmap::IndexSet;
use std::borrow::Borrow;
use std::collections::HashSet;
use std::collections::VecDeque;

/// 2-colouring of the vertex indices (direction ignored); `None` if an odd cycle exists.
pub(crate) fn two_color<G: Graph>(ix: &Indexed<'_, G>) -> Option<Vec<bool>> {
    let n = ix.n();
    let mut side: Vec<Option<bool>> = vec![None; n];
    for r in 0..n {
        if side[r].is_some() {
            continue;
        }
        side[r] = Some(false);
        let mut q = VecDeque::from([r]);
        while let Some(v) = q.pop_front() {
            for &(u, _) in ix.out[v].iter().chain(&ix.inn[v]) {
                match side[u] {
                    None => {
                        side[u] = Some(!side[v].unwrap());
                        q.push_back(u);
                    }
                    Some(s) if s == side[v].unwrap() => return None,
                    _ => {}
                }
            }
        }
    }
    Some(side.into_iter().map(Option::unwrap).collect())
}

/// Splits the vertices into two independent sets, or `None` if the graph is not bipartite.
pub fn bipartite_partition<G: Graph>(g: &G) -> Option<(Vec<G::Vertex>, Vec<G::Vertex>)> {
    let ix = Indexed::new(g);
    let side = two_color(&ix)?;
    let pick = |s: bool| {
        (0..ix.n())
            .filter(|&v| side[v] == s)
            .map(|v| ix.vertex(v))
            .collect()
    };
    Some((pick(false), pick(true)))
}

/// Returns whether bipartite.
pub fn is_bipartite<G: Graph>(g: &G) -> bool {
    two_color(&Indexed::new(g)).is_some()
}

/// Algorithm for computing bipartite partitions thus checking whether a graph is bipartite or not.
///
/// The algorithm runs in linear time in the number of vertices and edges.
///
/// # Rust notes
///
/// Bipartite partitioning (`BipartitePartitioning`): breadth-first 2-colouring ignoring edge
/// directions. The result is cached.
pub struct BipartitePartitioning<'g, G: Graph> {
    graph: &'g G,
    computed: bool,
    cached: Option<Partitioning<G::Vertex>>,
}

impl<'g, G: IncidenceGraph> BipartitePartitioning<'g, G> {
    /// Constructs a new bipartite partitioning.
    pub fn new(graph: &'g G) -> Self {
        BipartitePartitioning {
            graph,
            computed: false,
            cached: None,
        }
    }

    /// Test whether the input graph is bipartite.
    pub fn is_bipartite(&mut self) -> bool {
        if props::is_empty(self.graph) {
            return true;
        }
        // a bipartite graph has at most n^2/4 edges
        let (n, m) = (self.graph.vertex_count(), self.graph.edge_count());
        if let (Some(m4), Some(nn)) = (m.checked_mul(4), n.checked_mul(n)) {
            if m4 > nn {
                return false;
            }
        }
        self.partitioning().is_some()
    }
}

impl<G: IncidenceGraph> PartitioningAlgorithm<G> for BipartitePartitioning<'_, G> {
    /// Computes a vertex partitioning.
    fn partitioning(&mut self) -> Option<&Partitioning<G::Vertex>> {
        if self.computed {
            return self.cached.as_ref();
        }
        let g = self.graph;
        let mut unknown: IndexSet<G::Vertex> = g.vertices_cloned().collect();
        let mut odd: HashSet<G::Vertex> = HashSet::new();
        let mut queue: VecDeque<G::Vertex> = VecDeque::new();
        while !unknown.is_empty() {
            if queue.is_empty() {
                queue.push_back(unknown.first().unwrap().clone());
            }
            let v = queue.pop_front().unwrap();
            unknown.shift_remove(&v);
            for e in g.edges_of(&v) {
                let n = g.opposite_vertex(e, &v).unwrap().clone();
                if unknown.contains(&n) {
                    queue.push_back(n.clone());
                    if !odd.contains(&v) {
                        odd.insert(n);
                    }
                } else if odd.contains(&v) == odd.contains(&n) {
                    self.computed = true;
                    self.cached = None;
                    return None;
                }
            }
        }
        let even: IndexSet<G::Vertex> = g
            .vertices()
            .filter(|v| {
                let v = &(*v).borrow();
                !odd.contains(*v)
            })
            .map(|x| {
                let x = x.borrow();
                x.clone()
            })
            .collect();
        let odd: IndexSet<G::Vertex> = g
            .vertices()
            .filter(|v| {
                let v = &(*v).borrow();
                odd.contains(*v)
            })
            .map(|x| {
                let x = x.borrow();
                x.clone()
            })
            .collect();
        self.computed = true;
        self.cached = Some(Partitioning::new(vec![even, odd]));
        self.cached.as_ref()
    }

    /// Check if the given vertex partitioning is valid.
    fn is_valid_partitioning(&self, partitioning: &Partitioning<G::Vertex>) -> bool {
        if partitioning.number_partitions() != 2 {
            return false;
        }
        let g = self.graph;
        let (first, second) = (partitioning.partition(0), partitioning.partition(1));
        if g.vertex_count() != first.len() + second.len() {
            return false;
        }
        for v in g.vertices() {
            let v = v.borrow();
            let other = if first.contains(v) {
                second
            } else if second.contains(v) {
                first
            } else {
                return false;
            };
            for e in g.edges_of(v) {
                if !other.contains(g.opposite_vertex(e, v).unwrap()) {
                    return false;
                }
            }
        }
        true
    }
}
