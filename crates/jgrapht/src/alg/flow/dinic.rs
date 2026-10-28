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

use super::DEFAULT_EPSILON;
use super::base::{FlowNet, flow_algorithm_api};
use crate::error::{GraphError, Result};
use crate::graph::OutgoingGraph;
use std::collections::VecDeque;

/// Implementation of [Dinic algorithm](https://en.wikipedia.org/wiki/Dinic%27s_algorithm) with scaling for [maximum flow problem](https://en.wikipedia.org/wiki/Maximum_flow_problem).
///
/// The running time of the algorithm is $O(n^2m)$.
///
/// Dinic algorithm firstly was mentioned in *DINIC, E. A. 1970. Algorithm for Solution
/// of a Problem of Maximum Flow in Networks With Power Estimation. Soviet Math. Dokl. 11,
/// 1277-1280.
///
/// Scheme of the algorithm:
///
/// 1). Create a level graph. If we can't reach the sink return flow value.
///
/// 2). Find a blocking flow $f'$ in the level graph.
///
/// 3). Add $f'$ to the flow $f$. Move to the step $1$.
///
/// # Rust notes
///
/// `DinicMFImpl`: blocking flows on level graphs, `O(V² E)`.
pub struct DinicMFImpl<'g, G: OutgoingGraph> {
    net: FlowNet<'g, G>,
    /// current-arc pointer per vertex
    index: Vec<usize>,
    /// BFS level per vertex, `-1` when unreached
    level: Vec<i64>,
}

impl<'g, G: OutgoingGraph> DinicMFImpl<'g, G> {
    /// Constructor. Constructs a new network on which we will calculate the maximum flow, using
    /// Dinic algorithm.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(network: &'g G) -> Result<Self> {
        Self::with_epsilon(network, DEFAULT_EPSILON)
    }

    /// Creates an instance with the given epsilon.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if a numeric or collection parameter is out of range; errors of the underlying graph operations are propagated.
    pub fn with_epsilon(network: &'g G, epsilon: f64) -> Result<Self> {
        if epsilon <= 0.0 {
            return Err(GraphError::must_be_positive("epsilon"));
        }
        let net = FlowNet::new(network, epsilon)?;
        net.check_capacities(epsilon)?;
        Ok(DinicMFImpl {
            net,
            index: Vec::new(),
            level: Vec::new(),
        })
    }

    fn calculate(&mut self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        let (s, t) = self.net.resolve(source, sink)?;
        self.net.init(s, t)?;
        self.dinic();
        Ok(self.net.max_flow_value)
    }

    /// Creates a level graph. We can split all vertices of the graph in disjoint sets. In the same
    /// set will lie vertices with equal distance from the source. It's obvious that level network
    /// cannot contain edges $i \to j$, where $i$ and $j$ are two vertices for which holds: $|i.level
    /// - j.level| > 1$. It follows from a property of the shortest paths. Level graph contains only
    ///   edges that lead from level $i$ to the level $i + 1$. Thus level graph does not contain
    ///   backward edges or edges that lead from $i$-th level to $i$-th.
    fn bfs(&mut self) -> bool {
        let (s, t) = (self.net.source, self.net.sink);
        self.level = vec![-1; self.net.vs.len()];
        let mut queue = VecDeque::from([s]);
        self.level[s] = 0;
        while !queue.is_empty() && self.level[t] == -1 {
            let v = queue.pop_front().unwrap();
            for &edge in &self.net.outgoing[v] {
                let arc = &self.net.arcs[edge];
                let u = arc.target;
                if self.net.comparator.compare(arc.flow, arc.capacity) < 0 && self.level[u] == -1 {
                    self.level[u] = self.level[v] + 1;
                    queue.push_back(u);
                }
            }
        }
        self.level[t] != -1
    }

    /// Finds a blocking flow in the network. For each vertex we have a pointer on the first edge
    /// which we can use to reach the sink. If we can't reach the sink using current edge, we
    /// increment the pointer. So on each iteration we either saturate at least one edge or we
    /// increment pointer.
    fn dfs(&mut self, v: usize, flow: f64) -> f64 {
        if self.net.comparator.compare(0.0, flow) == 0 {
            return flow;
        }
        if v == self.net.sink {
            return flow;
        }
        while self.index[v] < self.net.outgoing[v].len() {
            let edge = self.net.outgoing[v][self.index[v]];
            let arc = &self.net.arcs[edge];
            let u = arc.target;
            if self.net.comparator.compare(arc.flow, arc.capacity) < 0
                && self.level[u] == self.level[v] + 1
            {
                let pushed = self.dfs(u, flow.min(arc.capacity - arc.flow));
                if self.net.comparator.compare(pushed, 0.0) != 0 {
                    self.net.push_flow_through(edge, pushed);
                    return pushed;
                }
            }
            self.index[v] += 1;
        }
        0.0
    }

    /// Runs Dinic algorithm with scaling. Construct a level graph, then find blocking flow and
    /// finally increase the flow.
    fn dinic(&mut self) {
        while self.bfs() {
            self.index = vec![0; self.net.vs.len()];
            loop {
                let pushed = self.dfs(self.net.source, f64::INFINITY);
                if pushed == 0.0 {
                    break;
                }
                self.net.max_flow_value += pushed;
            }
        }
    }
}

flow_algorithm_api!(DinicMFImpl, crate::graph::OutgoingGraph);
