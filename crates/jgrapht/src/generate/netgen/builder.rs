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
use super::generator::{CAPACITY_COST_BOUND, MAX_ARC_NUM, MAX_NODE_NUM, MAX_SUPPLY};
use crate::{GraphError, Result};

/// Builder of [`NetworkGeneratorConfig`].
///
/// JGraphT's setters throw as soon as a value is out of range; here the first such error is
/// remembered and returned by [`build`](Self::build).
#[derive(Debug, Clone)]
pub struct NetworkGeneratorConfigBuilder {
    node_num: i32,
    arc_num: i32,
    source_num: i32,
    sink_num: i32,
    tsource_num: i32,
    tsink_num: i32,
    total_supply: i32,
    min_cap: i32,
    max_cap: i32,
    min_cost: i32,
    max_cost: i32,
    percent_capacitated: i32,
    percent_with_inf_cost: i32,
    error: Option<String>,
}

impl Default for NetworkGeneratorConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(GraphError::InvalidArgument(msg.into()))
}

impl NetworkGeneratorConfigBuilder {
    /// Creates a new instance.
    pub fn new() -> Self {
        NetworkGeneratorConfigBuilder {
            node_num: 0,
            arc_num: 0,
            source_num: 0,
            sink_num: 0,
            tsource_num: 0,
            tsink_num: 0,
            total_supply: 0,
            min_cap: 0,
            max_cap: 0,
            min_cost: 0,
            max_cost: 0,
            percent_capacitated: 100,
            percent_with_inf_cost: 0,
            error: None,
        }
    }

    fn fail(&mut self, msg: impl Into<String>) {
        if self.error.is_none() {
            self.error = Some(msg.into());
        }
    }

    /// Returns the build.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if a numeric or collection parameter is out of range; errors of the underlying graph operations are propagated.
    pub fn build(&self) -> Result<NetworkGeneratorConfig> {
        if let Some(e) = &self.error {
            return invalid(e.clone());
        }
        let (n, a, s, t) = (self.node_num, self.arc_num, self.source_num, self.sink_num);
        if n <= 0 {
            return Err(GraphError::must_be_positive("number_of_nodes"));
        } else if a <= 0 {
            return Err(GraphError::must_be_positive("number_of_arcs"));
        } else if s <= 0 {
            return Err(GraphError::must_be_positive("number_of_sources"));
        } else if t <= 0 {
            return Err(GraphError::must_be_positive("number_of_sinks"));
        } else if s as i64 + t as i64 > n as i64 {
            return invalid("Number of sources and sinks must not exceed the number of nodes");
        } else if self.tsource_num > s {
            return invalid(
                "Number of transhipment sources must not exceed the overall number of sources",
            );
        } else if self.tsink_num > t {
            return invalid(
                "Number of transhipment sinks must not exceed the overall number of sinks",
            );
        } else if self.total_supply < s.max(t) {
            return invalid(
                "Total supply must not be less than the number of sources and the number of sinks",
            );
        } else if self.min_cap > self.max_cap {
            return invalid("Minimum capacity must not exceed the maximum capacity");
        } else if self.min_cap <= 0 {
            return Err(GraphError::must_be_positive("minimum_capacity"));
        } else if self.min_cost > self.max_cost {
            return invalid("Minimum cost must not exceed the maximum cost");
        }
        let tnode_num = n as i64 - s as i64 - t as i64;
        let min_arc_num =
            NetworkGeneratorConfig::minimum_arc_num_for(s as i64, tnode_num, t as i64);
        let max_arc_num = NetworkGeneratorConfig::maximum_arc_num_full(
            s as i64,
            self.tsource_num as i64,
            tnode_num,
            self.tsink_num as i64,
            t as i64,
        );
        if (a as i64) < min_arc_num {
            return invalid("Too few arcs to generate a valid problem");
        } else if a as i64 > max_arc_num {
            return invalid("Too many arcs to generate a valid problem");
        }
        Ok(NetworkGeneratorConfig {
            node_num: n,
            arc_num: a,
            source_num: s,
            sink_num: t,
            transship_source_num: self.tsource_num,
            transship_sink_num: self.tsink_num,
            total_supply: self.total_supply,
            min_cap: self.min_cap,
            max_cap: self.max_cap,
            min_cost: self.min_cost,
            max_cost: self.max_cost,
            percent_capacitated: self.percent_capacitated,
            percent_with_inf_cost: self.percent_with_inf_cost,
        })
    }

