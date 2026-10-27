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

use crate::graph::GraphExt;
use crate::graph::OutgoingGraph;
use crate::error::{GraphError, Result};
use crate::util::ToleranceDoubleComparator;
use indexmap::IndexSet;
use std::collections::{HashMap, VecDeque};

/// `MaximumFlowAlgorithm.MaximumFlow`: the flow value and the flow on every edge.
#[derive(Debug, Clone)]
pub struct MaximumFlow<E> {
    pub value: f64,
    pub flow_map: HashMap<E, f64>,
}

impl<E: Eq + std::hash::Hash> MaximumFlow<E> {
    /// Flow on `edge`, if it is an edge of the network.
    pub fn flow(&self, edge: &E) -> Option<f64> {
        self.flow_map.get(edge).copied()
    }
}

/// Allows to derive [maximum-flow](https://en.wikipedia.org/wiki/Maximum_flow_problem)
/// from the supplied [flow network](https://en.wikipedia.org/wiki/Flow_network)
///
/// # Rust notes
///
/// `MaximumFlowAlgorithm` together with the `FlowAlgorithm` queries.
pub trait MaximumFlowAlgorithm<V, E> {
    /// Computes a maximum flow from `source` to `sink`.
    fn maximum_flow(&mut self, source: &V, sink: &V) -> Result<MaximumFlow<E>>;

    /// The value of a maximum flow from `source` to `sink`.
    fn maximum_flow_value(&mut self, source: &V, sink: &V) -> Result<f64> {
        Ok(self.maximum_flow(source, sink)?.value)
    }

    /// The flow on every edge after the last computation.
    fn flow_map(&mut self) -> Result<&HashMap<E, f64>>;

    /// The endpoint of `edge` the flow runs towards (the target of a directed edge).
    fn flow_direction(&self, edge: &E) -> Result<V>;
}

/// Given a weighted graph $G(V,E)$ (directed or undirected). This class computes a minimum $s-t$
/// cut. A cut is a partitioning of the vertices into two disjoint sets $S, T $such that $s \in S, t
/// \in T$, and that $S \cup T = V$. The *capacity* of a cut is defined as the sum of the
/// weights of the edges from $S$ to $T$. In case of a directed graph, only the edges with their tail
/// in $S$ and their head in $T$ are counted. In cased of a undirected graph, all edges with one
/// endpoint in $S$ and one endpoint in $T$ are counted. For a given $s$ and $t$, this class computes
/// two partitions $S$ and $T$ such that the capacity of the cut is minimized. When each edge has
/// equal weight, by definition this class minimizes the number of edges from $S$ to $T$.
///
/// Note: it is not recommended to use this class to calculate the overall minimum cut in a graph by
/// iteratively invoking this class for all source-sink pairs. This is computationally expensive.
/// Instead, use the StoerWagnerMinimumCut implementation.
///
/// # Rust notes
///
/// `MinimumSTCutAlgorithm`.
pub trait MinimumSTCutAlgorithm<V, E> {
    /// Computes a minimum cut and returns its capacity.
    fn calculate_min_cut(&mut self, source: &V, sink: &V) -> Result<f64>;
    /// Capacity of the last cut.
    fn cut_capacity(&mut self) -> f64;
    /// Vertices on the source side of the last cut.
    fn source_partition(&mut self) -> &IndexSet<V>;
    /// Vertices on the sink side of the last cut.
    fn sink_partition(&mut self) -> &IndexSet<V>;
    /// Edges crossing the last cut.
    fn cut_edges(&mut self) -> &IndexSet<E>;
}

/// `AnnotatedFlowEdge`: one direction of an edge in the residual network.
#[derive(Debug, Clone)]
pub(crate) struct Arc {
    pub source: usize,
    pub target: usize,
    pub inverse: usize,
    /// Index of the network edge this arc stands for (`None` for a synthetic backward arc).
    pub prototype: Option<usize>,
    /// Maximum by which the flow in this direction can be increased (on top of the flow in the
    /// opposite direction).
    pub capacity: f64,
    /// Flow in the direction of this arc.
    pub flow: f64,
}

