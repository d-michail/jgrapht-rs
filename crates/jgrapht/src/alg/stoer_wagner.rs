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

//! `StoerWagnerMinimumCut`: the Stoer–Wagner minimum cut (M. Stoer and F. Wagner, "A Simple
//! Min-Cut Algorithm", JACM 44(4), 1997).
//!
//! Same technique as JGraphT: a working simple weighted graph whose vertices are the merged vertex
//! sets, an extract-max queue that is a binary heap with the exact sift rules of
//! `java.util.PriorityQueue` (so ties are resolved as in Java), and `PriorityQueue.remove(Object)`
//! for the key updates, which scans the heap array. Edge lookup in the working graph scans the
//! incidence list of the first endpoint, like JGraphT's default specifics.

use crate::error::{GraphError, Result};
use crate::graph::GraphExt;
use crate::graph::UndirectedGraph;
use indexmap::{IndexMap, IndexSet};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashMap;

/// Working vertex id (index into the member list).
type Id = usize;

/// The simple weighted working graph: vertices in insertion order, per vertex the incident edges
/// in insertion order.
struct Working {
    vertices: IndexSet<Id>,
    /// `(neighbour, edge)` per vertex id, in insertion order.
    incident: Vec<Vec<(Id, usize)>>,
    weight: Vec<f64>,
    /// Edge endpoints, `None` once removed.
    ends: Vec<Option<(Id, Id)>>,
}

impl Working {
    fn add_vertex(&mut self) -> Id {
        let id = self.incident.len();
        self.incident.push(Vec::new());
        self.vertices.insert(id);
        id
    }

    fn contains(&self, v: Id) -> bool {
        self.vertices.contains(&v)
    }

    /// `getEdge(u, v)`: scans the incidence list of `u`.
    fn get_edge(&self, u: Id, v: Id) -> Option<usize> {
        if self.contains(u) && self.contains(v) {
            self.incident[u]
                .iter()
                .find(|&&(w, _)| w == v)
                .map(|&(_, e)| e)
        } else {
            None
        }
    }

    fn add_edge(&mut self, u: Id, v: Id) -> usize {
        let e = self.weight.len();
        self.weight.push(0.0);
        self.ends.push(Some((u, v)));
        self.incident[u].push((v, e));
        self.incident[v].push((u, e));
        e
    }

    fn remove_vertex(&mut self, v: Id) {
        for (w, e) in std::mem::take(&mut self.incident[v]) {
            self.ends[e] = None;
            if let Some(p) = self.incident[w].iter().position(|&(_, f)| f == e) {
                self.incident[w].remove(p);
            }
        }
        self.vertices.shift_remove(&v);
    }
}

/// `StoerWagnerMinimumCut.VertexAndWeight`.
#[derive(Clone, Copy)]
struct VertexAndWeight {
    vertex: Id,
    weight: f64,
    active: bool,
}

/// Extract-max order on top of an extract-min queue.
fn compare(a: &VertexAndWeight, b: &VertexAndWeight) -> Ordering {
    match (a.active, b.active) {
        (true, true) => b.weight.total_cmp(&a.weight),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => Ordering::Equal,
    }
}

/// `java.util.PriorityQueue` (binary heap, same sift rules).
struct PriorityQueue {
    queue: Vec<VertexAndWeight>,
}

