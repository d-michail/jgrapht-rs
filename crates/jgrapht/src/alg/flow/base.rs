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

use crate::error::{GraphError, Result};
use crate::graph::GraphExt;
use crate::graph::OutgoingGraph;
use crate::util::ToleranceDoubleComparator;
use indexmap::IndexSet;
use std::borrow::Borrow;
use std::collections::{HashMap, VecDeque};

/// `MaximumFlowAlgorithm.MaximumFlow` together with the minimum cut that certifies it: everything
/// one run of a maximum-flow algorithm produced, owned, so it stays valid after the algorithm
/// object is reused or dropped.
///
/// By the max-flow min-cut theorem the flow value is the capacity of the cut between
/// [`MaximumFlow::source_partition`] and [`MaximumFlow::sink_partition`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "V: serde::Serialize, E: serde::Serialize",
        deserialize = "V: serde::Deserialize<'de> + Eq + std::hash::Hash, E: serde::Deserialize<'de> + Eq + std::hash::Hash"
    ))
)]
#[derive(Debug, Clone)]
pub struct MaximumFlow<V, E> {
    value: f64,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_support::hash_pairs"))]
    flow_map: HashMap<E, f64>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_support::hash_pairs"))]
    direction: HashMap<E, V>,
    source_partition: IndexSet<V>,
    sink_partition: IndexSet<V>,
    cut_edges: IndexSet<E>,
}

impl<V: Clone + Eq + std::hash::Hash, E: Clone + Eq + std::hash::Hash> MaximumFlow<V, E> {
    /// The value of the flow, which equals [`MaximumFlow::cut_capacity`].
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Flow on `edge`, if it is an edge of the network.
    pub fn flow(&self, edge: &E) -> Option<f64> {
        self.flow_map.get(edge).copied()
    }

    /// The flow on every edge of the network.
    pub fn flow_map(&self) -> &HashMap<E, f64> {
        &self.flow_map
    }

    /// The endpoint of `edge` the flow runs towards (the target of a directed edge); `None` if
    /// `edge` is not an edge of the network.
    pub fn flow_direction(&self, edge: &E) -> Option<&V> {
        self.direction.get(edge)
    }

    /// Capacity of the minimum cut; equal to the flow value.
    pub fn cut_capacity(&self) -> f64 {
        self.value
    }

    /// Vertices on the source side of the minimum cut found by the run.
    pub fn source_partition(&self) -> &IndexSet<V> {
        &self.source_partition
    }

    /// Vertices on the sink side of the minimum cut.
    pub fn sink_partition(&self) -> &IndexSet<V> {
        &self.sink_partition
    }

    /// Edges crossing the minimum cut.
    pub fn cut_edges(&self) -> &IndexSet<E> {
        &self.cut_edges
    }

    /// The minimum cut of this run, without the flow.
    pub fn into_minimum_cut(self) -> MinimumCut<V, E> {
        MinimumCut {
            capacity: self.value,
            source_partition: self.source_partition,
            sink_partition: self.sink_partition,
            cut_edges: self.cut_edges,
        }
    }
}

/// A minimum $s$-$t$ cut: a partition of the vertices into a source side and a sink side, the edges
/// crossing it and its capacity.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "V: serde::Serialize, E: serde::Serialize",
        deserialize = "V: serde::Deserialize<'de> + Eq + std::hash::Hash, E: serde::Deserialize<'de> + Eq + std::hash::Hash"
    ))
)]
#[derive(Debug, Clone)]
pub struct MinimumCut<V, E> {
    capacity: f64,
    source_partition: IndexSet<V>,
    sink_partition: IndexSet<V>,
    cut_edges: IndexSet<E>,
}

impl<V, E> MinimumCut<V, E> {
    /// Builds a cut from its parts (used by algorithms that derive a cut without a flow).
    pub(crate) fn new(
        capacity: f64,
        source_partition: IndexSet<V>,
        sink_partition: IndexSet<V>,
        cut_edges: IndexSet<E>,
    ) -> Self {
        MinimumCut {
            capacity,
            source_partition,
            sink_partition,
            cut_edges,
        }
    }

    /// Sum of the capacities of the edges from the source side to the sink side.
    pub fn capacity(&self) -> f64 {
        self.capacity
    }

    /// Vertices on the source side of the cut.
    pub fn source_partition(&self) -> &IndexSet<V> {
        &self.source_partition
    }

    /// Vertices on the sink side of the cut.
    pub fn sink_partition(&self) -> &IndexSet<V> {
        &self.sink_partition
    }

    /// Edges crossing the cut.
    pub fn cut_edges(&self) -> &IndexSet<E> {
        &self.cut_edges
    }
}