/// `MaximumFlowAlgorithmBase`: the residual network built from the input graph, plus the cut and
/// flow-map queries that all algorithms share. Vertices and arcs are addressed by index.
pub(crate) struct FlowNet<'g, G: OutgoingGraph> {
    pub graph: &'g G,
    pub directed: bool,
    pub comparator: ToleranceDoubleComparator,
    pub vs: Vec<&'g G::Vertex>,
    vindex: HashMap<&'g G::Vertex, usize>,
    es: Vec<&'g G::Edge>,
    eindex: HashMap<&'g G::Edge, usize>,
    pub arcs: Vec<Arc>,
    /// `VertexExtensionBase.outgoing`
    pub outgoing: Vec<Vec<usize>>,
    /// `VertexExtensionBase.excess`
    pub excess: Vec<f64>,
    /// the arc of every network edge, by edge index
    edge_arc: Vec<usize>,
    pub source: usize,
    pub sink: usize,
    pub max_flow_value: f64,
    pub max_flow: Option<HashMap<G::Edge, f64>>,
    source_partition: Option<IndexSet<G::Vertex>>,
    sink_partition: Option<IndexSet<G::Vertex>>,
    cut_edges: Option<IndexSet<G::Edge>>,
}

impl<'g, G: OutgoingGraph> FlowNet<'g, G> {
    pub fn new(graph: &'g G, epsilon: f64) -> Result<Self> {
        let vs: Vec<&G::Vertex> = graph.vertices().collect();
        let es: Vec<&G::Edge> = graph.edges().collect();
        Ok(FlowNet {
            graph,
            directed: graph.graph_type().directed,
            comparator: ToleranceDoubleComparator::new(epsilon)?,
            vindex: vs.iter().enumerate().map(|(i, v)| (*v, i)).collect(),
            eindex: es.iter().enumerate().map(|(i, e)| (*e, i)).collect(),
            vs,
            es,
            arcs: Vec::new(),
            outgoing: Vec::new(),
            excess: Vec::new(),
            edge_arc: Vec::new(),
            source: 0,
            sink: 0,
            max_flow_value: -1.0,
            max_flow: None,
            source_partition: None,
            sink_partition: None,
            cut_edges: None,
        })
    }

    /// The validation shared by the constructors: capacities must not be negative.
    pub fn check_capacities(&self, epsilon: f64, what: &str) -> Result<()> {
        if self
            .es
            .iter()
            .any(|e| self.weight(self.eindex[*e]) < -epsilon)
        {
            return Err(GraphError::InvalidArgument(what.into()));
        }
        Ok(())
    }

    pub fn vertex_index(&self, v: &G::Vertex) -> Option<usize> {
        self.vindex.get(v).copied()
    }

    /// Checks that `source` and `sink` are distinct vertices of the network.
    pub fn resolve(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
        msgs: [&str; 3],
    ) -> Result<(usize, usize)> {
        let s = self
            .vertex_index(source)
            .ok_or_else(|| GraphError::InvalidArgument(msgs[0].into()))?;
        let t = self
            .vertex_index(sink)
            .ok_or_else(|| GraphError::InvalidArgument(msgs[1].into()))?;
        if s == t {
            return Err(GraphError::InvalidArgument(msgs[2].into()));
        }
        Ok((s, t))
    }

    fn weight(&self, e: usize) -> f64 {
        self.graph.weight_of(self.es[e])
    }

    /// `init` + `buildInternal`: rebuilds the residual network for a new computation.
    pub fn init(&mut self, source: usize, sink: usize) -> Result<()> {
        let n = self.vs.len();
        self.arcs = (0..self.es.len())
            .map(|_| Arc {
                source: 0,
                target: 0,
                inverse: usize::MAX,
                prototype: None,
                capacity: 0.0,
                flow: 0.0,
            })
            .collect();
        self.edge_arc = (0..self.es.len()).collect();
        self.outgoing = vec![Vec::new(); n];
        self.excess = vec![0.0; n];
        self.build_internal()?;
        self.source = source;
        self.sink = sink;
        self.max_flow_value = 0.0;
        self.max_flow = None;
        self.source_partition = None;
        self.sink_partition = None;
        self.cut_edges = None;
        Ok(())
    }

    fn build_internal(&mut self) -> Result<()> {
        let g = self.graph;
        if self.directed {
            for u in 0..self.vs.len() {
                for e in g.out_edges_of(self.vs[u]) {
                    let ei = self.eindex[e];
                    let v = self.vindex[g.edge_target(e).unwrap()];
                    let forward = self.create_edge(u, v, ei);
                    let backward = self.create_backward_edge(forward);
                    self.outgoing[u].push(forward);
                    if self.arcs[backward].prototype.is_none() {
                        self.outgoing[v].push(backward);
                    }
                }
            }
        } else {
            for ei in 0..self.es.len() {
                let e = self.es[ei];
                let u = self.vindex[g.edge_source(e).unwrap()];
                let v = self.vindex[g.edge_target(e).unwrap()];
                let forward = self.create_edge(u, v, ei);
                let backward = self.create_backward_edge(forward);
                self.outgoing[u].push(forward);
                self.outgoing[v].push(backward);
            }
        }
        Ok(())
    }

