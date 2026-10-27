use super::DEFAULT_EPSILON;
use super::base::{FlowNet, flow_algorithm_api};
use crate::error::Result;
use crate::graph::Graph;
use crate::util::ToleranceDoubleComparator;
use std::collections::VecDeque;

/// `PushRelabelMFImpl`: FIFO push-relabel with the global-relabelling and gap heuristics,
/// `O(V³)`.
pub struct PushRelabelMFImpl<'g, G: Graph> {
    net: FlowNet<'g, G>,
    n: usize,
    count_height: Vec<usize>,
    active_vertices: VecDeque<usize>,
    height: Vec<usize>,
    active: Vec<bool>,
    current_arc: Vec<usize>,
    relabel_counter: usize,
    /// As in Java, the push-relabel steps compare with the default tolerance whatever epsilon the
    /// base class was given.
    comparator: ToleranceDoubleComparator,
}

impl<'g, G: Graph> PushRelabelMFImpl<'g, G> {
    pub fn new(network: &'g G) -> Result<Self> {
        Self::with_epsilon(network, DEFAULT_EPSILON)
    }

    pub fn with_epsilon(network: &'g G, epsilon: f64) -> Result<Self> {
        let net = FlowNet::new(network, epsilon)?;
        let n = net.vs.len();
        Ok(PushRelabelMFImpl {
            net,
            n,
            count_height: Vec::new(),
            active_vertices: VecDeque::new(),
            height: vec![0; n],
            active: vec![false; n],
            current_arc: vec![0; n],
            relabel_counter: 0,
            comparator: ToleranceDoubleComparator::default(),
        })
    }

    /// Computes the flow and returns its value (`calculateMaximumFlow`).
    pub fn calculate_maximum_flow(&mut self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        self.calculate(source, sink)
    }

    fn has_excess(&self, v: usize) -> bool {
        self.comparator.compare(self.net.excess[v], 0.0) > 0
    }

    fn enqueue(&mut self, v: usize) {
        if !self.active[v] && self.has_excess(v) {
            self.active[v] = true;
            self.active_vertices.push_back(v);
        }
    }

    fn initialize(&mut self, source: usize, sink: usize) {
        let n = self.n;
        self.active_vertices.clear();
        for i in 0..n {
            self.net.excess[i] = 0.0;
            self.height[i] = 0;
            self.active[i] = false;
            self.current_arc[i] = 0;
        }
        self.height[source] = n;
        self.active[source] = true;
        self.active[sink] = true;
        self.count_height[n] = 1;
        self.count_height[0] = n - 1;
        for k in 0..self.net.outgoing[source].len() {
            let ex = self.net.outgoing[source][k];
            self.net.excess[source] += self.net.arcs[ex].capacity;
            self.push(ex);
        }
        self.recompute_heights_heuristic();
        self.relabel_counter = 0;
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
        self.net.init(s, t);
        self.count_height = vec![0; 2 * self.n + 1];
        self.initialize(s, t);
        while let Some(v) = self.active_vertices.pop_front() {
            self.active[v] = false;
            self.discharge(v);
        }
        let g = self.net.graph;
        let mut value = 0.0;
        for e in g.edges_of(self.net.vs[t]) {
            let a = self.net.arc_of_edge(e);
            let arc = &self.net.arcs[a];
            value += if self.net.directed {
                arc.flow
            } else {
                arc.flow + self.net.arcs[arc.inverse].flow
            };
        }
        self.net.max_flow_value += value;
        Ok(self.net.max_flow_value)
    }

    /// The push-relabel override of `pushFlowThrough`, which also moves excess.
    fn push_flow_through(&mut self, ex: usize, f: f64) {
        let (s, t) = (self.net.arcs[ex].source, self.net.arcs[ex].target);
        self.net.excess[s] -= f;
        self.net.excess[t] += f;
        self.net.push_flow_through(ex, f);
    }

    /// Pushes as much excess as the arc admits (precondition: `height(u) > height(v)` is checked
    /// here, as in the Java class).
    fn push(&mut self, ex: usize) {
        let arc = &self.net.arcs[ex];
        let (u, v) = (arc.source, arc.target);
        let delta = self.net.excess[u].min(arc.capacity - arc.flow);
        if self.height[u] <= self.height[v] || self.comparator.compare(delta, 0.0) <= 0 {
            return;
        }
        self.push_flow_through(ex, delta);
        self.enqueue(v);
    }

    fn gap_heuristic(&mut self, l: usize) {
        let n = self.n;
        for i in 0..n {
            if l < self.height[i] && self.height[i] < n {
                self.count_height[self.height[i]] -= 1;
                self.height[i] = self.height[i].max(n + 1);
                self.count_height[self.height[i]] += 1;
            }
        }
    }

    fn relabel(&mut self, u: usize) {
        let n = self.n;
        let old_height = self.height[u];
        self.count_height[old_height] -= 1;
        self.height[u] = 2 * n;
        for k in 0..self.net.outgoing[u].len() {
            let ex = self.net.outgoing[u][k];
            if self.net.has_capacity(ex) {
                self.height[u] = self.height[u].min(self.height[self.net.arcs[ex].target] + 1);
            }
        }
        self.count_height[self.height[u]] += 1;
        if 0 < old_height && old_height < n && self.count_height[old_height] == 0 {
            self.gap_heuristic(old_height);
        }
    }

    fn bfs(&mut self, queue: &mut VecDeque<usize>, visited: &mut [bool]) {
        while let Some(id) = queue.pop_front() {
            for &ex in &self.net.outgoing[id] {
                let v = self.net.arcs[ex].target;
                if !visited[v] && self.net.has_capacity(self.net.arcs[ex].inverse) {
                    self.height[v] = self.height[id] + 1;
                    visited[v] = true;
                    queue.push_back(v);
                }
            }
        }
    }

    fn recompute_heights_heuristic(&mut self) {
        let n = self.n;
        self.count_height.iter_mut().for_each(|c| *c = 0);
        let mut queue = VecDeque::with_capacity(n);
        let mut visited = vec![false; n];
        self.height.iter_mut().for_each(|h| *h = 2 * n);
        let (sink, source) = (self.net.sink, self.net.source);
        self.height[source] = n;
        visited[source] = true;
        self.height[sink] = 0;
        visited[sink] = true;
        queue.push_back(sink);
        self.bfs(&mut queue, &mut visited);
        queue.push_back(source);
        self.bfs(&mut queue, &mut visited);
        for i in 0..n {
            self.count_height[self.height[i]] += 1;
        }
    }

    fn discharge(&mut self, u: usize) {
        while self.has_excess(u) {
            if self.current_arc[u] >= self.net.outgoing[u].len() {
                self.relabel(u);
                self.relabel_counter += 1;
                if self.relabel_counter == self.n {
                    self.recompute_heights_heuristic();
                    self.current_arc.iter_mut().for_each(|c| *c = 0);
                    self.relabel_counter = 0;
                }
                self.current_arc[u] = 0;
            } else {
                let ex = self.net.outgoing[u][self.current_arc[u]];
                if self.is_admissible(ex) {
                    self.push(ex);
                } else {
                    self.current_arc[u] += 1;
                }
            }
        }
    }

    fn is_admissible(&self, e: usize) -> bool {
        let arc = &self.net.arcs[e];
        self.net.has_capacity(e) && self.height[arc.source] == self.height[arc.target] + 1
    }
}

flow_algorithm_api!(PushRelabelMFImpl);