impl PriorityQueue {
    fn new() -> Self {
        Self { queue: Vec::new() }
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn add(&mut self, x: VertexAndWeight) {
        self.queue.push(x);
        let k = self.queue.len() - 1;
        self.sift_up(k, x);
    }

    fn poll(&mut self) -> Option<VertexAndWeight> {
        if self.queue.is_empty() {
            return None;
        }
        let result = self.queue[0];
        let x = self.queue.pop().expect("non-empty");
        if !self.queue.is_empty() {
            self.sift_down(0, x);
        }
        Some(result)
    }

    /// `remove(Object)`: linear scan for the element, then `removeAt`.
    fn remove(&mut self, vertex: Id) -> Option<VertexAndWeight> {
        let i = self.queue.iter().position(|q| q.vertex == vertex)?;
        let removed = self.queue[i];
        let last = self.queue.pop().expect("non-empty");
        if i < self.queue.len() {
            self.sift_down(i, last);
            if self.queue[i].vertex == last.vertex {
                self.sift_up(i, last);
            }
        }
        Some(removed)
    }

    fn sift_up(&mut self, mut k: usize, x: VertexAndWeight) {
        while k > 0 {
            let parent = (k - 1) >> 1;
            let e = self.queue[parent];
            if compare(&x, &e) != Ordering::Less {
                break;
            }
            self.queue[k] = e;
            k = parent;
        }
        self.queue[k] = x;
    }

    fn sift_down(&mut self, mut k: usize, x: VertexAndWeight) {
        let n = self.queue.len();
        let half = n >> 1;
        while k < half {
            let mut child = 2 * k + 1;
            let mut c = self.queue[child];
            let right = child + 1;
            if right < n && compare(&c, &self.queue[right]) == Ordering::Greater {
                child = right;
                c = self.queue[child];
            }
            if compare(&x, &c) != Ordering::Greater {
                break;
            }
            self.queue[k] = c;
            k = child;
        }
        self.queue[k] = x;
    }
}

/// Implements the [Stoer and Wagner minimum cut algorithm](http://dl.acm.org/citation.cfm?id=263872). Deterministically computes the minimum cut in $O(|V||E| + |V| \log |V|)$ time.
/// This implementation uses Java's PriorityQueue and requires $O(|V||E| \log |E|)$ time. M. Stoer
/// and F. Wagner, "A Simple Min-Cut Algorithm", Journal of the ACM, volume 44, number 4. pp 585-591,
/// 1997.
///
/// # Rust notes
///
/// The Stoer–Wagner minimum cut of an undirected graph.
///
/// The cut is computed on construction; read it with [`min_cut_weight`](Self::min_cut_weight) and
/// [`min_cut`](Self::min_cut). Parallel edges are summed (they are all in the cut or none is).
pub struct StoerWagnerMinimumCut<V> {
    working: Working,
    members: Vec<IndexSet<V>>,
    best_cut_weight: f64,
    best_cut: Option<Id>,
}

impl<V: Clone + Eq + std::hash::Hash> StoerWagnerMinimumCut<V> {
    /// Computes the minimum cut of `graph`.
    ///
    /// Fails if the graph is not undirected, has fewer than 2 vertices, has an edge of negative
    /// weight, or has a self-loop.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if the graph lacks a property the algorithm requires; an edge weight is negative, NaN or otherwise unusable; errors of the underlying graph operations are propagated.
    ///
    /// # Panics
    ///
    /// Panics only if the graph breaks the integrity contract of [`Graph`](crate::Graph) (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
    pub fn new<G: UndirectedGraph<Vertex = V>>(graph: &G) -> Result<Self> {
        if graph.vertex_count() < 2 {
            return Err(GraphError::InvalidGraph(
                crate::GraphProperty::AtLeastTwoVertices,
            ));
        }

        let mut this = Self {
            working: Working {
                vertices: IndexSet::new(),
                incident: Vec::new(),
                weight: Vec::new(),
                ends: Vec::new(),
            },
            members: Vec::new(),
            best_cut_weight: f64::INFINITY,
            best_cut: None,
        };

        // each vertex wrapped in a set
        let vertex_snapshot: Vec<V> = graph.vertices_cloned().collect();
        let mut vertex_map: HashMap<&V, Id> = HashMap::new();
        for v in &vertex_snapshot {
            let id = this.working.add_vertex();
            this.members.push(IndexSet::from([v.clone()]));
            vertex_map.insert(v, id);
        }
        for e in graph.edges() {
            let e = e.borrow();
            let w = graph.weight_of(e);
            if w < 0.0 {
                return Err(GraphError::InvalidWeight(crate::WeightProblem::Negative));
            }
            let (s, t) = graph
                .edge_endpoints(e)
                .ok_or(crate::GraphError::EdgeNotFound)?;
            let (s_new, t_new) = (vertex_map[s], vertex_map[t]);
            // For multigraphs, we sum the edge weights (either all are contained in a cut, or
            // none).
            match this.working.get_edge(s_new, t_new) {
                None => {
                    if s_new == t_new {
                        return Err(GraphError::InvalidGraph(crate::GraphProperty::NoSelfLoops));
                    }
                    let e_new = this.working.add_edge(s_new, t_new);
                    this.working.weight[e_new] = w;
                }
                Some(e_new) => this.working.weight[e_new] += w,
            }
        }

        // arbitrary vertex used to seed the algorithm
        let a = *this.working.vertices.iter().next().expect("two vertices");
        while this.working.vertices.len() > 1 {
            this.minimum_cut_phase(a);
        }
        Ok(this)
    }