    fn create_edge(&mut self, source: usize, target: usize, e: usize) -> usize {
        let weight = self.weight(e);
        let a = self.edge_arc[e];
        let arc = &mut self.arcs[a];
        arc.source = source;
        arc.target = target;
        arc.capacity = weight;
        arc.prototype = Some(e);
        a
    }

    fn create_backward_edge(&mut self, forward: usize) -> usize {
        let (fs, ft) = (self.arcs[forward].source, self.arcs[forward].target);
        let backward_prototype = self
            .graph
            .get_edge(self.vs[ft], self.vs[fs])
            .map(|e| self.eindex[e]);
        let backward = match backward_prototype {
            Some(p) if self.directed => self.create_edge(ft, fs, p),
            _ => {
                let mut arc = Arc {
                    source: ft,
                    target: fs,
                    inverse: usize::MAX,
                    prototype: None,
                    capacity: 0.0,
                    flow: 0.0,
                };
                if !self.directed {
                    let p = backward_prototype.expect("an undirected edge is its own reverse");
                    arc.capacity = self.weight(p);
                    arc.prototype = Some(p);
                }
                self.arcs.push(arc);
                self.arcs.len() - 1
            }
        };
        self.arcs[forward].inverse = backward;
        self.arcs[backward].inverse = forward;
        backward
    }

    /// The arc standing for network edge `e` (its forward direction).
    pub fn arc_of_edge(&self, e: &G::Edge) -> usize {
        self.edge_arc[self.eindex[e]]
    }

    pub fn has_capacity(&self, a: usize) -> bool {
        self.comparator
            .compare(self.arcs[a].capacity, self.arcs[a].flow)
            > 0
    }

    pub fn residual_capacity(&self, a: usize) -> f64 {
        self.arcs[a].capacity - self.arcs[a].flow
    }

    pub fn push_flow_through(&mut self, edge: usize, flow: f64) {
        let inverse = self.arcs[edge].inverse;
        debug_assert!(
            self.comparator.compare(self.arcs[edge].flow, 0.0) == 0
                || self.comparator.compare(self.arcs[inverse].flow, 0.0) == 0
        );
        if self.comparator.compare(self.arcs[inverse].flow, flow) < 0 {
            let inv_flow = self.arcs[inverse].flow;
            let flow_difference = flow - inv_flow;
            self.arcs[edge].flow += flow_difference;
            self.arcs[edge].capacity -= inv_flow;
            self.arcs[inverse].flow = 0.0;
            self.arcs[inverse].capacity += flow_difference;
        } else {
            self.arcs[edge].capacity -= flow;
            self.arcs[inverse].flow -= flow;
        }
    }

    pub fn compose_flow(&self) -> HashMap<G::Edge, f64> {
        (0..self.es.len())
            .map(|ei| {
                let a = &self.arcs[self.edge_arc[ei]];
                let f = if self.directed {
                    a.flow
                } else {
                    a.flow.max(self.arcs[a.inverse].flow)
                };
                (self.es[ei].clone(), f)
            })
            .collect()
    }

    pub fn flow_map(&mut self) -> &HashMap<G::Edge, f64> {
        if self.max_flow.is_none() {
            self.max_flow = Some(self.compose_flow());
        }
        self.max_flow.as_ref().unwrap()
    }

    pub fn flow_direction(&self, e: &G::Edge) -> Result<G::Vertex> {
        let ei = *self.eindex.get(e).ok_or_else(|| {
            GraphError::InvalidArgument(
                "Cannot query the flow on an edge which does not exist in the input graph!".into(),
            )
        })?;
        let a = &self.arcs[self.edge_arc[ei]];
        if self.directed {
            return Ok(self.vs[a.target].clone());
        }
        let inverse = &self.arcs[a.inverse];
        Ok(if a.flow > inverse.flow {
            self.vs[a.target].clone()
        } else {
            self.vs[inverse.target].clone()
        })
    }

