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

use crate::graph::OutgoingGraph;
use super::DEFAULT_EPSILON;
use super::base::{FlowNet, flow_algorithm_api};
use crate::error::Result;
use std::collections::VecDeque;

const FREE_NODE_TIMESTAMP: u64 = 0;
const INITIAL_TIMESTAMP: u64 = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TreeStatus {
    Source,
    Sink,
    Free,
}

#[derive(Clone)]
struct VertexData {
    timestamp: u64,
    distance: i64,
    active: bool,
    parent_edge: Option<usize>,
    tree_status: TreeStatus,
}

struct AdoptionParent {
    edge: usize,
    distance: i64,
}

/// This is an implementation of the
/// [Boykov-Kolmogorov maximum flow algorithm](https://en.wikipedia.org/wiki/Graph_cuts_in_computer_vision#Implementation_(exact)). This algorithm is a special-purpose approach to
/// solving computer vision related maximum flow problems. The algorithm was initially described in:
/// *Y. Boykov and V. Kolmogorov, "An experimental comparison of min-cut/max-flow algorithms for
/// energy minimization in vision," in IEEE Transactions on Pattern Analysis and Machine
/// Intelligence, vol. 26, no. 9, pp. 1124-1137, Sept. 2004, doi: 10.1109/TPAMI.2004.60.*. An
/// extended description is given in: *Vladimir Kolmogorov. 2004. Graph based algorithms for scene
/// reconstruction from two or more views. Ph.D. Dissertation. Cornell University, USA. Advisor(s)
/// Ramin Zabih. Order Number: AAI3114475.*.
///
/// This implementation uses 2 heuristics described in Vladimir Kolmogorov's original PhD thesis:
///
/// - Timestamp heuristic.
/// - Distance heuristic;
///
/// The worse-case running time of this algorithm on a network $G = (V, E)$ with a capacity function
/// $c: E \rightArrow R^{+}$ is $\mathcal{O}(E\times f)$, where $f$ is the maximum flow value. The
/// reason for this is that the algorithm doesn't necessarily augments shortest $s-t$ paths in a
/// residual network. That's why the argument about the running time complexity is the same as with
/// the Ford-Fulkerson algorithm.
///
/// This algorithm doesn't have the best performance on all types of networks. It's recommended to
/// check if this algorithm gives substantial performance improvement before using it in a particular
/// application. A good general-purpose alternative which works fast in all scenarios is the
/// `PushRelabelMFImpl`.
///
/// This algorithm works with both directed and undirected networks. The algorithm doesn't have
/// internal synchronization, thus any concurrent network modification has undefined behaviour.
///
/// # Rust notes
///
/// `BoykovKolmogorovMFImpl`: two search trees (from the source and from the sink) that are grown,
/// augmented along and repaired, `O(V² E |C|)` in theory and fast on vision-style graphs.
pub struct BoykovKolmogorovMFImpl<'g, G: OutgoingGraph> {
    net: FlowNet<'g, G>,
    data: Vec<VertexData>,
    current_timestamp: u64,
    active_vertices: VecDeque<usize>,
    orphans: Vec<usize>,
    child_orphans: VecDeque<usize>,
}

impl<'g, G: OutgoingGraph> BoykovKolmogorovMFImpl<'g, G> {
    /// Creates a new algorithm instance with the specified `network`. The created algorithm
    /// uses default epsilon.
    pub fn new(network: &'g G) -> Result<Self> {
        Self::with_epsilon(network, DEFAULT_EPSILON)
    }

    pub fn with_epsilon(network: &'g G, epsilon: f64) -> Result<Self> {
        Ok(BoykovKolmogorovMFImpl {
            net: FlowNet::new(network, epsilon)?,
            data: Vec::new(),
            current_timestamp: 0,
            active_vertices: VecDeque::new(),
            orphans: Vec::new(),
            child_orphans: VecDeque::new(),
        })
    }

    fn calculate(&mut self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        let (s, t) = self.net.resolve(
            source,
            sink,
            [
                "invalid source (null or not from this network)",
                "invalid sink (null or not from this network)",
                "source is equal to sink",
            ],
        )?;
        self.net.init(s, t)?;
        self.data = vec![
            VertexData {
                timestamp: 0,
                distance: 0,
                active: false,
                parent_edge: None,
                tree_status: TreeStatus::Free,
            };
            self.net.vs.len()
        ];
        self.current_timestamp = INITIAL_TIMESTAMP;
        self.augment_short_paths(s, t);
        self.data[s].tree_status = TreeStatus::Source;
        self.data[t].tree_status = TreeStatus::Sink;
        self.make_active(s);
        self.make_active(t);
        while let Some(bounding_edge) = self.grow() {
            self.augment(bounding_edge);
            self.next_iteration();
            self.adopt();
        }
        Ok(self.net.max_flow_value)
    }

