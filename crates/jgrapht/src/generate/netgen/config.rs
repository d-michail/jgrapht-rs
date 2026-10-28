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
    pub(crate) node_num: i32,
    pub(crate) arc_num: i32,
    pub(crate) source_num: i32,
    pub(crate) sink_num: i32,
    pub(crate) transship_source_num: i32,
    pub(crate) transship_sink_num: i32,
    pub(crate) total_supply: i32,
    pub(crate) min_cap: i32,
    pub(crate) max_cap: i32,
    pub(crate) min_cost: i32,
    pub(crate) max_cost: i32,
    pub(crate) percent_capacitated: i32,
    pub(crate) percent_with_inf_cost: i32,
}

impl NetworkGeneratorConfig {
    /// Returns the max source to tsource arc num.
    pub fn max_source_to_tsource_arc_num(&self) -> i64 {
        self.pure_source_num() as i64 * self.transship_source_num as i64
            + self.transship_source_num as i64 * (self.transship_source_num as i64 - 1)
    }
    /// Returns the max source to tnode arc num.
    pub fn max_source_to_tnode_arc_num(&self) -> i64 {
        self.source_num as i64 * self.transship_node_num() as i64
    }
    /// Returns the max source to sink arc num.
    pub fn max_source_to_sink_arc_num(&self) -> i64 {
        self.source_num as i64 * self.sink_num as i64
    }
    /// Returns the max tnode to tsource arc num.
    pub fn max_tnode_to_tsource_arc_num(&self) -> i64 {
        self.transship_node_num() as i64 * self.transship_source_num as i64
    }
    /// Returns the max tnode to tnode arc num.
    pub fn max_tnode_to_tnode_arc_num(&self) -> i64 {
        self.transship_node_num() as i64 * (self.transship_node_num() as i64 - 1)
    }
    /// Returns the max tnode to sink arc num.
    pub fn max_tnode_to_sink_arc_num(&self) -> i64 {
        self.transship_node_num() as i64 * self.sink_num as i64
    }
    /// Returns the max tsink to tsource arc num.
    pub fn max_tsink_to_tsource_arc_num(&self) -> i64 {
        self.transship_sink_num as i64 * self.transship_source_num as i64
    }
    /// Returns the max tsink to tnode arc num.
    pub fn max_tsink_to_tnode_arc_num(&self) -> i64 {
        self.transship_sink_num as i64 * self.transship_node_num() as i64
    }
    /// Returns the max tsink to sink arc num.
    pub fn max_tsink_to_sink_arc_num(&self) -> i64 {
        // the second product is an `int` multiplication in JGraphT
        self.transship_sink_num as i64 * (self.transship_sink_num as i64 - 1)
            + self.pure_sink_num().wrapping_mul(self.transship_sink_num) as i64
    }
    /// Returns the max source to all arc num.
    pub fn max_source_to_all_arc_num(&self) -> i64 {
        self.max_source_to_tsource_arc_num()
            + self.max_source_to_tnode_arc_num()
            + self.max_source_to_sink_arc_num()
    }
    /// Returns the max transship node to all arc num.
    pub fn max_transship_node_to_all_arc_num(&self) -> i64 {
        self.max_tnode_to_tsource_arc_num()
            + self.max_tnode_to_tnode_arc_num()
            + self.max_tnode_to_sink_arc_num()
    }
    /// Returns the max sink to all arc num.
    pub fn max_sink_to_all_arc_num(&self) -> i64 {
        self.max_tsink_to_tsource_arc_num()
            + self.max_tsink_to_tnode_arc_num()
            + self.max_tsink_to_sink_arc_num()
    }
    /// Returns the minimum arc num.
    pub fn minimum_arc_num(&self) -> i64 {
        self.transship_node_num() as i64 + self.source_num.max(self.sink_num) as i64
    }
    /// Returns the maximum arc num.
    pub fn maximum_arc_num(&self) -> i64 {
        self.max_source_to_all_arc_num()
            + self.max_transship_node_to_all_arc_num()
            + self.max_sink_to_all_arc_num()
    }

