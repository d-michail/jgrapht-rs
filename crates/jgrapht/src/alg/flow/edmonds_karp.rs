use super::DEFAULT_EPSILON;
use super::base::{FlowNet, flow_algorithm_api};
use crate::error::{GraphError, Result};
use crate::graph::Graph;
use std::collections::{HashSet, VecDeque};

/// `EdmondsKarpMFImpl`: shortest augmenting paths found by breadth-first search, `O(V E²)`.
/// All shortest paths of a round are augmented together.
pub struct EdmondsKarpMFImpl<'g, G: Graph> {
    net: FlowNet<'g, G>,
    visited: Vec<bool>,
    /// last arc(s) of the shortest path(s) used to reach each vertex
    last_arcs: Vec<Option<Vec<usize>>>,
}

impl<'g, G: Graph> EdmondsKarpMFImpl<'g, G> {
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
