// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

//! Densest subgraph (density = edges / vertices, direction and multiplicity ignored).

use super::flow::{MinimumStCutAlgorithm, PushRelabel};
use super::util::Indexed;
use crate::Result;
use crate::graph::OutgoingGraph;
use crate::graph::{Graph, GraphExt, GraphMut};
use std::borrow::Borrow;
use std::collections::HashSet;

fn simple_edges<G: Graph>(ix: &Indexed<'_, G>) -> Vec<(usize, usize)> {
    let set: HashSet<(usize, usize)> = ix
        .ends
        .iter()
        .filter(|&&(a, b)| a != b)
        .map(|&(a, b)| (a.min(b), a.max(b)))
        .collect();
    let mut v: Vec<_> = set.into_iter().collect();
    v.sort_unstable();
    v
}

/// Charikar's greedy peeling: a 1/2-approximation of the densest subgraph.
pub fn greedy_densest_subgraph<G: Graph>(g: &G) -> (Vec<G::Vertex>, f64) {
    let ix = Indexed::new(g);
    let n = ix.n();
    let edges = simple_edges(&ix);
    let mut adj = vec![HashSet::new(); n];
    for &(a, b) in &edges {
        adj[a].insert(b);
        adj[b].insert(a);
    }
    let mut alive = vec![true; n];
    let (mut m, mut count) = (edges.len(), n);
    let (mut best, mut best_set) = (if n > 0 { m as f64 / n as f64 } else { 0.0 }, alive.clone());
    while count > 1 {
        let v = (0..n)
            .filter(|&v| alive[v])
            .min_by_key(|&v| adj[v].iter().filter(|&&u| alive[u]).count())
            .unwrap();
        m -= adj[v].iter().filter(|&&u| alive[u]).count();
        alive[v] = false;
        count -= 1;
        let d = m as f64 / count as f64;
        if d > best {
            best = d;
            best_set = alive.clone();
        }
    }
    (
        (0..n)
            .filter(|&v| best_set[v])
            .map(|v| ix.vertex(v))
            .collect(),
        best,
    )
}

/// The directed multigraph Goldberg's algorithm builds its flow networks in.
pub type DensityNetwork<V> = crate::BaseGraph<V, crate::DefaultEdge, crate::graph::Directed>;

/// Computes the source side of a minimum `s`-`t` cut of a [`DensityNetwork`] (edge weights are
/// the capacities). This plays the role of JGraphT's `MinimumStCutAlgorithm` factory argument.
pub type MinCutFn<V> = Box<dyn Fn(&DensityNetwork<V>, &V, &V) -> Result<HashSet<V>>>;

/// The default min-cut used by the Goldberg algorithms (max-flow based).
pub fn default_min_cut<V: Clone + Eq + std::hash::Hash + 'static>() -> MinCutFn<V> {
    Box::new(|net, s, t| {
        let mut push_relabel = PushRelabel::new_allowing_negative_capacities(net)?;
        let cut = push_relabel.minimum_cut(s, t)?;
        Ok(cut.source_partition().iter().cloned().collect())
    })
}

enum DensityKind<V> {
    /// density = sum of edge weights / number of vertices
    EdgeWeights,
    /// density = (sum of edge weights + sum of vertex weights) / number of vertices
    NodeWeights(Box<dyn Fn(&V) -> f64>),
    /// density = sum of edge weights / sum of vertex weights
    NodeWeightPerEdgeWeight(Box<dyn Fn(&V) -> f64>),
}

/// Goldberg's maximum density subgraph algorithm (`GoldbergMaximumDensitySubgraphAlgorithm` and
/// its node-weight variants): binary search on the density, each step a minimum cut.
///
/// `s` and `t` are two extra vertices (not in the graph) used as source and sink of the network.
pub struct GoldbergMaximumDensitySubgraph<'g, G: Graph>
where
    G::Vertex: 'static,
{
    graph: &'g G,
    kind: DensityKind<G::Vertex>,
    lower: f64,
    upper: f64,
    epsilon: f64,
    guess: f64,
    check_weights: bool,
    network: DensityNetwork<G::Vertex>,
    s: G::Vertex,
    t: G::Vertex,
    min_cut: MinCutFn<G::Vertex>,
    current_vertices: HashSet<G::Vertex>,
    /// `Some` once the densest subgraph is known
    densest: Option<HashSet<G::Vertex>>,
}

