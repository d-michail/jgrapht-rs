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
    error: Option<GraphError>,
}

impl Default for NetworkGeneratorConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkGeneratorConfigBuilder {
    /// A builder with all counts zero; set at least the node, arc, source and sink counts before
    /// [`build`](Self::build).
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

    fn fail(&mut self, error: GraphError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    /// Returns the build.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if a numeric or collection parameter is out of range; errors of the underlying graph operations are propagated.
    pub fn build(&self) -> Result<NetworkGeneratorConfig> {
        if let Some(e) = &self.error {
            return Err(e.clone());
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
            return Err(GraphError::InvalidParameter {
                name: "sources_and_sinks",
                requirement: crate::ParameterRequirement::Consistent,
            });
        } else if self.tsource_num > s {
            return Err(GraphError::InvalidParameter {
                name: "transshipment_sources",
                requirement: crate::ParameterRequirement::Consistent,
            });
        } else if self.tsink_num > t {
            return Err(GraphError::InvalidParameter {
                name: "transshipment_sinks",
                requirement: crate::ParameterRequirement::Consistent,
            });
        } else if self.total_supply < s.max(t) {
            return Err(GraphError::InvalidParameter {
                name: "total_supply",
                requirement: crate::ParameterRequirement::Consistent,
            });
        } else if self.min_cap > self.max_cap {
            return Err(GraphError::InvalidParameter {
                name: "capacity",
                requirement: crate::ParameterRequirement::Consistent,
            });
        } else if self.min_cap <= 0 {
            return Err(GraphError::must_be_positive("minimum_capacity"));
        } else if self.min_cost > self.max_cost {
            return Err(GraphError::InvalidParameter {
                name: "cost",
                requirement: crate::ParameterRequirement::Consistent,
            });
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
        if (a as i64) < min_arc_num || a as i64 > max_arc_num {
            return Err(GraphError::InvalidParameter {
                name: "arcs",
                requirement: crate::ParameterRequirement::Consistent,
            });
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
            self.fail(GraphError::InvalidParameter {
                name: "number_of_nodes",
                requirement: crate::ParameterRequirement::AtMost(MAX_NODE_NUM as u64),
            });
        }
        value
    }

    fn check_capacity_cost_constraint(&mut self, value: i32) -> i32 {
        if (value as i64).abs() > CAPACITY_COST_BOUND as i64 {
            self.fail(GraphError::InvalidParameter {
                name: "capacity_or_cost",
                requirement: crate::ParameterRequirement::AbsAtMost(CAPACITY_COST_BOUND as u64),
            });
        }
        value
    }

    #[allow(clippy::too_many_arguments)]
    /// Sets every parameter of a general minimum-cost flow problem at once. The two `transship_*`
    /// counts say how many sources and sinks also act as transshipment nodes.
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

    /// As [`set_maximum_flow_problem_params`](Self::set_maximum_flow_problem_params) with the arc
    /// capacities drawn from `min_cap..=max_cap`.
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

    /// As [`set_maximum_flow_problem_params_cap`](Self::set_maximum_flow_problem_params_cap) with
    /// explicit numbers of sources and sinks.
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
    /// As [`set_maximum_flow_problem_params_sources`](Self::set_maximum_flow_problem_params_sources)
    /// with `percent_capacitated` percent of the arcs capacitated; the others have infinite capacity.
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

    /// Bipartite matching problem on `node_num` nodes (half on each side) with `arc_num` arcs and unit
    /// costs. `node_num` must be even.
    pub fn set_bipartite_matching_problem_params(self, node_num: i32, arc_num: i32) -> Self {
        self.set_bipartite_matching_problem_params_cost(node_num, arc_num, 1, 1)
    }

    /// As [`set_bipartite_matching_problem_params`](Self::set_bipartite_matching_problem_params) with
    /// arc costs drawn from `min_cost..=max_cost`.
    pub fn set_bipartite_matching_problem_params_cost(
        self,
        node_num: i32,
        arc_num: i32,
        min_cost: i32,
        max_cost: i32,
    ) -> Self {
        self.set_bipartite_matching_problem_params_full(node_num, arc_num, min_cost, max_cost, 0)
    }

    /// As [`set_bipartite_matching_problem_params_cost`](Self::set_bipartite_matching_problem_params_cost)
    /// with `percent_with_inf_cost` percent of the arcs given infinite cost.
    pub fn set_bipartite_matching_problem_params_full(
        mut self,
        node_num: i32,
        arc_num: i32,
        min_cost: i32,
        max_cost: i32,
        percent_with_inf_cost: i32,
    ) -> Self {
        if node_num & 1 != 0 {
            self.fail(GraphError::InvalidParameter {
                name: "number_of_nodes",
                requirement: crate::ParameterRequirement::Even,
            });
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

    /// Total number of nodes; must be positive.
    pub fn set_node_num(mut self, node_num: i32) -> Self {
        if node_num <= 0 {
            self.fail(GraphError::must_be_positive("number_of_nodes"));
        } else {
            self.node_num = self.check_node_constraint(node_num);
        }
        self
    }
    /// Number of arcs to generate.
    pub fn set_arc_num(mut self, arc_num: i32) -> Self {
        if arc_num > MAX_ARC_NUM {
            self.fail(GraphError::InvalidParameter {
                name: "number_of_arcs",
                requirement: crate::ParameterRequirement::AtMost(MAX_ARC_NUM as u64),
            });
        } else {
            self.arc_num = arc_num;
        }
        self
    }
    /// Number of sources; must be positive.
    pub fn set_source_num(mut self, source_num: i32) -> Self {
        if source_num <= 0 {
            self.fail(GraphError::must_be_positive("number_of_sources"));
        } else {
            self.source_num = self.check_node_constraint(source_num);
        }
        self
    }
    /// Number of sinks; must be positive.
    pub fn set_sink_num(mut self, sink_num: i32) -> Self {
        if sink_num <= 0 {
            self.fail(GraphError::must_be_positive("number_of_sinks"));
        } else {
            self.sink_num = self.check_node_constraint(sink_num);
        }
        self
    }
    /// Number of sources that are also transshipment nodes; must not be negative.
    pub fn set_tsource_num(mut self, tsource_num: i32) -> Self {
        if tsource_num < 0 {
            self.fail(GraphError::must_be_non_negative(
                "number_of_transshipment_sources",
            ));
        } else {
            self.tsource_num = self.check_node_constraint(tsource_num);
        }
        self
    }
    /// Number of sinks that are also transshipment nodes; must not be negative.
    pub fn set_tsink_num(mut self, tsink_num: i32) -> Self {
        if tsink_num < 0 {
            self.fail(GraphError::must_be_non_negative(
                "number_of_transshipment_sinks",
            ));
        } else {
            self.tsink_num = self.check_node_constraint(tsink_num);
        }
        self
    }
    /// Total supply injected by the sources.
    pub fn set_total_supply(mut self, total_supply: i32) -> Self {
        if total_supply > MAX_SUPPLY {
            self.fail(GraphError::InvalidParameter {
                name: "total_supply",
                requirement: crate::ParameterRequirement::AtMost(MAX_SUPPLY as u64),
            });
        } else {
            self.total_supply = total_supply;
        }
        self
    }
    /// Smallest arc capacity; must not be negative.
    pub fn set_min_cap(mut self, min_cap: i32) -> Self {
        if min_cap < 0 {
            self.fail(GraphError::must_be_non_negative("minimum_capacity"));
        } else {
            self.min_cap = self.check_capacity_cost_constraint(min_cap);
        }
        self
    }
    /// Largest arc capacity; must not be negative.
    pub fn set_max_cap(mut self, max_cap: i32) -> Self {
        if max_cap < 0 {
            self.fail(GraphError::must_be_non_negative("maximum_capacity"));
        } else {
            self.max_cap = self.check_capacity_cost_constraint(max_cap);
        }
        self
    }
    /// Smallest arc cost.
    pub fn set_min_cost(mut self, min_cost: i32) -> Self {
        self.min_cost = self.check_capacity_cost_constraint(min_cost);
        self
    }
    /// Largest arc cost.
    pub fn set_max_cost(mut self, max_cost: i32) -> Self {
        self.max_cost = self.check_capacity_cost_constraint(max_cost);
        self
    }
    /// Percentage of arcs that get a finite capacity, in `0..=100`.
    pub fn set_percent_capacitated(mut self, p: i32) -> Self {
        if !(0..=100).contains(&p) {
            self.fail(GraphError::InvalidParameter {
                name: "percent_capacitated",
                requirement: crate::ParameterRequirement::InRange(0, 100),
            });
        } else {
            self.percent_capacitated = p;
        }
        self
    }
    /// Percentage of arcs with infinite cost, in `0..=100`.
    pub fn set_percent_with_inf_cost(mut self, p: i32) -> Self {
        if !(0..=100).contains(&p) {
            self.fail(GraphError::InvalidParameter {
                name: "percent_with_inf_cost",
                requirement: crate::ParameterRequirement::InRange(0, 100),
            });
        } else {
            self.percent_with_inf_cost = p;
        }
        self
    }
}