    /// Saturates the paths `source -> sink` and `source -> x -> sink` first.
    fn augment_short_paths(&mut self, source: usize, sink: usize) {
        for i in 0..self.net.outgoing[source].len() {
            let source_edge = self.net.outgoing[source][i];
            let medium = self.net.arcs[source_edge].target;
            if medium == sink {
                let flow = self.net.residual_capacity(source_edge);
                self.net.push_flow_through(source_edge, flow);
                self.net.max_flow_value += flow;
            } else {
                for j in 0..self.net.outgoing[medium].len() {
                    let sink_edge = self.net.outgoing[medium][j];
                    if self.net.arcs[sink_edge].target == sink {
                        let flow = self
                            .net
                            .residual_capacity(source_edge)
                            .min(self.net.residual_capacity(sink_edge));
                        self.net.push_flow_through(source_edge, flow);
                        self.net.push_flow_through(sink_edge, flow);
                        self.net.max_flow_value += flow;
                    }
                    if !self.net.has_capacity(source_edge) {
                        break;
                    }
                }
            }
        }
    }

    /// Grows the trees until they touch; returns the bounding edge, or `None` when they cannot.
    fn grow(&mut self) -> Option<usize> {
        while let Some(active_vertex) = self.next_active_vertex() {
            if self.data[active_vertex].tree_status == TreeStatus::Source {
                for i in 0..self.net.outgoing[active_vertex].len() {
                    let edge = self.net.outgoing[active_vertex][i];
                    if self.net.has_capacity(edge) {
                        let target = self.net.arcs[edge].target;
                        match self.data[target].tree_status {
                            TreeStatus::Sink => return Some(edge),
                            TreeStatus::Free => {
                                let (distance, timestamp) = {
                                    let a = &self.data[active_vertex];
                                    (a.distance + 1, a.timestamp)
                                };
                                let t = &mut self.data[target];
                                t.parent_edge = Some(edge);
                                t.tree_status = TreeStatus::Source;
                                t.distance = distance;
                                t.timestamp = timestamp;
                                self.make_active(target);
                            }
                            TreeStatus::Source => {
                                // the distance heuristic may give a lower tree
                                if self.is_closer_to_terminal(active_vertex, target) {
                                    let (distance, timestamp) = {
                                        let a = &self.data[active_vertex];
                                        (a.distance + 1, a.timestamp)
                                    };
                                    let t = &mut self.data[target];
                                    t.parent_edge = Some(edge);
                                    t.distance = distance;
                                    t.timestamp = timestamp;
                                }
                            }
                        }
                    }
                }
            } else {
                // The Java class tests the arc and its source here, which for an arc leaving
                // the active vertex is the active vertex itself: the sink tree never grows.
                for i in 0..self.net.outgoing[active_vertex].len() {
                    let edge = self.net.outgoing[active_vertex][i];
                    if self.net.has_capacity(edge) {
                        let source = self.net.arcs[edge].source;
                        match self.data[source].tree_status {
                            TreeStatus::Source => return Some(edge),
                            TreeStatus::Free => {
                                let (distance, timestamp) = {
                                    let a = &self.data[active_vertex];
                                    (a.distance + 1, a.timestamp)
                                };
                                let s = &mut self.data[source];
                                s.parent_edge = Some(edge);
                                s.tree_status = TreeStatus::Sink;
                                s.distance = distance;
                                s.timestamp = timestamp;
                                self.make_active(source);
                            }
                            TreeStatus::Sink => {
                                if self.is_closer_to_terminal(active_vertex, source) {
                                    let (distance, timestamp) = {
                                        let a = &self.data[active_vertex];
                                        (a.distance + 1, a.timestamp)
                                    };
                                    let s = &mut self.data[source];
                                    s.parent_edge = Some(edge);
                                    s.distance = distance;
                                    s.timestamp = timestamp;
                                }
                            }
                        }
                    }
                }
            }
            self.finish_vertex(active_vertex);
        }
        None
    }