    pub fn source_partition(&mut self) -> &IndexSet<G::Vertex> {
        if self.source_partition.is_none() {
            let mut partition = IndexSet::new();
            let mut seen = vec![false; self.vs.len()];
            let mut queue = VecDeque::from([self.source]);
            while let Some(v) = queue.pop_front() {
                if seen[v] {
                    continue;
                }
                seen[v] = true;
                partition.insert(self.vs[v].clone());
                for &a in &self.outgoing[v] {
                    if self.has_capacity(a) {
                        queue.push_back(self.arcs[a].target);
                    }
                }
            }
            self.source_partition = Some(partition);
        }
        self.source_partition.as_ref().unwrap()
    }

    pub fn sink_partition(&mut self) -> &IndexSet<G::Vertex> {
        if self.sink_partition.is_none() {
            let source_side = self.source_partition().clone();
            let sink_side = self
                .vs
                .iter()
                .filter(|v| !source_side.contains(**v))
                .map(|v| (*v).clone())
                .collect();
            self.sink_partition = Some(sink_side);
        }
        self.sink_partition.as_ref().unwrap()
    }

    pub fn cut_edges(&mut self) -> &IndexSet<G::Edge> {
        if self.cut_edges.is_none() {
            let p1 = self.source_partition().clone();
            let g = self.graph;
            let mut cut = IndexSet::new();
            if self.directed {
                for v in &p1 {
                    for e in g.out_edges_of(v) {
                        if !p1.contains(g.edge_target(e).unwrap()) {
                            cut.insert(e.clone());
                        }
                    }
                }
            } else {
                for e in g.edges() {
                    if p1.contains(g.edge_source(e).unwrap())
                        ^ p1.contains(g.edge_target(e).unwrap())
                    {
                        cut.insert(e.clone());
                    }
                }
            }
            self.cut_edges = Some(cut);
        }
        self.cut_edges.as_ref().unwrap()
    }
}

/// Implements [`MaximumFlowAlgorithm`] and [`MinimumSTCutAlgorithm`] for an algorithm struct with a
/// `net: FlowNet` field and a `calculate(&mut self, &V, &V) -> Result<f64>` method.
macro_rules! flow_algorithm_api {
    ($name:ident, $bound:path) => {
        impl<'g, G: $bound> $name<'g, G> {
            /// The source of the last computation.
            pub fn current_source(&self) -> Option<&G::Vertex> {
                (self.net.max_flow_value >= 0.0).then(|| self.net.vs[self.net.source])
            }

            /// The sink of the last computation.
            pub fn current_sink(&self) -> Option<&G::Vertex> {
                (self.net.max_flow_value >= 0.0).then(|| self.net.vs[self.net.sink])
            }

            /// The flow value of the last computation (`-1` before the first one).
            pub fn last_flow_value(&self) -> f64 {
                self.net.max_flow_value
            }
        }

        impl<'g, G: $bound> $crate::alg::flow::MaximumFlowAlgorithm<G::Vertex, G::Edge>
            for $name<'g, G>
        {
            fn maximum_flow(
                &mut self,
                source: &G::Vertex,
                sink: &G::Vertex,
            ) -> $crate::error::Result<$crate::alg::flow::MaximumFlow<G::Edge>> {
                self.calculate(source, sink)?;
                let flow_map = self.net.compose_flow();
                self.net.max_flow = Some(flow_map.clone());
                Ok($crate::alg::flow::MaximumFlow {
                    value: self.net.max_flow_value,
                    flow_map,
                })
            }

            fn flow_map(
                &mut self,
            ) -> $crate::error::Result<&std::collections::HashMap<G::Edge, f64>> {
                Ok(self.net.flow_map())
            }

            fn flow_direction(&self, edge: &G::Edge) -> $crate::error::Result<G::Vertex> {
                self.net.flow_direction(edge)
            }
        }

        impl<'g, G: $bound> $crate::alg::flow::MinimumSTCutAlgorithm<G::Vertex, G::Edge>
            for $name<'g, G>
        {
            fn calculate_min_cut(
                &mut self,
                source: &G::Vertex,
                sink: &G::Vertex,
            ) -> $crate::error::Result<f64> {
                use $crate::alg::flow::MaximumFlowAlgorithm;
                self.maximum_flow_value(source, sink)
            }

            fn cut_capacity(&mut self) -> f64 {
                self.net.max_flow_value
            }

            fn source_partition(&mut self) -> &indexmap::IndexSet<G::Vertex> {
                self.net.source_partition()
            }

            fn sink_partition(&mut self) -> &indexmap::IndexSet<G::Vertex> {
                self.net.sink_partition()
            }

            fn cut_edges(&mut self) -> &indexmap::IndexSet<G::Edge> {
                self.net.cut_edges()
            }
        }
    };
}
pub(crate) use flow_algorithm_api;