    /// `minimumCutPhase`.
    fn minimum_cut_phase(&mut self, a: Id) {
        // The last and before last vertices added to A.
        let (mut last, mut before_last) = (a, None);

        // queue contains vertices not in A ordered by max weight of edges to A
        let mut queue = PriorityQueue::new();
        // vertices that are in the queue
        let mut dmap: IndexMap<Id, ()> = IndexMap::new();

        for &v in &self.working.vertices {
            if v == a {
                continue;
            }
            let e = self.working.get_edge(v, a);
            let w = e.map_or(0.0, |e| self.working.weight[e]);
            queue.add(VertexAndWeight {
                vertex: v,
                weight: w,
                active: e.is_some(),
            });
            dmap.insert(v, ());
        }

        // iteratively update the queue to get the required vertex ordering
        while !queue.is_empty() {
            let v = queue.poll().expect("non-empty").vertex;
            dmap.swap_remove(&v);

            before_last = Some(last);
            last = v;

            for &(vc, e) in &self.working.incident[v] {
                if dmap.contains_key(&vc) {
                    let mut vcandw = queue.remove(vc).expect("queued");
                    vcandw.active = true;
                    vcandw.weight += self.working.weight[e];
                    queue.add(vcandw);
                }
            }
        }

        // update the best cut
        let w = self.vertex_weight_id(last);
        if w < self.best_cut_weight {
            self.best_cut_weight = w;
            self.best_cut = Some(last);
        }

        // merge the last added vertices
        self.merge_vertices(before_last.expect("at least two vertices"), last);
    }

    /// Merges vertex `t` into vertex `s`, summing the weights as required.
    fn merge_vertices(&mut self, s: Id, t: Id) {
        // construct the new combined vertex
        let mut set: IndexSet<V> = self.members[s].clone();
        set.extend(self.members[t].iter().cloned());
        let new = self.working.add_vertex();
        self.members.push(set);

        // add edges and weights to the combined vertex
        let others: Vec<Id> = self.working.vertices.iter().copied().collect();
        for v in others {
            if s != v && t != v {
                let mut neww = 0.0;
                let etv = self.working.get_edge(t, v);
                let esv = self.working.get_edge(s, v);
                if let Some(e) = etv {
                    neww += self.working.weight[e];
                }
                if let Some(e) = esv {
                    neww += self.working.weight[e];
                }
                if etv.is_some() || esv.is_some() {
                    let e = self.working.add_edge(new, v);
                    self.working.weight[e] = neww;
                }
            }
        }

        // remove original vertices
        self.working.remove_vertex(t);
        self.working.remove_vertex(s);
    }

    fn vertex_weight_id(&self, v: Id) -> f64 {
        self.working.incident[v]
            .iter()
            .fold(0.0, |acc, &(_, e)| acc + self.working.weight[e])
    }

    /// The weight of the minimum cut.
    pub fn min_cut_weight(&self) -> f64 {
        self.best_cut_weight
    }

    /// The vertices on one side of the minimum cut.
    ///
    /// # Panics
    ///
    /// Panics only if an internal invariant is violated, which indicates a bug in this library.
    pub fn min_cut(&self) -> &IndexSet<V> {
        &self.members[self.best_cut.expect("cut computed")]
    }
}