    /// Smallest number of arcs of a valid network.
    pub fn minimum_arc_num_for(source_num: i64, tnode_num: i64, sink_num: i64) -> i64 {
        tnode_num + source_num.max(sink_num)
    }
    /// Largest number of arcs without transshipment sources / sinks.
    pub fn maximum_arc_num_for(source_num: i64, tnode_num: i64, sink_num: i64) -> i64 {
        Self::maximum_arc_num_full(source_num, 0, tnode_num, 0, sink_num)
    }
    /// Performs `maximum arc num full`.
    pub fn maximum_arc_num_full(
        source_num: i64,
        tsource_num: i64,
        tnode_num: i64,
        tsink_num: i64,
        sink_num: i64,
    ) -> i64 {
        let pure_source_num = source_num - tsource_num;
        let source_arcs = pure_source_num * tsource_num
            + tsource_num * (tsource_num - 1)
            + source_num * (tnode_num + sink_num);
        let tnode_arcs = tnode_num * (tsource_num + (tnode_num - 1) + sink_num);
        let sink_arcs = tsink_num * (tsource_num + tnode_num + (sink_num - 1));
        source_arcs + tnode_arcs + sink_arcs
    }

    /// Returns the pure source num.
    pub fn pure_source_num(&self) -> i32 {
        self.source_num - self.transship_source_num
    }
    /// Returns the pure sink num.
    pub fn pure_sink_num(&self) -> i32 {
        self.sink_num - self.transship_sink_num
    }
    /// Returns whether cost weighted.
    pub fn is_cost_weighted(&self) -> bool {
        self.min_cost != self.max_cost
    }
    /// Returns the transship node num.
    pub fn transship_node_num(&self) -> i32 {
        self.node_num - self.source_num - self.sink_num
    }
    fn transportation_problem_condition(&self) -> bool {
        self.source_num + self.sink_num == self.node_num
            && self.transship_source_num == 0
            && self.transship_sink_num == 0
    }
    fn assignment_problem_condition(&self) -> bool {
        self.source_num == self.sink_num
            && self.total_supply == self.source_num
            && self.min_cap == 1
            && self.max_cap == 1
    }
    /// Returns whether max flow problem.
    pub fn is_max_flow_problem(&self) -> bool {
        !self.is_cost_weighted()
    }
    /// Returns whether assignment problem.
    pub fn is_assignment_problem(&self) -> bool {
        self.transportation_problem_condition() && self.assignment_problem_condition()
    }

    /// Returns the node num.
    pub fn node_num(&self) -> i32 {
        self.node_num
    }
    /// Returns the arc num.
    pub fn arc_num(&self) -> i32 {
        self.arc_num
    }
    /// Returns the source num.
    pub fn source_num(&self) -> i32 {
        self.source_num
    }
    /// Returns the sink num.
    pub fn sink_num(&self) -> i32 {
        self.sink_num
    }
    /// Returns the transship source num.
    pub fn transship_source_num(&self) -> i32 {
        self.transship_source_num
    }
    /// Returns the transship sink num.
    pub fn transship_sink_num(&self) -> i32 {
        self.transship_sink_num
    }
    /// Returns the total supply.
    pub fn total_supply(&self) -> i32 {
        self.total_supply
    }
    /// Returns the min cap.
    pub fn min_cap(&self) -> i32 {
        self.min_cap
    }
    /// Returns the max cap.
    pub fn max_cap(&self) -> i32 {
        self.max_cap
    }
    /// Returns the min cost.
    pub fn min_cost(&self) -> i32 {
        self.min_cost
    }
    /// Returns the max cost.
    pub fn max_cost(&self) -> i32 {
        self.max_cost
    }
    /// Returns the percent capacitated.
    pub fn percent_capacitated(&self) -> i32 {
        self.percent_capacitated
    }
    /// Returns the percent with inf cost.
    pub fn percent_with_inf_cost(&self) -> i32 {
        self.percent_with_inf_cost
    }
}
