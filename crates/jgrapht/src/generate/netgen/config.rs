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
    pub fn max_source_to_tsource_arc_num(&self) -> i64 {
        self.pure_source_num() as i64 * self.transship_source_num as i64
            + self.transship_source_num as i64 * (self.transship_source_num as i64 - 1)
    }
    pub fn max_source_to_tnode_arc_num(&self) -> i64 {
        self.source_num as i64 * self.transship_node_num() as i64
    }
    pub fn max_source_to_sink_arc_num(&self) -> i64 {
        self.source_num as i64 * self.sink_num as i64
    }
    pub fn max_tnode_to_tsource_arc_num(&self) -> i64 {
        self.transship_node_num() as i64 * self.transship_source_num as i64
    }
    pub fn max_tnode_to_tnode_arc_num(&self) -> i64 {
        self.transship_node_num() as i64 * (self.transship_node_num() as i64 - 1)
    }
    pub fn max_tnode_to_sink_arc_num(&self) -> i64 {
        self.transship_node_num() as i64 * self.sink_num as i64
    }
    pub fn max_tsink_to_tsource_arc_num(&self) -> i64 {
        self.transship_sink_num as i64 * self.transship_source_num as i64
    }
    pub fn max_tsink_to_tnode_arc_num(&self) -> i64 {
        self.transship_sink_num as i64 * self.transship_node_num() as i64
    }
    pub fn max_tsink_to_sink_arc_num(&self) -> i64 {
        // the second product is an `int` multiplication in JGraphT
        self.transship_sink_num as i64 * (self.transship_sink_num as i64 - 1)
            + self.pure_sink_num().wrapping_mul(self.transship_sink_num) as i64
    }
    pub fn max_source_to_all_arc_num(&self) -> i64 {
        self.max_source_to_tsource_arc_num()
            + self.max_source_to_tnode_arc_num()
            + self.max_source_to_sink_arc_num()
    }
    pub fn max_transship_node_to_all_arc_num(&self) -> i64 {
        self.max_tnode_to_tsource_arc_num()
            + self.max_tnode_to_tnode_arc_num()
            + self.max_tnode_to_sink_arc_num()
    }
    pub fn max_sink_to_all_arc_num(&self) -> i64 {
        self.max_tsink_to_tsource_arc_num()
            + self.max_tsink_to_tnode_arc_num()
            + self.max_tsink_to_sink_arc_num()
    }
    pub fn minimum_arc_num(&self) -> i64 {
        self.transship_node_num() as i64 + self.source_num.max(self.sink_num) as i64
    }
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

    pub fn pure_source_num(&self) -> i32 {
        self.source_num - self.transship_source_num
    }
    pub fn pure_sink_num(&self) -> i32 {
        self.sink_num - self.transship_sink_num
    }
    pub fn is_cost_weighted(&self) -> bool {
        self.min_cost != self.max_cost
    }
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
    pub fn is_max_flow_problem(&self) -> bool {
        !self.is_cost_weighted()
    }
    pub fn is_assignment_problem(&self) -> bool {
        self.transportation_problem_condition() && self.assignment_problem_condition()
    }

    pub fn node_num(&self) -> i32 {
        self.node_num
    }
    pub fn arc_num(&self) -> i32 {
        self.arc_num
    }
    pub fn source_num(&self) -> i32 {
        self.source_num
    }
    pub fn sink_num(&self) -> i32 {
        self.sink_num
    }
    pub fn transship_source_num(&self) -> i32 {
        self.transship_source_num
    }
    pub fn transship_sink_num(&self) -> i32 {
        self.transship_sink_num
    }
    pub fn total_supply(&self) -> i32 {
        self.total_supply
    }
    pub fn min_cap(&self) -> i32 {
        self.min_cap
    }
    pub fn max_cap(&self) -> i32 {
        self.max_cap
    }
    pub fn min_cost(&self) -> i32 {
        self.min_cost
    }
    pub fn max_cost(&self) -> i32 {
        self.max_cost
    }
    pub fn percent_capacitated(&self) -> i32 {
        self.percent_capacitated
    }
    pub fn percent_with_inf_cost(&self) -> i32 {
        self.percent_with_inf_cost
    }
}