/// An algorithm that can report the value of a maximum flow between two vertices.
///
/// Equivalent-flow trees implement only this trait: they answer all-pairs flow *values* but never
/// produce the flow on individual edges, which [`MaximumFlowAlgorithm`] promises.
pub trait MaximumFlowValueAlgorithm<V> {
    /// The value of a maximum flow from `source` to `sink`.
    ///
    /// # Errors
    ///
    /// [`GraphError::VertexNotFound`] if either vertex is not in the network, and
    /// [`GraphError::SourceEqualsSink`] where the algorithm requires distinct terminals.
    fn maximum_flow_value(&mut self, source: &V, sink: &V) -> Result<f64>;
}

/// Allows to derive [maximum-flow](https://en.wikipedia.org/wiki/Maximum_flow_problem)
/// from the supplied [flow network](https://en.wikipedia.org/wiki/Flow_network)
///
/// # Rust notes
///
/// Every run returns a complete [`MaximumFlow`]; nothing about the run is stored for later
/// queries, so there is no call order to get wrong.
pub trait MaximumFlowAlgorithm<V, E>: MaximumFlowValueAlgorithm<V> {
    /// Computes a maximum flow from `source` to `sink`, together with its minimum cut.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::SourceEqualsSink`](crate::GraphError::SourceEqualsSink) when both vertices coincide, [`GraphError::VertexNotFound`](crate::GraphError::VertexNotFound) when one is missing, and an `InvalidWeight` error for a negative or NaN capacity.
    fn maximum_flow(&mut self, source: &V, sink: &V) -> Result<MaximumFlow<V, E>>;
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
/// `MinimumSTCutAlgorithm`: each call returns the whole [`MinimumCut`].
pub trait MinimumSTCutAlgorithm<V, E> {
    /// Computes a minimum cut between `source` and `sink`.
    ///
    /// # Errors
    ///
    /// Fails exactly when [`MaximumFlowAlgorithm::maximum_flow`] does: equal or missing terminals, or an invalid capacity.
    fn minimum_cut(&mut self, source: &V, sink: &V) -> Result<MinimumCut<V, E>>;
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
    pub vs: Vec<G::Vertex>,
    vindex: HashMap<G::Vertex, usize>,
    es: Vec<G::Edge>,
    eindex: HashMap<G::Edge, usize>,
    pub arcs: Vec<Arc>,
    /// `VertexExtensionBase.outgoing`
    pub outgoing: Vec<Vec<usize>>,
    /// `VertexExtensionBase.excess`
    pub excess: Vec<f64>,
    /// the arc of every network edge, by edge index
    edge_arc: Vec<usize>,
    pub source: usize,
    pub sink: usize,
    /// Flow pushed so far by the current computation.
    pub max_flow_value: f64,
}

impl<'g, G: OutgoingGraph> FlowNet<'g, G> {
    pub fn new(graph: &'g G, epsilon: f64) -> Result<Self> {
        let vs: Vec<G::Vertex> = graph.vertices_cloned().collect();
        let es: Vec<G::Edge> = graph.edges_cloned().collect();
        let net = FlowNet {
            graph,
            directed: graph.graph_type().directed,
            comparator: ToleranceDoubleComparator::new(epsilon)?,
            vindex: vs.iter().enumerate().map(|(i, v)| (v.clone(), i)).collect(),
            eindex: es.iter().enumerate().map(|(i, e)| (e.clone(), i)).collect(),
            vs,
            es,
            arcs: Vec::new(),
            outgoing: Vec::new(),
            excess: Vec::new(),
            edge_arc: Vec::new(),
            source: 0,
            sink: 0,
            max_flow_value: 0.0,
        };
        Ok(net)
    }

    /// The validation shared by the constructors: capacities must not be negative or NaN. Callers whose networks legitimately
    /// carry negative arcs (Goldberg's density network) skip it.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if an edge weight is negative, NaN or otherwise unusable; errors of the underlying graph operations are propagated.
    pub fn check_capacities(&self, epsilon: f64) -> Result<()> {
        for e in &self.es {
            let capacity = self.weight(self.eindex[e]);
            if capacity.is_nan() {
                return Err(GraphError::InvalidWeight(crate::WeightProblem::NaN));
            }
            if capacity < -epsilon {
                return Err(GraphError::InvalidWeight(crate::WeightProblem::Negative));
            }
        }
        Ok(())
    }

    pub fn vertex_index(&self, v: &G::Vertex) -> Option<usize> {
        self.vindex.get(v).copied()
    }

    /// Checks that `source` and `sink` are distinct vertices of the network.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if a vertex argument is not in the graph; source and sink coincide; errors of the underlying graph operations are propagated.
    pub fn resolve(&self, source: &G::Vertex, sink: &G::Vertex) -> Result<(usize, usize)> {
        let s = self
            .vertex_index(source)
            .ok_or(GraphError::VertexNotFound)?;
        let t = self.vertex_index(sink).ok_or(GraphError::VertexNotFound)?;
        if s == t {
            return Err(GraphError::SourceEqualsSink);
        }
        Ok((s, t))
    }

