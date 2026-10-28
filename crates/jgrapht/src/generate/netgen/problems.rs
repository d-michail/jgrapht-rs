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

use super::config::NetworkGeneratorConfig;
use crate::graph::GraphExt;
use crate::graph::GraphMut;
use crate::{GraphError, Result};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Capacity given to the arcs added by [`MaximumFlowProblem::to_single_source_single_sink_problem`].
pub const CAPACITY_INF: f64 = i32::MAX as f64;

/// Classification of the generated vertices, in the order the generator created them.
#[derive(Debug, Clone)]
pub struct NetworkInfo<V, E> {
    pub(crate) config: NetworkGeneratorConfig,
    pub(crate) vertices: Vec<V>,
    pub(crate) skeleton_arcs: Vec<E>,
}

impl<V, E> NetworkInfo<V, E> {
    pub(crate) fn new(config: NetworkGeneratorConfig) -> Self {
        NetworkInfo {
            config,
            vertices: Vec::new(),
            skeleton_arcs: Vec::new(),
        }
    }
    fn range(&self, from: i32, to: i32) -> &[V] {
        &self.vertices[from as usize..to as usize]
    }
    /// The sources that are not transshipment sources.
    pub fn pure_sources(&self) -> &[V] {
        self.range(0, self.config.pure_source_num())
    }
    /// The sources that also pass flow on from other nodes.
    pub fn transshipment_sources(&self) -> &[V] {
        self.range(self.config.pure_source_num(), self.config.source_num())
    }
    /// All sources: the pure sources followed by the transshipment sources.
    pub fn sources(&self) -> &[V] {
        self.range(0, self.config.source_num())
    }
    /// The nodes that are neither sources nor sinks.
    pub fn transshipment_nodes(&self) -> &[V] {
        self.range(
            self.config.source_num(),
            self.config.source_num() + self.config.transship_node_num(),
        )
    }
    /// The sinks that are not transshipment sinks.
    pub fn pure_sinks(&self) -> &[V] {
        self.range(
            self.config.node_num() - self.config.pure_sink_num(),
            self.config.node_num(),
        )
    }
    /// The sinks that also send flow on to other nodes.
    pub fn transshipment_sinks(&self) -> &[V] {
        self.range(
            self.config.node_num() - self.config.sink_num(),
            self.config.node_num() - self.config.pure_sink_num(),
        )
    }
    /// All sinks: the transshipment sinks followed by the pure sinks.
    pub fn sinks(&self) -> &[V] {
        self.range(
            self.config.node_num() - self.config.sink_num(),
            self.config.node_num(),
        )
    }
    /// Arcs of the initial chains / chain-to-sink skeleton.
    pub fn skeleton_arcs(&self) -> &[E] {
        &self.skeleton_arcs
    }
}

/// Bipartite matching / assignment problem: partitions are the sources and the sinks.
#[derive(Debug, Clone)]
pub struct BipartiteMatchingProblem<V, E> {
    /// The vertices of the first side.
    pub partition1: HashSet<V>,
    /// The vertices of the second side.
    pub partition2: HashSet<V>,
    /// Cost of every arc.
    pub costs: HashMap<E, f64>,
    /// Whether the arc costs matter; `false` for the unweighted matching problem.
    pub weighted: bool,
}

impl<V: Eq + Hash + Clone, E: Eq + Hash + Clone> BipartiteMatchingProblem<V, E> {
    /// Cost of the arc `e`.
    pub fn cost(&self, e: &E) -> f64 {
        self.costs[e]
    }
    /// Whether the costs are meaningful; `false` for the unweighted matching problem.
    pub fn is_weighted(&self) -> bool {
        self.weighted
    }
    /// Stores the costs as the edge weights of `graph`.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn dump_costs<G: GraphMut<Vertex = V, Edge = E>>(&self, graph: &mut G) -> Result<()> {
        let edges: Vec<E> = graph.edges_cloned().collect();
        for e in edges {
            graph.set_edge_weight(&e, self.costs[&e])?;
        }
        Ok(())
    }
}

/// Maximum flow problem with (possibly several) sources and sinks.
#[derive(Debug, Clone)]
pub struct MaximumFlowProblem<V, E> {
    /// The vertices that supply flow.
    pub sources: HashSet<V>,
    /// The vertices that absorb flow.
    pub sinks: HashSet<V>,
    /// Capacity of every arc.
    pub capacities: HashMap<E, f64>,
}

impl<V: Eq + Hash + Clone, E: Eq + Hash + Clone> MaximumFlowProblem<V, E> {
    /// Capacity of the arc `e`.
    pub fn capacity(&self, e: &E) -> f64 {
        self.capacities[e]
    }
    /// Returns the source.
    pub fn source(&self) -> &V {
        self.sources.iter().next().expect("no source")
    }
    /// Returns the sink.
    pub fn sink(&self) -> &V {
        self.sinks.iter().next().expect("no sink")
    }
    /// Whether there is exactly one source and one sink.
    pub fn is_single_source_single_sink_problem(&self) -> bool {
        self.sources.len() == 1 && self.sinks.len() == 1
    }

    /// Stores the capacities as the edge weights of `graph`.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn dump_capacities<G: GraphMut<Vertex = V, Edge = E>>(&self, graph: &mut G) -> Result<()> {
        let edges: Vec<E> = graph.edges_cloned().collect();
        for e in edges {
            graph.set_edge_weight(&e, self.capacities[&e])?;
        }
        Ok(())
    }

    /// Adds a super source / super sink to `graph` when there are several, connected with
    /// [`CAPACITY_INF`] arcs.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn to_single_source_single_sink_problem<G: GraphMut<Vertex = V, Edge = E>>(
        &self,
        graph: &mut G,
    ) -> Result<MaximumFlowProblem<V, E>> {
        let mut capacities = self.capacities.clone();
        let sources = Self::convert(graph, &self.sources, &mut capacities, true)?;
        let sinks = Self::convert(graph, &self.sinks, &mut capacities, false)?;
        Ok(MaximumFlowProblem {
            sources,
            sinks,
            capacities,
        })
    }

    fn convert<G: GraphMut<Vertex = V, Edge = E>>(
        graph: &mut G,
        vertices: &HashSet<V>,
        capacities: &mut HashMap<E, f64>,
        sources: bool,
    ) -> Result<HashSet<V>> {
        if vertices.len() == 1 {
            return Ok(vertices.clone());
        }
        let super_vertex = graph.add_new_vertex()?;
        for v in vertices {
            let edge = if sources {
                graph.add_edge(&super_vertex, v)?
            } else {
                graph.add_edge(v, &super_vertex)?
            };
            let edge = edge.ok_or(GraphError::Internal("arc not added"))?;
            capacities.insert(edge, CAPACITY_INF);
        }
        Ok(HashSet::from([super_vertex]))
    }
}