    fn check_node_constraint(&mut self, value: i32) -> i32 {
        if value > MAX_NODE_NUM {
            self.fail(format!("Number of nodes must not exceed {MAX_NODE_NUM}"));
        }
        value
    }

    fn check_capacity_cost_constraint(&mut self, value: i32) -> i32 {
        if (value as i64).abs() > CAPACITY_COST_BOUND as i64 {
            self.fail(format!(
                "Arcs capacities and cost must be between -{CAPACITY_COST_BOUND} and {CAPACITY_COST_BOUND}"
            ));
        }
        value
    }

    #[allow(clippy::too_many_arguments)]
    /// Sets the params.
    pub fn set_params(
        self,
        node_num: i32,
        arc_num: i32,
        source_num: i32,
        sink_num: i32,
        transship_source_num: i32,
        transship_sink_num: i32,
        total_supply: i32,
        min_cap: i32,
        max_cap: i32,
        min_cost: i32,
        max_cost: i32,
        percent_capacitated: i32,
        percent_with_inf_cost: i32,
    ) -> Self {
        self.set_node_num(node_num)
            .set_arc_num(arc_num)
            .set_source_num(source_num)
            .set_sink_num(sink_num)
            .set_tsource_num(transship_source_num)
            .set_tsink_num(transship_sink_num)
            .set_total_supply(total_supply)
            .set_min_cap(min_cap)
            .set_max_cap(max_cap)
            .set_min_cost(min_cost)
            .set_max_cost(max_cost)
            .set_percent_capacitated(percent_capacitated)
            .set_percent_with_inf_cost(percent_with_inf_cost)
    }

    /// Single source, single sink, unit capacities.
    pub fn set_maximum_flow_problem_params(self, node_num: i32, arc_num: i32, supply: i32) -> Self {
        self.set_maximum_flow_problem_params_cap(node_num, arc_num, supply, 1, 1)
    }

    /// Sets the maximum flow problem params cap.
    pub fn set_maximum_flow_problem_params_cap(
        self,
        node_num: i32,
        arc_num: i32,
        supply: i32,
        min_cap: i32,
        max_cap: i32,
    ) -> Self {
        self.set_maximum_flow_problem_params_sources(
            node_num, arc_num, supply, min_cap, max_cap, 1, 1,
        )
    }

    /// Sets the maximum flow problem params sources.
    pub fn set_maximum_flow_problem_params_sources(
        self,
        node_num: i32,
        arc_num: i32,
        supply: i32,
        min_cap: i32,
        max_cap: i32,
        source_num: i32,
        sink_num: i32,
    ) -> Self {
        self.set_maximum_flow_problem_params_full(
            node_num, arc_num, supply, min_cap, max_cap, source_num, sink_num, 100,
        )
    }

    #[allow(clippy::too_many_arguments)]
    /// Sets the maximum flow problem params full.
    pub fn set_maximum_flow_problem_params_full(
        self,
        node_num: i32,
        arc_num: i32,
        supply: i32,
        min_cap: i32,
        max_cap: i32,
        source_num: i32,
        sink_num: i32,
        percent_capacitated: i32,
    ) -> Self {
        self.set_params(
            node_num,
            arc_num,
            source_num,
            sink_num,
            0,
            0,
            supply,
            min_cap,
            max_cap,
            1,
            1,
            percent_capacitated,
            0,
        )
    }

    /// Sets the bipartite matching problem params.
    pub fn set_bipartite_matching_problem_params(self, node_num: i32, arc_num: i32) -> Self {
        self.set_bipartite_matching_problem_params_cost(node_num, arc_num, 1, 1)
    }

    /// Sets the bipartite matching problem params cost.
    pub fn set_bipartite_matching_problem_params_cost(
        self,
        node_num: i32,
        arc_num: i32,
        min_cost: i32,
        max_cost: i32,
    ) -> Self {
        self.set_bipartite_matching_problem_params_full(node_num, arc_num, min_cost, max_cost, 0)
    }