    fn weight(&self, e: usize) -> f64 {
        self.graph.weight_of(&self.es[e])
    }

    /// `init` + `buildInternal`: rebuilds the residual network for a new computation.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
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
        Ok(())
    }

    fn build_internal(&mut self) -> Result<()> {
        let g = self.graph;
        if self.directed {
            for u in 0..self.vs.len() {
                let vertex = self.vs[u].clone();
                for e in g.out_edges_of(&vertex) {
                    let ei = self.eindex[e];
                    let v = self.vindex[g.edge_target(e).ok_or(crate::GraphError::EdgeNotFound)?];
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
                let e = &self.es[ei];
                let u = self.vindex[g.edge_source(e).ok_or(crate::GraphError::EdgeNotFound)?];
                let v = self.vindex[g.edge_target(e).ok_or(crate::GraphError::EdgeNotFound)?];
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
            .get_edge(&self.vs[ft], &self.vs[fs])
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

    /// The endpoint of `e` the flow runs towards.
    fn direction_of(&self, ei: usize) -> G::Vertex {
        let a = &self.arcs[self.edge_arc[ei]];
        if self.directed {
            return self.vs[a.target].clone();
        }
        let inverse = &self.arcs[a.inverse];
        if a.flow > inverse.flow {
            self.vs[a.target].clone()
        } else {
            self.vs[inverse.target].clone()
        }
    }

    /// Vertices reachable from the source through arcs with residual capacity.
    fn residual_source_side(&self) -> IndexSet<G::Vertex> {
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
        partition
    }

    /// Everything the finished computation produced: the flow, its direction on every edge, and
    /// the minimum cut given by the residual network.
    pub fn finish(&self) -> MaximumFlow<G::Vertex, G::Edge> {
        let source_partition = self.residual_source_side();
        let sink_partition: IndexSet<G::Vertex> = self
            .vs
            .iter()
            .filter(|v| !source_partition.contains(*v))
            .cloned()
            .collect();
        let g = self.graph;
        let mut cut_edges = IndexSet::new();
        for e in g.edges() {
            let e = e.borrow();
            let (Some(s), Some(t)) = (g.edge_source(e), g.edge_target(e)) else {
                continue;
            };
            let (s_in, t_in) = (source_partition.contains(s), source_partition.contains(t));
            let crossing = if self.directed {
                s_in && !t_in
            } else {
                s_in ^ t_in
            };
            if crossing {
                cut_edges.insert(e.clone());
            }
        }
        MaximumFlow {
            value: self.max_flow_value,
            flow_map: self.compose_flow(),
            direction: (0..self.es.len())
                .map(|ei| (self.es[ei].clone(), self.direction_of(ei)))
                .collect(),
            source_partition,
            sink_partition,
            cut_edges,
        }
    }
}

/// Implements [`MaximumFlowAlgorithm`] and [`MinimumSTCutAlgorithm`] for an algorithm struct with a
/// `net: FlowNet` field and a `calculate(&mut self, &V, &V) -> Result<f64>` method.
macro_rules! flow_algorithm_api {
    ($name:ident, $bound:path) => {
        impl<'g, G: $bound> $crate::alg::flow::MaximumFlowValueAlgorithm<G::Vertex>
            for $name<'g, G>
        {
            fn maximum_flow_value(
                &mut self,
                source: &G::Vertex,
                sink: &G::Vertex,
            ) -> $crate::error::Result<f64> {
                self.calculate(source, sink)
            }
        }

        impl<'g, G: $bound> $crate::alg::flow::MaximumFlowAlgorithm<G::Vertex, G::Edge>
            for $name<'g, G>
        {
            fn maximum_flow(
                &mut self,
                source: &G::Vertex,
                sink: &G::Vertex,
            ) -> $crate::error::Result<$crate::alg::flow::MaximumFlow<G::Vertex, G::Edge>> {
                self.calculate(source, sink)?;
                Ok(self.net.finish())
            }
        }

        impl<'g, G: $bound> $crate::alg::flow::MinimumSTCutAlgorithm<G::Vertex, G::Edge>
            for $name<'g, G>
        {
            fn minimum_cut(
                &mut self,
                source: &G::Vertex,
                sink: &G::Vertex,
            ) -> $crate::error::Result<$crate::alg::flow::MinimumCut<G::Vertex, G::Edge>> {
                use $crate::alg::flow::MaximumFlowAlgorithm;
                Ok(self.maximum_flow(source, sink)?.into_minimum_cut())
            }
        }
    };
}
pub(crate) use flow_algorithm_api;
