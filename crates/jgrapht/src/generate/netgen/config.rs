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

/// Immutable parameters of a generated network; built with
/// [`NetworkGeneratorConfigBuilder`](super::NetworkGeneratorConfigBuilder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkGeneratorConfig {
    pub(crate) node_count: i32,
    pub(crate) arc_count: i32,
    pub(crate) source_count: i32,
    pub(crate) sink_count: i32,
    pub(crate) transship_source_count: i32,
    pub(crate) transship_sink_count: i32,
    pub(crate) total_supply: i32,
    pub(crate) min_cap: i32,
    pub(crate) max_cap: i32,
    pub(crate) min_cost: i32,
    pub(crate) max_cost: i32,
    pub(crate) percent_capacitated: i32,
    pub(crate) percent_with_inf_cost: i32,
}

impl NetworkGeneratorConfig {
    /// Largest number of arcs from the pure sources to the transshipment sources (every pure source
    /// to every transshipment source, plus all arcs among the transshipment sources).
    pub fn max_source_to_tsource_arc_count(&self) -> i64 {
        self.pure_source_count() as i64 * self.transship_source_count as i64
            + self.transship_source_count as i64 * (self.transship_source_count as i64 - 1)
    }
    /// Largest number of arcs from the sources to the transshipment nodes.
    pub fn max_source_to_tnode_arc_count(&self) -> i64 {
        self.source_count as i64 * self.transship_node_count() as i64
    }
    /// Largest number of arcs from the sources directly to the sinks.
    pub fn max_source_to_sink_arc_count(&self) -> i64 {
        self.source_count as i64 * self.sink_count as i64
    }
    /// Largest number of arcs from the transshipment nodes to the transshipment sources.
    pub fn max_tnode_to_tsource_arc_count(&self) -> i64 {
        self.transship_node_count() as i64 * self.transship_source_count as i64
    }
    /// Largest number of arcs between distinct transshipment nodes.
    pub fn max_tnode_to_tnode_arc_count(&self) -> i64 {
        self.transship_node_count() as i64 * (self.transship_node_count() as i64 - 1)
    }
    /// Largest number of arcs from the transshipment nodes to the sinks.
    pub fn max_tnode_to_sink_arc_count(&self) -> i64 {
        self.transship_node_count() as i64 * self.sink_count as i64
    }
    /// Largest number of arcs from the transshipment sinks to the transshipment sources.
    pub fn max_tsink_to_tsource_arc_count(&self) -> i64 {
        self.transship_sink_count as i64 * self.transship_source_count as i64
    }
    /// Largest number of arcs from the transshipment sinks to the transshipment nodes.
    pub fn max_tsink_to_tnode_arc_count(&self) -> i64 {
        self.transship_sink_count as i64 * self.transship_node_count() as i64
    }
    /// Largest number of arcs from the transshipment sinks to the sinks.
    pub fn max_tsink_to_sink_arc_count(&self) -> i64 {
        // the second product is an `int` multiplication in JGraphT
        self.transship_sink_count as i64 * (self.transship_sink_count as i64 - 1)
            + self
                .pure_sink_count()
                .wrapping_mul(self.transship_sink_count) as i64
    }
    /// Largest number of arcs leaving the sources, whatever their target.
    pub fn max_source_to_all_arc_count(&self) -> i64 {
        self.max_source_to_tsource_arc_count()
            + self.max_source_to_tnode_arc_count()
            + self.max_source_to_sink_arc_count()
    }
    /// Largest number of arcs leaving the transshipment nodes, whatever their target.
    pub fn max_transship_node_to_all_arc_count(&self) -> i64 {
        self.max_tnode_to_tsource_arc_count()
            + self.max_tnode_to_tnode_arc_count()
            + self.max_tnode_to_sink_arc_count()
    }
    /// Largest number of arcs leaving the sinks, whatever their target.
    pub fn max_sink_to_all_arc_count(&self) -> i64 {
        self.max_tsink_to_tsource_arc_count()
            + self.max_tsink_to_tnode_arc_count()
            + self.max_tsink_to_sink_arc_count()
    }
    /// Smallest number of arcs a valid problem needs: one per transshipment node plus enough to attach
    /// every source and every sink (the larger of the two counts).
    pub fn minimum_arc_count(&self) -> i64 {
        self.transship_node_count() as i64 + self.source_count.max(self.sink_count) as i64
    }
    /// Largest number of arcs the node counts allow, summed over all source, transshipment and sink
    /// nodes.
    pub fn maximum_arc_count(&self) -> i64 {
        self.max_source_to_all_arc_count()
            + self.max_transship_node_to_all_arc_count()
            + self.max_sink_to_all_arc_count()
    }

