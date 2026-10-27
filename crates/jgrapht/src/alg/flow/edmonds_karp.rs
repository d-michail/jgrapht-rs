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

use crate::graph::IncidenceGraph;
use crate::graph::OutgoingGraph;
use super::DEFAULT_EPSILON;
use super::base::{FlowNet, flow_algorithm_api};
use crate::error::{GraphError, Result};
use std::collections::{HashSet, VecDeque};

/// This class computes a maximum flow in a
/// [flow network](http://en.wikipedia.org/wiki/Flow_network) using
/// [Edmonds-Karp algorithm](http://en.wikipedia.org/wiki/Edmonds-Karp_algorithm). Given
/// is a weighted directed or undirected graph $G(V,E)$ with vertex set $V$ and edge set $E$. Each
/// edge $e\in E$ has an associated non-negative capacity $u_e$. The maximum flow problem involves
/// finding a feasible flow from a source vertex $s$ to a sink vertex $t$ which is maximum. The
/// amount of flow $f_e$ through any edge $e$ cannot exceed capacity $u_e$. Moreover, flow
/// conservation must hold: the sum of flows entering a node must equal the sum of flows exiting that
/// node, except for the source and the sink nodes.
///
/// Mathematically, the maximum flow problem is stated as follows: \[ \begin{align} \max~&
/// \sum_{e\in \delta^+(s)}f_e &\\ \mbox{s.t. }&\sum_{e\in \delta^-(i)} f_e=\sum_{e\in
/// \delta^+(i)} f_e & \forall i\in V\setminus\{s,t\}\\ &0\leq f_e \leq u_e & \forall
/// e\in E \end{align} \] Here $\delta^+(i)$ resp $\delta^-(i)$ denote resp the outgoing and incoming
/// edges of vertex $i$.
///
/// When the input graph is undirected, an edge $(i,j)$ is treated as two directed arcs: $(i,j)$ and
/// $(j,i)$. In such a case, there is the additional restriction that the flow can only go in one
/// direction: the flow either goes form $i$ to $j$, or from $j$ to $i$, but there cannot be a
/// positive flow on $(i,j)$ and $(j,i)$ simultaneously.
///
/// The runtime complexity of this class is $O(nm^2)$, where $n$ is the number of vertices and $m$
/// the number of edges in the graph. For a more efficient algorithm, consider using
/// `PushRelabelMFImpl` instead.
///
/// This class can also compute minimum s-t cuts. Effectively, to compute a minimum s-t cut, the
/// implementation first computes a minimum s-t flow, after which a BFS is run on the residual graph.
///
/// For more details see Andrew V. Goldberg's *Combinatorial Optimization (Lecture Notes)*.
///
/// Note: even though the algorithm accepts any kind of graph, currently only Simple directed and
/// undirected graphs are supported (and tested!).
///
/// # Rust notes
///
/// `EdmondsKarpMFImpl`: shortest augmenting paths found by breadth-first search, `O(V E²)`.
/// All shortest paths of a round are augmented together.
pub struct EdmondsKarpMFImpl<'g, G: IncidenceGraph> {
    net: FlowNet<'g, G>,
    visited: Vec<bool>,
    /// last arc(s) of the shortest path(s) used to reach each vertex
    last_arcs: Vec<Option<Vec<usize>>>,
}

impl<'g, G: OutgoingGraph + IncidenceGraph> EdmondsKarpMFImpl<'g, G> {
    /// Constructs `MaximumFlow` instance to work with *a copy of* `network`.
    /// Current source and sink are set to `null`. If `network` is weighted, then
    /// capacities are weights, otherwise all capacities are equal to one. Doubles are compared using
    /// `DEFAULT_EPSILON` tolerance.
    pub fn new(network: &'g G) -> Result<Self> {
        Self::with_epsilon(network, DEFAULT_EPSILON)
    }

    pub fn with_epsilon(network: &'g G, epsilon: f64) -> Result<Self> {
        if epsilon <= 0.0 {
            return Err(GraphError::InvalidArgument(
                "invalid epsilon (must be positive)".into(),
            ));
        }
        let net = FlowNet::new(network, epsilon)?;
        net.check_capacities(epsilon, "invalid capacity (must be non-negative)")?;
        Ok(EdmondsKarpMFImpl {
            net,
            visited: Vec::new(),
            last_arcs: Vec::new(),
        })
    }

    /// Computes the flow and returns its value (`calculateMaximumFlow`).
    pub fn calculate_maximum_flow(&mut self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        self.calculate(source, sink)
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
        loop {
            self.breadth_first_search();
            if !self.visited[t] {
                break;
            }
            let increase = self.augment_flow();
            self.net.max_flow_value += increase;
        }
        Ok(self.net.max_flow_value)
    }

    /// Method which finds a path from source to sink the in the residual graph. Note that this
    /// method tries to find multiple paths at once. Once a single path has been discovered, no new
    /// nodes are added to the queue, but nodes which are already in the queue are fully explored. As
    /// such there's a chance that multiple paths are discovered.
    fn breadth_first_search(&mut self) {
        let n = self.net.vs.len();
        self.visited = vec![false; n];
        self.last_arcs = vec![None; n];
        let (s, t) = (self.net.source, self.net.sink);
        let mut queue = VecDeque::from([s]);
        self.visited[s] = true;
        self.net.excess[s] = f64::INFINITY;
        self.net.excess[t] = 0.0;
        let mut seen_sink = false;
        while let Some(u) = queue.pop_front() {
            for k in 0..self.net.outgoing[u].len() {
                let ex = self.net.outgoing[u][k];
                let arc = &self.net.arcs[ex];
                if self.net.comparator.compare(arc.flow, arc.capacity) < 0 {
                    let v = arc.target;
                    let room = arc.capacity - arc.flow;
                    if v == t {
                        self.visited[v] = true;
                        self.last_arcs[v].get_or_insert_with(Vec::new).push(ex);
                        self.net.excess[v] += self.net.excess[u].min(room);
                        seen_sink = true;
                    } else if !self.visited[v] {
                        self.visited[v] = true;
                        self.net.excess[v] = self.net.excess[u].min(room);
                        self.last_arcs[v] = Some(vec![ex]);
                        if !seen_sink {
                            queue.push_back(v);
                        }
                    }
                }
            }
        }
    }

    /// For all paths which end in the sink. trace them back to the source and push flow through
    /// them.
    fn augment_flow(&mut self) -> f64 {
        let mut flow_increase = 0.0;
        let mut seen = HashSet::new();
        let sink_arcs = self.last_arcs[self.net.sink].clone().unwrap_or_default();
        for ex in sink_arcs {
            let arc = &self.net.arcs[ex];
            let source = arc.source;
            let delta_flow = self.net.excess[source].min(arc.capacity - arc.flow);
            if self.augment_flow_along_internal(delta_flow, source, &mut seen) {
                self.net.push_flow_through(ex, delta_flow);
                flow_increase += delta_flow;
            }
        }
        flow_increase
    }

    fn augment_flow_along_internal(
        &mut self,
        delta_flow: f64,
        node: usize,
        seen: &mut HashSet<usize>,
    ) -> bool {
        if node == self.net.source {
            return true;
        }
        if !seen.insert(node) {
            return false;
        }
        let prev = self.last_arcs[node].as_ref().unwrap()[0];
        let prev_source = self.net.arcs[prev].source;
        if self.augment_flow_along_internal(delta_flow, prev_source, seen) {
            self.net.push_flow_through(prev, delta_flow);
            return true;
        }
        false
    }
}

flow_algorithm_api!(EdmondsKarpMFImpl);
