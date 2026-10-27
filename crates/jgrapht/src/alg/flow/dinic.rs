use super::DEFAULT_EPSILON;
use super::base::{FlowNet, flow_algorithm_api};
use crate::error::{GraphError, Result};
use crate::graph::Graph;
use std::collections::VecDeque;

/// `DinicMFImpl`: blocking flows on level graphs, `O(V² E)`.
pub struct DinicMFImpl<'g, G: Graph> {
    net: FlowNet<'g, G>,
    /// current-arc pointer per vertex
    index: Vec<usize>,
    /// BFS level per vertex, `-1` when unreached
    level: Vec<i64>,
}

impl<'g, G: Graph> DinicMFImpl<'g, G> {
    pub fn new(network: &'g G) -> Result<Self> {
        Self::with_epsilon(network, DEFAULT_EPSILON)
    }

    pub fn with_epsilon(network: &'g G, epsilon: f64) -> Result<Self> {
        if epsilon <= 0.0 {
            return Err(GraphError::InvalidArgument(
                "Epsilon must be positive!".into(),
            ));
        }
        let net = FlowNet::new(network, epsilon)?;
        net.check_capacities(epsilon, "Capacity must be non-negative!")?;
        Ok(DinicMFImpl {
            net,
            index: Vec::new(),
            level: Vec::new(),
        })
    }

    fn calculate(&mut self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        let (s, t) = self.net.resolve(
            source,
            sink,
            [
                "Network does not contain source!",
                "Network does not contain sink!",
                "Source is equal to sink!",
            ],
        )?;
        self.net.init(s, t);
        self.dinic();
        Ok(self.net.max_flow_value)
    }

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

flow_algorithm_api!(DinicMFImpl);