    /// Smallest number of arcs of a valid network.
    pub fn minimum_arc_count_for(source_count: i64, tnode_count: i64, sink_count: i64) -> i64 {
        tnode_count + source_count.max(sink_count)
    }
    /// Largest number of arcs without transshipment sources / sinks.
    pub fn maximum_arc_count_for(source_count: i64, tnode_count: i64, sink_count: i64) -> i64 {
        Self::maximum_arc_count_full(source_count, 0, tnode_count, 0, sink_count)
    }
    /// [`maximum_arc_count`](Self::maximum_arc_count) computed from the node counts alone, without a
    /// configuration. The builder uses it to validate the requested arc count.
    pub fn maximum_arc_count_full(
        source_count: i64,
        tsource_count: i64,
        tnode_count: i64,
        tsink_count: i64,
        sink_count: i64,
    ) -> i64 {
        let pure_source_count = source_count - tsource_count;
        let source_arcs = pure_source_count * tsource_count
            + tsource_count * (tsource_count - 1)
            + source_count * (tnode_count + sink_count);
        let tnode_arcs = tnode_count * (tsource_count + (tnode_count - 1) + sink_count);
        let sink_arcs = tsink_count * (tsource_count + tnode_count + (sink_count - 1));
        source_arcs + tnode_arcs + sink_arcs
    }

    /// Number of sources that are not transshipment sources.
    pub fn pure_source_count(&self) -> i32 {
        self.source_count - self.transship_source_count
    }
    /// Number of sinks that are not transshipment sinks.
    pub fn pure_sink_count(&self) -> i32 {
        self.sink_count - self.transship_sink_count
    }
    /// Whether this describes a minimum-cost flow problem with unit capacities, one unit of supply
    /// per source and varying costs.
    pub fn is_cost_weighted(&self) -> bool {
        self.min_cost != self.max_cost
    }
    /// Number of transshipment nodes: the nodes that are neither sources nor sinks.
    pub fn transship_node_count(&self) -> i32 {
        self.node_count - self.source_count - self.sink_count
    }
    fn transportation_problem_condition(&self) -> bool {
        self.source_count + self.sink_count == self.node_count
            && self.transship_source_count == 0
            && self.transship_sink_count == 0
    }
    fn assignment_problem_condition(&self) -> bool {
        self.source_count == self.sink_count
            && self.total_supply == self.source_count
            && self.min_cap == 1
            && self.max_cap == 1
    }
    /// Whether this describes a maximum flow problem: every configuration that is not cost weighted.
    pub fn is_max_flow_problem(&self) -> bool {
        !self.is_cost_weighted()
    }
    /// Whether this describes an assignment problem, a transportation problem with one unit per node.
    pub fn is_assignment_problem(&self) -> bool {
        self.transportation_problem_condition() && self.assignment_problem_condition()
    }

    /// Total number of nodes.
    pub fn node_count(&self) -> i32 {
        self.node_count
    }
    /// Number of arcs to generate.
    pub fn arc_count(&self) -> i32 {
        self.arc_count
    }
    /// Number of sources (including the transshipment sources).
    pub fn source_count(&self) -> i32 {
        self.source_count
    }
    /// Number of sinks (including the transshipment sinks).
    pub fn sink_count(&self) -> i32 {
        self.sink_count
    }
    /// Number of sources that also receive flow from other nodes.
    pub fn transship_source_count(&self) -> i32 {
        self.transship_source_count
    }
    /// Number of sinks that also send flow to other nodes.
    pub fn transship_sink_count(&self) -> i32 {
        self.transship_sink_count
    }
    /// Total supply that the sources inject into the network.
    pub fn total_supply(&self) -> i32 {
        self.total_supply
    }
    /// Smallest arc capacity.
    pub fn min_cap(&self) -> i32 {
        self.min_cap
    }
    /// Largest arc capacity.
    pub fn max_cap(&self) -> i32 {
        self.max_cap
    }
    /// Smallest arc cost.
    pub fn min_cost(&self) -> i32 {
        self.min_cost
    }
    /// Largest arc cost.
    pub fn max_cost(&self) -> i32 {
        self.max_cost
    }
    /// Percentage of arcs that get a finite capacity; the others are uncapacitated.
    pub fn percent_capacitated(&self) -> i32 {
        self.percent_capacitated
    }
    /// Percentage of arcs with infinite cost.
    pub fn percent_with_inf_cost(&self) -> i32 {
        self.percent_with_inf_cost
    }
}