impl<'g, G: OutgoingGraph> GoldbergMaximumDensitySubgraph<'g, G>
where
    G::Vertex: 'static,
{
    /// Edge density: sum of edge weights over the number of vertices.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn new(graph: &'g G, s: G::Vertex, t: G::Vertex, epsilon: f64) -> Result<Self> {
        Self::build(
            graph,
            s,
            t,
            DensityKind::EdgeWeights,
            false,
            epsilon,
            default_min_cut(),
        )
    }

    /// Edge density plus vertex weights `node_weight`.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn with_node_weights(
        graph: &'g G,
        s: G::Vertex,
        t: G::Vertex,
        epsilon: f64,
        node_weight: impl Fn(&G::Vertex) -> f64 + 'static,
    ) -> Result<Self> {
        Self::build(
            graph,
            s,
            t,
            DensityKind::NodeWeights(Box::new(node_weight)),
            true,
            epsilon,
            default_min_cut(),
        )
    }

    /// Sum of edge weights per unit of vertex weight.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn with_node_weight_per_edge_weight(
        graph: &'g G,
        s: G::Vertex,
        t: G::Vertex,
        epsilon: f64,
        node_weight: impl Fn(&G::Vertex) -> f64 + 'static,
    ) -> Result<Self> {
        Self::build(
            graph,
            s,
            t,
            DensityKind::NodeWeightPerEdgeWeight(Box::new(node_weight)),
            true,
            epsilon,
            default_min_cut(),
        )
    }

    /// Replaces the minimum cut routine.
    pub fn with_min_cut(mut self, min_cut: MinCutFn<G::Vertex>) -> Self {
        self.min_cut = min_cut;
        self
    }

    fn build(
        graph: &'g G,
        s: G::Vertex,
        t: G::Vertex,
        kind: DensityKind<G::Vertex>,
        check_weights: bool,
        epsilon: f64,
        min_cut: MinCutFn<G::Vertex>,
    ) -> Result<Self> {
        if graph.contains_vertex(&s) || graph.contains_vertex(&t) {
            return Err(crate::GraphError::InvalidParameter {
                name: "source_and_sink",
                requirement: crate::ParameterRequirement::Consistent,
            });
        }
        let mut alg = GoldbergMaximumDensitySubgraph {
            graph,
            kind,
            lower: 0.0,
            upper: 0.0,
            epsilon,
            guess: 0.0,
            check_weights,
            network: DensityNetwork::<G::Vertex>::pseudo().weighted(),
            s,
            t,
            min_cut,
            current_vertices: HashSet::new(),
            densest: None,
        };
        alg.upper = alg.numerator(graph);
        alg.initialize_network()?;
        // an empty denominator means the whole (empty) graph is the solution
        if alg.denominator(graph) == 0.0 {
            alg.densest = Some(graph.vertices_cloned().collect());
        }
        Ok(alg)
    }

    fn weight_of<H: Graph<Vertex = G::Vertex>>(g: &H, e: &H::Edge) -> f64 {
        g.weight_of(e)
    }

    fn numerator<H: Graph<Vertex = G::Vertex>>(&self, g: &H) -> f64 {
        let edges: f64 = g
            .edges()
            .map(|e| {
                let e = e.borrow();
                Self::weight_of(g, e)
            })
            .sum();
        match &self.kind {
            DensityKind::NodeWeights(w) => edges + g.vertices().map(|v| w(v.borrow())).sum::<f64>(),
            _ => edges,
        }
    }

    fn denominator<H: Graph<Vertex = G::Vertex>>(&self, g: &H) -> f64 {
        match &self.kind {
            DensityKind::NodeWeightPerEdgeWeight(w) => g.vertices().map(|v| w(v.borrow())).sum(),
            _ => g.vertex_count() as f64,
        }
    }

    fn outgoing_weight(&self, v: &G::Vertex) -> f64 {
        self.graph
            .out_edges_of(v)
            .map(|e| Self::weight_of(self.graph, e))
            .sum()
    }

    fn source_weight(&self, _v: &G::Vertex) -> f64 {
        match &self.kind {
            DensityKind::EdgeWeights => self.graph.edge_count() as f64,
            _ => 0.0,
        }
    }

    fn sink_weight(&self, v: &G::Vertex) -> f64 {
        match &self.kind {
            DensityKind::EdgeWeights => {
                self.graph.edge_count() as f64 + 2.0 * self.guess - self.outgoing_weight(v)
            }
            DensityKind::NodeWeights(w) => 2.0 * self.guess - self.outgoing_weight(v) - 2.0 * w(v),
            DensityKind::NodeWeightPerEdgeWeight(w) => {
                2.0 * self.guess * w(v) - self.outgoing_weight(v)
            }
        }
    }

    fn initialize_network(&mut self) -> Result<()> {
        let (s, t) = (self.s.clone(), self.t.clone());
        self.network.add_vertex(s.clone());
        self.network.add_vertex(t.clone());
        for v in self.graph.vertices() {
            let v = v.borrow();
            self.network.add_vertex(v.clone());
            self.network.add_edge(&s, v)?;
            self.network.add_edge(v, &t)?;
        }
        for e in self.graph.edges() {
            let e = e.borrow();
            let (a, b) = (
                self.graph
                    .edge_source(e)
                    .ok_or(crate::GraphError::EdgeNotFound)?
                    .clone(),
                self.graph
                    .edge_target(e)
                    .ok_or(crate::GraphError::EdgeNotFound)?
                    .clone(),
            );
            let weight = Self::weight_of(self.graph, e);
            for (x, y) in [(&a, &b), (&b, &a)] {
                if let Some(edge) = self.network.add_edge(x, y)? {
                    self.network.set_edge_weight(&edge, weight)?;
                }
            }
        }
        Ok(())
    }

    fn update_network(&mut self) -> Result<()> {
        let vertices: Vec<G::Vertex> = self.graph.vertices_cloned().collect();
        for v in &vertices {
            let (sw, tw) = (self.source_weight(v), self.sink_weight(v));
            let to_sink = self.network.edge_between(v, &self.t).cloned().unwrap();
            self.network.set_edge_weight(&to_sink, tw)?;
            let from_source = self.network.edge_between(&self.s, v).cloned().unwrap();
            self.network.set_edge_weight(&from_source, sw)?;
        }
        if self.check_weights {
            // all capacities must be non-negative: shift them by the minimum
            let min = vertices
                .iter()
                .flat_map(|v| {
                    [
                        self.network.edge_between(v, &self.t).cloned(),
                        self.network.edge_between(&self.s, v).cloned(),
                    ]
                })
                .flatten()
                .map(|e| self.network.edge_weight(&e).unwrap())
                .fold(None, |m: Option<f64>, w| Some(m.map_or(w, |m| m.min(w))))
                .unwrap_or(0.0);
            if min < 0.0 {
                for v in &vertices {
                    for e in [
                        self.network.edge_between(v, &self.t).cloned().unwrap(),
                        self.network.edge_between(&self.s, v).cloned().unwrap(),
                    ] {
                        let w = self
                            .network
                            .edge_weight(&e)
                            .ok_or(crate::GraphError::EdgeNotFound)?;
                        self.network.set_edge_weight(&e, w - min)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// The densest subgraph (induced).
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::Internal`](crate::GraphError::Internal) if an internal invariant of the library is broken (a bug).
    /// Errors of the graph operations it performs are propagated.
    pub fn densest_subgraph(&mut self) -> Result<crate::AsSubgraph<&'g G>> {
        if self.densest.is_none() {
            while self.upper - self.lower >= self.epsilon {
                self.guess = self.lower + (self.upper - self.lower) / 2.0;
                self.update_network()?;
                let mut source_partition = (self.min_cut)(&self.network, &self.s, &self.t)?;
                source_partition.remove(&self.s);
                if source_partition.is_empty() {
                    self.upper = self.guess;
                } else {
                    self.lower = self.guess;
                    self.current_vertices = source_partition;
                }
            }
            self.densest = Some(std::mem::take(&mut self.current_vertices));
        }
        Ok(crate::AsSubgraph::induced(
            self.graph,
            self.densest
                .as_ref()
                .ok_or(crate::GraphError::Internal(
                    "a value computed earlier is missing",
                ))?
                .iter()
                .cloned(),
        ))
    }

    /// Density of the densest subgraph.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn density(&mut self) -> Result<f64> {
        let sub = self.densest_subgraph()?;
        let denominator = self.denominator(&sub);
        Ok(if denominator != 0.0 {
            self.numerator(&sub) / denominator
        } else {
            0.0
        })
    }
}