    /// Augments an s-t path specified using the `boundingEdge` and computes the set of tree
    /// orphans emerged after augmentation.
    ///
    /// First, the path flow bottleneck is found. Then the bottleneck flow value is pushed through
    /// every path edge. If some path edge gets saturated, the corresponding tree node is added to
    /// the orphan set. In the case the saturated edge connects source tree vertices, the edge target
    /// becomes an orphan, otherwise if the saturated edge connects sink tree vertices, that the edge
    /// source becomes an orphan.
    fn augment(&mut self, bounding_edge: usize) {
        let bottleneck = self.find_bottleneck(bounding_edge);
        self.net.push_flow_through(bounding_edge, bottleneck);
        let (s, t) = (self.net.source, self.net.sink);
        let mut source = self.net.arcs[bounding_edge].source;
        while source != s {
            let parent_edge = self.data[source].parent_edge.unwrap();
            self.net.push_flow_through(parent_edge, bottleneck);
            if !self.net.has_capacity(parent_edge) {
                self.data[source].parent_edge = None;
                self.orphans.push(source);
            }
            source = self.net.arcs[parent_edge].source;
        }
        let mut target = self.net.arcs[bounding_edge].target;
        while target != t {
            let parent_edge = self.data[target].parent_edge.unwrap();
            self.net.push_flow_through(parent_edge, bottleneck);
            if !self.net.has_capacity(parent_edge) {
                self.data[target].parent_edge = None;
                self.orphans.push(target);
            }
            target = self.net.arcs[parent_edge].target;
        }
        self.net.max_flow_value += bottleneck;
    }

    /// Finds augmenting path bottleneck by traversing the path edges.
    fn find_bottleneck(&self, bounding_edge: usize) -> f64 {
        let (s, t) = (self.net.source, self.net.sink);
        let mut bottleneck = self.net.residual_capacity(bounding_edge);
        let mut source = self.net.arcs[bounding_edge].source;
        while source != s {
            let p = self.data[source].parent_edge.unwrap();
            bottleneck = bottleneck.min(self.net.residual_capacity(p));
            source = self.net.arcs[p].source;
        }
        let mut target = self.net.arcs[bounding_edge].target;
        while target != t {
            let p = self.data[target].parent_edge.unwrap();
            bottleneck = bottleneck.min(self.net.residual_capacity(p));
            target = self.net.arcs[p].target;
        }
        bottleneck
    }

    /// Adopts all orphans.
    ///
    /// Processing every orphan, the goal of this procedure is to either find a parent node within
    /// the same tree, or identify that no such parent can be found, make the orphan a free vertex
    /// and process all descendants of this node the same way. If multiple parents exist, the closest
    /// to terminal is selected using distance and timestamp heuristic.
    fn adopt(&mut self) {
        while !self.orphans.is_empty() || !self.child_orphans.is_empty() {
            let current = self.next_orphan();
            if self.data[current].tree_status == TreeStatus::Source {
                self.adopt_source_orphan(current);
            } else {
                self.adopt_sink_orphan(current);
            }
        }
    }

    fn next_orphan(&mut self) -> usize {
        match self.child_orphans.pop_back() {
            Some(v) => v,
            None => self.orphans.pop().unwrap(),
        }
    }

    fn adopt_source_orphan(&mut self, current: usize) {
        match self.find_source_tree_parent(current) {
            None => self.make_source_tree_vertex_free(current),
            Some(parent) => self.adopt_with_parent(current, parent),
        }
    }

    fn find_source_tree_parent(&mut self, current: usize) -> Option<AdoptionParent> {
        let mut adoption_parent = None;
        let mut min_distance = i32::MAX as i64;
        for i in 0..self.net.outgoing[current].len() {
            let edge = self.net.outgoing[current][i];
            let inverse = self.net.arcs[edge].inverse;
            if !self.net.has_capacity(inverse) {
                continue;
            }
            let target = self.net.arcs[edge].target;
            if self.data[target].tree_status == TreeStatus::Source
                && self.has_connection_to_terminal(target)
                && self.data[target].distance < min_distance
            {
                min_distance = self.data[target].distance;
                adoption_parent = Some(AdoptionParent {
                    edge: inverse,
                    distance: min_distance,
                });
            }
        }
        adoption_parent
    }

    fn make_source_tree_vertex_free(&mut self, current: usize) {
        self.make_free(current);
        for i in 0..self.net.outgoing[current].len() {
            let edge = self.net.outgoing[current][i];
            let target = self.net.arcs[edge].target;
            if self.data[target].tree_status != TreeStatus::Source {
                continue;
            }
            if self.net.has_capacity(self.net.arcs[edge].inverse) {
                self.make_active(target);
            }
            if self.data[target].parent_edge == Some(edge) {
                self.data[target].parent_edge = None;
                self.child_orphans.push_front(target);
            }
        }
    }

    fn adopt_sink_orphan(&mut self, current: usize) {
        match self.find_sink_tree_parent(current) {
            None => self.make_sink_tree_vertex_free(current),
            Some(parent) => self.adopt_with_parent(current, parent),
        }
    }