    /// Sets the bipartite matching problem params full.
    pub fn set_bipartite_matching_problem_params_full(
        mut self,
        node_num: i32,
        arc_num: i32,
        min_cost: i32,
        max_cost: i32,
        percent_with_inf_cost: i32,
    ) -> Self {
        if node_num & 1 != 0 {
            self.fail("Assignment problem must have even number of nodes");
            return self;
        }
        self.set_params(
            node_num,
            arc_num,
            node_num / 2,
            node_num / 2,
            0,
            0,
            node_num / 2,
            1,
            1,
            min_cost,
            max_cost,
            100,
            percent_with_inf_cost,
        )
    }

    /// Sets the node num.
    pub fn set_node_num(mut self, node_num: i32) -> Self {
        if node_num <= 0 {
            self.fail("Number of nodes must be positive");
        } else {
            self.node_num = self.check_node_constraint(node_num);
        }
        self
    }
    /// Sets the arc num.
    pub fn set_arc_num(mut self, arc_num: i32) -> Self {
        if arc_num > MAX_ARC_NUM {
            self.fail(format!("Number of arcs must not exceed {arc_num}"));
        } else {
            self.arc_num = arc_num;
        }
        self
    }
    /// Sets the source num.
    pub fn set_source_num(mut self, source_num: i32) -> Self {
        if source_num <= 0 {
            self.fail("Number of sources must be positive");
        } else {
            self.source_num = self.check_node_constraint(source_num);
        }
        self
    }
    /// Sets the sink num.
    pub fn set_sink_num(mut self, sink_num: i32) -> Self {
        if sink_num <= 0 {
            self.fail("Number of sinks must be positive");
        } else {
            self.sink_num = self.check_node_constraint(sink_num);
        }
        self
    }
    /// Sets the tsource num.
    pub fn set_tsource_num(mut self, tsource_num: i32) -> Self {
        if tsource_num < 0 {
            self.fail("Number of transshipment sources must be non-negative");
        } else {
            self.tsource_num = self.check_node_constraint(tsource_num);
        }
        self
    }
    /// Sets the tsink num.
    pub fn set_tsink_num(mut self, tsink_num: i32) -> Self {
        if tsink_num < 0 {
            self.fail("Number of transshipment sinks must be non-negative");
        } else {
            self.tsink_num = self.check_node_constraint(tsink_num);
        }
        self
    }
    /// Sets the total supply.
    pub fn set_total_supply(mut self, total_supply: i32) -> Self {
        if total_supply > MAX_SUPPLY {
            self.fail(format!("Total supply must not exceed {MAX_NODE_NUM}"));
        } else {
            self.total_supply = total_supply;
        }
        self
    }
    /// Sets the min cap.
    pub fn set_min_cap(mut self, min_cap: i32) -> Self {
        if min_cap < 0 {
            self.fail("Minimum arc capacity must be non-negative");
        } else {
            self.min_cap = self.check_capacity_cost_constraint(min_cap);
        }
        self
    }
    /// Sets the max cap.
    pub fn set_max_cap(mut self, max_cap: i32) -> Self {
        if max_cap < 0 {
            self.fail("Maximum arc capacity must be non-negative");
        } else {
            self.max_cap = self.check_capacity_cost_constraint(max_cap);
        }
        self
    }
    /// Sets the min cost.
    pub fn set_min_cost(mut self, min_cost: i32) -> Self {
        self.min_cost = self.check_capacity_cost_constraint(min_cost);
        self
    }
    /// Sets the max cost.
    pub fn set_max_cost(mut self, max_cost: i32) -> Self {
        self.max_cost = self.check_capacity_cost_constraint(max_cost);
        self
    }
    /// Sets the percent capacitated.
    pub fn set_percent_capacitated(mut self, p: i32) -> Self {
        if !(0..=100).contains(&p) {
            self.fail("Percent of capacitated arcs must be between 0 and 100 inclusive");
        } else {
            self.percent_capacitated = p;
        }
        self
    }
    /// Sets the percent with inf cost.
    pub fn set_percent_with_inf_cost(mut self, p: i32) -> Self {
        if !(0..=100).contains(&p) {
            self.fail("Percent of arcs with infinite cost must be between 0 and 100 inclusive");
        } else {
            self.percent_with_inf_cost = p;
        }
        self
    }
}