    fn find_sink_tree_parent(&mut self, current: usize) -> Option<AdoptionParent> {
        let mut adoption_parent = None;
        let mut min_distance = i32::MAX as i64;
        for i in 0..self.net.outgoing[current].len() {
            let edge = self.net.outgoing[current][i];
            if !self.net.has_capacity(edge) {
                continue;
            }
            let target = self.net.arcs[edge].target;
            if self.data[target].tree_status == TreeStatus::Sink
                && self.has_connection_to_terminal(target)
                && self.data[target].distance < min_distance
            {
                min_distance = self.data[target].distance;
                adoption_parent = Some(AdoptionParent {
                    edge,
                    distance: min_distance,
                });
            }
        }
        adoption_parent
    }

    fn make_sink_tree_vertex_free(&mut self, current: usize) {
        self.make_free(current);
        for i in 0..self.net.outgoing[current].len() {
            let edge = self.net.outgoing[current][i];
            let target = self.net.arcs[edge].target;
            if self.data[target].tree_status != TreeStatus::Sink {
                continue;
            }
            if self.net.has_capacity(edge) {
                self.make_active(target);
            }
            if self.data[target].parent_edge == Some(self.net.arcs[edge].inverse) {
                self.data[target].parent_edge = None;
                self.child_orphans.push_front(target);
            }
        }
    }

    fn make_free(&mut self, current: usize) {
        self.data[current].timestamp = FREE_NODE_TIMESTAMP;
        self.data[current].tree_status = TreeStatus::Free;
    }

    fn adopt_with_parent(&mut self, current: usize, parent: AdoptionParent) {
        self.make_checked_in_this_iteration(current);
        self.data[current].parent_edge = Some(parent.edge);
        self.data[current].distance = parent.distance + 1;
    }

    /// Initializes a new algorithm iteration.
    fn next_iteration(&mut self) {
        self.current_timestamp += 1;
        self.make_checked_in_this_iteration(self.net.source);
        self.make_checked_in_this_iteration(self.net.sink);
    }

    /// Makes the `vertex` an active vertex.
    fn make_active(&mut self, v: usize) {
        if !self.data[v].active {
            self.data[v].active = true;
            self.active_vertices.push_front(v);
        }
    }

    /// Returns the next active vertex to be processed.
    fn next_active_vertex(&mut self) -> Option<usize> {
        while let Some(&next_active) = self.active_vertices.back() {
            if self.data[next_active].tree_status != TreeStatus::Free {
                return Some(next_active);
            }
            self.active_vertices.pop_back();
            self.data[next_active].active = false;
        }
        None
    }

    /// Makes the `vertex` inactive.
    fn finish_vertex(&mut self, v: usize) {
        debug_assert_eq!(self.active_vertices.back(), Some(&v));
        self.active_vertices.pop_back();
        self.data[v].active = false;
    }

    /// Sets the timestamp of the `vertex` equal to the `currentTimestamp`.
    fn make_checked_in_this_iteration(&mut self, v: usize) {
        self.data[v].timestamp = self.current_timestamp;
    }

    /// Checks if the distance of the `vertex` was updated during this iteration.
    fn was_checked_in_this_iteration(&self, v: usize) -> bool {
        self.data[v].timestamp == self.current_timestamp
    }

    fn parent_of(&self, v: usize) -> usize {
        let edge = self.data[v].parent_edge.expect("vertex has a parent edge");
        let arc = &self.net.arcs[edge];
        if v == arc.source {
            arc.target
        } else {
            arc.source
        }
    }

    /// Checks if the `vertex` is connected to a terminal vertex (source or sink).
    fn has_connection_to_terminal(&mut self, vertex: usize) -> bool {
        let (s, t) = (self.net.source, self.net.sink);
        let mut distance: i64 = 0;
        let mut current = vertex;
        while current != s && current != t {
            if self.data[current].parent_edge.is_none() {
                return false;
            } else if self.was_checked_in_this_iteration(vertex) {
                distance += self.data[current].distance;
                break;
            }
            distance += 1;
            current = self.parent_of(current);
        }
        let mut current = vertex;
        while !self.was_checked_in_this_iteration(current) {
            self.data[current].distance = distance;
            distance -= 1;
            self.make_checked_in_this_iteration(current);
            current = self.parent_of(current);
        }
        true
    }

    /// Checks if the vertex `p` is closer to terminal than the vertex `t` using the
    /// distance heuristic.
    fn is_closer_to_terminal(&self, p: usize, t: usize) -> bool {
        self.data[p].timestamp >= self.data[t].timestamp
            && self.data[p].distance + 1 < self.data[t].distance
    }
}

flow_algorithm_api!(BoykovKolmogorovMFImpl, crate::graph::OutgoingGraph);
