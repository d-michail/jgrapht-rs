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
use super::distributor::Distributor;
use super::problems::{BipartiteMatchingProblem, MaximumFlowProblem, NetworkInfo};
use crate::alg::flow::MinimumCostFlowProblemImpl;
use crate::graph::OutgoingGraph;
use crate::graph::props::require_directed;
use crate::graph::{Graph, GraphMut};
use crate::util::ElementsSequenceGenerator;
use crate::util::java::RandomSource;
use crate::{GraphError, Result};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::{HashMap, HashSet};

pub const MAX_NODE_NUM: i32 = 100 * 1000 * 1000;
pub const MAX_SUPPLY: i32 = 200 * 1000 * 1000;
pub const MAX_ARC_NUM: i32 = 2 * 1000 * 1000 * 1000;
pub const CAPACITY_COST_BOUND: i32 = 2 * 1000 * 1000 * 1000;

fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(GraphError::InvalidArgument(msg.into()))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NodeType {
    PureSource,
    TransshipSource,
    TransshipNode,
    TransshipSink,
    PureSink,
}

struct Node<V> {
    vertex: V,
    supply: i32,
    ty: NodeType,
    chain_nodes: Vec<usize>,
}

/// Indices into the arc-class upper bound array.
const S2TS: usize = 0;
const S2TN: usize = 1;
const S2S: usize = 2;
const TN2TS: usize = 3;
const TN2TN: usize = 4;
const TN2S: usize = 5;
const TS2TS: usize = 6;
const TS2TN: usize = 7;
const TS2S: usize = 8;

/// NETGEN-style generator of minimum cost flow, maximum flow and assignment problems.
///
/// The structure (chains of transshipment nodes from the sources, arcs to the sinks, then random
/// arcs up to the requested count) and the random-number consumption follow JGraphT, so a
/// [`JavaRandom`](crate::util::java::JavaRandom) given to [`NetworkGenerator::with_rng`] reproduces
/// JGraphT's networks. The
/// seeded constructor [`NetworkGenerator::new`] uses `rand`'s `StdRng`.
pub struct NetworkGenerator<V, E, R = StdRng> {
    config: NetworkGeneratorConfig,
    rng: R,
    info: Option<NetworkInfo<V, E>>,
}

impl<V, E> NetworkGenerator<V, E, StdRng> {
    pub fn new(config: NetworkGeneratorConfig, seed: u64) -> Self {
        Self::with_rng(config, StdRng::seed_from_u64(seed))
    }
}

impl<V, E, R: RandomSource> NetworkGenerator<V, E, R> {
    pub fn with_rng(config: NetworkGeneratorConfig, rng: R) -> Self {
        NetworkGenerator {
            config,
            rng,
            info: None,
        }
    }

    /// Information about the last generated network.
    pub fn network_info(&self) -> Option<&NetworkInfo<V, E>> {
        self.info.as_ref()
    }
}

impl<V, E, R> NetworkGenerator<V, E, R>
where
    V: Clone + Eq + std::hash::Hash,
    E: Clone + Eq + std::hash::Hash,
    R: RandomSource,
{
    pub fn generate_bipartite_matching_problem<
        G: GraphMut<Vertex = V, Edge = E> + OutgoingGraph,
    >(
        &mut self,
        graph: &mut G,
    ) -> Result<BipartiteMatchingProblem<V, E>> {
        if !self.config.is_assignment_problem() {
            return invalid("Input config doesn't specify a bipartite matching problem");
        }
        require_directed(graph)?;
        let run = self.generate(graph)?;
        let info = self.info.as_ref().unwrap();
        Ok(BipartiteMatchingProblem {
            partition1: info.sources().iter().cloned().collect(),
            partition2: info.sinks().iter().cloned().collect(),
            costs: run.cost.into_iter().map(|(e, c)| (e, c as f64)).collect(),
            weighted: self.config.is_cost_weighted(),
        })
    }

    pub fn generate_max_flow_problem<G: GraphMut<Vertex = V, Edge = E> + OutgoingGraph>(
        &mut self,
        graph: &mut G,
    ) -> Result<MaximumFlowProblem<V, E>> {
        if !self.config.is_max_flow_problem() {
            return invalid("Input config doesn't specify a maximum flow problem");
        }
        require_directed(graph)?;
        let run = self.generate(graph)?;
        let info = self.info.as_ref().unwrap();
        Ok(MaximumFlowProblem {
            sources: info.sources().iter().cloned().collect(),
            sinks: info.sinks().iter().cloned().collect(),
            capacities: run
                .capacity
                .into_iter()
                .map(|(e, c)| (e, c as f64))
                .collect(),
        })
    }

    pub fn generate_minimum_cost_flow_problem<
        'g,
        G: GraphMut<Vertex = V, Edge = E> + OutgoingGraph,
    >(
        &mut self,
        graph: &'g mut G,
    ) -> Result<MinimumCostFlowProblemImpl<'g, G>>
    where
        V: 'g,
        E: 'g,
    {
        require_directed(graph)?;
        let run = self.generate(graph)?;
        Ok(MinimumCostFlowProblemImpl::with_costs(
            graph,
            move |vertex| run.supply.get(vertex).copied().unwrap_or(0),
            move |edge| run.capacity[edge],
            |_| 0,
            move |edge| run.cost[edge] as f64,
        ))
    }

    fn generate<G: GraphMut<Vertex = V, Edge = E> + OutgoingGraph>(
        &mut self,
        graph: &mut G,
    ) -> Result<RunResult<V, E>> {
        let mut info = NetworkInfo::new(self.config.clone());
        let mut run = Run::new(&self.config, &mut self.rng, graph)?;
        run.create_supply();
        run.init_chains();
        run.generate_chains()?;
        run.connect_chains_to_sinks()?;
        run.add_all_remaining_arcs()?;
        info.vertices = run.nodes.iter().map(|n| n.vertex.clone()).collect();
        info.skeleton_arcs = std::mem::take(&mut run.skeleton);
        let result = RunResult {
            supply: run.supply,
            capacity: run.capacity,
            cost: run.cost,
        };
        self.info = Some(info);
        Ok(result)
    }
}

struct RunResult<V, E> {
    supply: HashMap<V, i32>,
    capacity: HashMap<E, i32>,
    cost: HashMap<E, i32>,
}

/// State of one generation (the Java generator's fields).
struct Run<'a, G: Graph, R> {
    config: &'a NetworkGeneratorConfig,
    rng: &'a mut R,
    graph: &'a mut G,
    nodes: Vec<Node<G::Vertex>>,
    index: HashMap<G::Vertex, usize>,
    supply: HashMap<G::Vertex, i32>,
    capacity: HashMap<G::Edge, i32>,
    cost: HashMap<G::Edge, i32>,
    skeleton: Vec<G::Edge>,
    ub: [i64; 9],
}

impl<'a, G: GraphMut + OutgoingGraph, R: RandomSource> Run<'a, G, R> {
    fn new(config: &'a NetworkGeneratorConfig, rng: &'a mut R, graph: &'a mut G) -> Result<Self> {
        let mut run = Run {
            config,
            rng,
            graph,
            nodes: Vec::new(),
            index: HashMap::new(),
            supply: HashMap::new(),
            capacity: HashMap::new(),
            cost: HashMap::new(),
            skeleton: Vec::new(),
            ub: [
                config.max_source_to_tsource_arc_num(),
                config.max_source_to_tnode_arc_num(),
                config.max_source_to_sink_arc_num(),
                config.max_tnode_to_tsource_arc_num(),
                config.max_tnode_to_tnode_arc_num(),
                config.max_tnode_to_sink_arc_num(),
                config.max_tsink_to_tsource_arc_num(),
                config.max_tsink_to_tnode_arc_num(),
                config.max_tsink_to_sink_arc_num(),
            ],
        };
        run.create_nodes(config.pure_source_num(), NodeType::PureSource)?;
        run.create_nodes(config.transship_source_num(), NodeType::TransshipSource)?;
        run.create_nodes(config.transship_node_num(), NodeType::TransshipNode)?;
        run.create_nodes(config.transship_sink_num(), NodeType::TransshipSink)?;
        run.create_nodes(config.pure_sink_num(), NodeType::PureSink)?;
        Ok(run)
    }

    fn create_nodes(&mut self, num: i32, ty: NodeType) -> Result<()> {
        for _ in 0..num {
            let vertex = self.graph.add_new_vertex()?;
            self.index.insert(vertex.clone(), self.nodes.len());
            self.nodes.push(Node {
                vertex,
                supply: 0,
                ty,
                chain_nodes: Vec::new(),
            });
        }
        Ok(())
    }

    fn create_supply(&mut self) {
        let source_num = self.config.source_num();
        // supply per source is guaranteed to be at least 1
        let supply_per_source = self.config.total_supply() / source_num;
        for source_id in 0..source_num as usize {
            let partial = self.rng.next_int_bound(supply_per_source as usize) as i32 + 1;
            self.nodes[source_id].supply += partial;
            let random_source = self.rng.next_int_bound(source_num as usize);
            self.nodes[random_source].supply += supply_per_source - partial;
        }
        let random_source = self.rng.next_int_bound(source_num as usize);
        self.nodes[random_source].supply += self.config.total_supply() % source_num;
        for node in &self.nodes {
            if node.supply != 0 {
                self.supply.insert(node.vertex.clone(), node.supply);
            }
        }
    }

    fn init_chains(&mut self) {
        for id in 0..self.config.source_num() as usize {
            self.nodes[id].chain_nodes.push(id);
        }
    }

    fn generate_chains(&mut self) -> Result<()> {
        let source_num = self.config.source_num() as usize;
        let tnode_num = self.config.transship_node_num();
        let sixty_percent = (6 * tnode_num) / 10;
        let mut tnodes = ElementsSequenceGenerator::new(self.transship_nodes());
        // chains from the sources using ~60% of the transshipment nodes
        let mut chain_source_id = 0;
        for _ in 0..sixty_percent {
            if chain_source_id == source_num {
                chain_source_id = 0;
            }
            let head = tnodes.next(self.rng).unwrap();
            let last = *self.nodes[chain_source_id].chain_nodes.last().unwrap();
            self.add_skeleton_arc(chain_source_id, last, head)?;
            chain_source_id += 1;
        }
        // randomly extend the chains with the remaining ~40%
        while let Some(head) = tnodes.next(self.rng) {
            let source_id = self.rng.next_int_bound(source_num);
            let last = *self.nodes[source_id].chain_nodes.last().unwrap();
            self.add_skeleton_arc(source_id, last, head)?;
        }
        Ok(())
    }

    fn connect_chains_to_sinks(&mut self) -> Result<()> {
        let remaining_arcs = self.config.arc_num() - self.graph.edge_count() as i32;
        debug_assert!(remaining_arcs >= self.config.sink_num());
        let mut chain_to_sink_arcs =
            remaining_arcs.min(2 * self.config.source_num().max(self.config.sink_num()));
        let chain_to_sink_arc_ub = (self.ub[S2S] + self.ub[TN2S]).min(MAX_ARC_NUM as i64) as i32;
        chain_to_sink_arcs = chain_to_sink_arc_ub.min(chain_to_sink_arcs);
        let sources = self.sources();
        // this sum is at least max(source_num, sink_num)
        let mut supply_and_sink_num_ub = 0i32;
        for &s in &sources {
            supply_and_sink_num_ub += self.config.sink_num().min(self.nodes[s].supply);
        }
        chain_to_sink_arcs = chain_to_sink_arcs.min(supply_and_sink_num_ub);

        let sinks_per_source = {
            let nodes = &self.nodes;
            let sink_num = self.config.sink_num();
            let mut distributor = Distributor::new();
            distributor.add_lower_bound(|_: &usize| 1);
            distributor.add_upper_bound(|s: &usize| nodes[*s].supply);
            distributor.add_upper_bound(|_: &usize| sink_num);
            distributor.get_distribution(self.rng, &sources, chain_to_sink_arcs)?
        };

        let sinks = self.sinks();
        let mut sink_id = 0;
        for (i, &chain_source) in sources.iter().enumerate() {
            let count = sinks_per_source[i];
            let mut chain_sinks = Vec::new();
            for _ in 0..count {
                if sink_id == sinks.len() {
                    sink_id = 0;
                }
                chain_sinks.push(sinks[sink_id]);
                sink_id += 1;
            }
            let supply_dist = {
                let mut distributor = Distributor::new();
                distributor.add_lower_bound(|_: &usize| 1);
                distributor.get_distribution(
                    self.rng,
                    &chain_sinks,
                    self.nodes[chain_source].supply,
                )?
            };
            for j in 0..count as usize {
                let sink = chain_sinks[j];
                let sink_supply = supply_dist[j];
                let tail_index = self
                    .rng
                    .next_int_bound(self.nodes[chain_source].chain_nodes.len());
                let tail = self.nodes[chain_source].chain_nodes[tail_index];
                self.add_skeleton_arc(chain_source, tail, sink)?;
                let v = self.nodes[sink].vertex.clone();
                *self.supply.entry(v).or_insert(0) -= sink_supply;
            }
        }
        Ok(())
    }

    fn add_all_remaining_arcs(&mut self) -> Result<()> {
        let remaining_arcs = self.config.arc_num() - self.graph.edge_count() as i32;
        debug_assert!(remaining_arcs >= 0);
        let upper_bounds = self.ub;
        let class_bounds_sum: i64 = upper_bounds.iter().sum();
        if class_bounds_sum == 0 {
            return Ok(());
        }
        let class_ids: Vec<usize> = (0..upper_bounds.len()).collect();
        let distribution = {
            let mut distributor = Distributor::new();
            distributor
                .add_upper_bound(|c: &usize| upper_bounds[*c].min(MAX_ARC_NUM as i64) as i32);
            distributor.add_upper_bound(|c: &usize| {
                let class_weight = upper_bounds[*c] as f64 / class_bounds_sum as f64;
                let weight_bound = (2.0 * class_weight * remaining_arcs as f64) as i32;
                weight_bound.wrapping_add(1) // make this bound positive
            });
            distributor.get_distribution(self.rng, &class_ids, remaining_arcs)?
        };
        let sources = self.sources();
        let tsources = self.transship_sources();
        let tnodes = self.transship_nodes();
        let tsinks = self.transship_sinks();
        let sinks = self.sinks();
        self.generate_arcs(&sources, &tsources, distribution[S2TS])?;
        self.generate_arcs(&sources, &tnodes, distribution[S2TN])?;
        self.generate_arcs(&sources, &sinks, distribution[S2S])?;
        self.generate_arcs(&tnodes, &tsources, distribution[TN2TS])?;
        self.generate_arcs(&tnodes, &tnodes, distribution[TN2TN])?;
        self.generate_arcs(&tnodes, &sinks, distribution[TN2S])?;
        self.generate_arcs(&tsinks, &tsources, distribution[TS2TS])?;
        self.generate_arcs(&tsinks, &tnodes, distribution[TS2TN])?;
        self.generate_arcs(&tsinks, &sinks, distribution[TS2S])?;
        if self.config.arc_num() as usize != self.graph.edge_count() {
            return invalid("generated an unexpected number of arcs");
        }
        Ok(())
    }

    fn generate_arcs(
        &mut self,
        tails: &[usize],
        heads: &[usize],
        arcs_to_generate: i32,
    ) -> Result<()> {
        // an upper bound on the number of arcs that can still start at each tail
        let heads_set: HashSet<usize> = heads.iter().copied().collect();
        let out_degrees: Vec<i32> = tails
            .iter()
            .map(|&n| self.possible_arc_num(n, &heads_set))
            .collect::<Result<_>>()?;
        let degree_sum: i64 = out_degrees.iter().map(|&d| d as i64).sum();
        let tail_ids: Vec<usize> = (0..tails.len()).collect();
        let distribution = {
            let mut distributor = Distributor::new();
            distributor.add_upper_bound(|t: &usize| out_degrees[*t]);
            distributor.add_upper_bound(|t: &usize| {
                let tail_weight = out_degrees[*t] as f64 / degree_sum as f64;
                let bound = (2.0 * tail_weight * arcs_to_generate as f64) as i32;
                bound.wrapping_add(1)
            });
            distributor.get_distribution(self.rng, &tail_ids, arcs_to_generate)?
        };
        for (i, &tail) in tails.iter().enumerate() {
            let mut tail_arc_num = distribution[i];
            let mut head_generator = ElementsSequenceGenerator::new(heads.iter().copied());
            while tail_arc_num > 0 && head_generator.has_next() {
                let head = head_generator.next(self.rng).unwrap();
                if self.is_valid_arc(tail, head) {
                    tail_arc_num -= 1;
                    self.add_arc(tail, head)?;
                }
            }
            if tail_arc_num != 0 {
                return invalid("could not generate the requested number of arcs");
            }
        }
        Ok(())
    }

    fn possible_arc_num(&self, node: usize, nodes: &HashSet<usize>) -> Result<i32> {
        let mut possible = nodes.len() as i32;
        if nodes.contains(&node) {
            possible -= 1;
        }
        let v = &self.nodes[node].vertex;
        for arc in self.graph.out_edges_of(v) {
            let head = self.graph.edge_target(arc).unwrap();
            if nodes.contains(&self.index[head]) {
                possible -= 1;
            }
        }
        Ok(possible)
    }

    fn is_valid_arc(&self, tail: usize, head: usize) -> bool {
        tail != head
            && !self
                .graph
                .contains_edge_between(&self.nodes[tail].vertex, &self.nodes[head].vertex)
    }

    fn new_arc(&mut self, tail: usize, head: usize) -> Result<G::Edge> {
        let (t, h) = (
            self.nodes[tail].vertex.clone(),
            self.nodes[head].vertex.clone(),
        );
        self.graph
            .add_edge(&t, &h)?
            .ok_or_else(|| GraphError::InvalidArgument("arc could not be added".into()))
    }

    fn add_skeleton_arc(&mut self, chain_source: usize, tail: usize, head: usize) -> Result<()> {
        debug_assert!(self.is_valid_arc(tail, head));
        let arc = self.new_arc(tail, head)?;
        let capacity = self.get_capacity()?.max(self.nodes[chain_source].supply);
        self.capacity.insert(arc.clone(), capacity);
        let cost = self.get_cost()?;
        self.cost.insert(arc.clone(), cost);
        self.register_skeleton_arc(tail, head);
        self.skeleton.push(arc);
        if self.nodes[head].ty == NodeType::TransshipNode {
            self.nodes[chain_source].chain_nodes.push(head);
        }
        Ok(())
    }

    fn add_arc(&mut self, tail: usize, head: usize) -> Result<()> {
        debug_assert!(self.is_valid_arc(tail, head));
        let arc = self.new_arc(tail, head)?;
        let capacity = self.get_capacity()?;
        self.capacity.insert(arc.clone(), capacity);
        let cost = self.get_cost()?;
        self.cost.insert(arc, cost);
        Ok(())
    }

    fn register_skeleton_arc(&mut self, tail: usize, head: usize) {
        use NodeType::*;
        let class = match (self.nodes[tail].ty, self.nodes[head].ty) {
            (PureSource | TransshipSource, TransshipNode) => S2TN,
            (PureSource | TransshipSource, TransshipSink | PureSink) => S2S,
            (TransshipNode, TransshipNode) => TN2TN,
            (TransshipNode, TransshipSink | PureSink) => TN2S,
            _ => unreachable!("invalid skeleton arc"),
        };
        self.ub[class] -= 1;
    }

    fn get_capacity(&mut self) -> Result<i32> {
        let percent = self.between(1, 100)?;
        if percent <= self.config.percent_capacitated() {
            self.between(self.config.min_cap(), self.config.max_cap())
        } else {
            Ok(i32::MAX)
        }
    }

    fn get_cost(&mut self) -> Result<i32> {
        let percent = self.between(1, 100)?;
        if percent <= self.config.percent_with_inf_cost() {
            Ok(i32::MAX)
        } else {
            self.between(self.config.min_cost(), self.config.max_cost())
        }
    }

    /// `rng.nextInt(end - start + 1) + start` with Java `int` arithmetic.
    fn between(&mut self, start: i32, end_inclusive: i32) -> Result<i32> {
        let bound = end_inclusive.wrapping_sub(start).wrapping_add(1);
        if bound <= 0 {
            return Err(GraphError::must_be_positive("bound"));
        }
        Ok(self.rng.next_int_bound(bound as usize) as i32 + start)
    }

    fn range(&self, from: i32, to: i32) -> Vec<usize> {
        (from as usize..to as usize).collect()
    }
    fn transship_sources(&self) -> Vec<usize> {
        self.range(self.config.pure_source_num(), self.config.source_num())
    }
    fn sources(&self) -> Vec<usize> {
        self.range(0, self.config.source_num())
    }
    fn transship_nodes(&self) -> Vec<usize> {
        self.range(
            self.config.source_num(),
            self.config.source_num() + self.config.transship_node_num(),
        )
    }
    fn transship_sinks(&self) -> Vec<usize> {
        self.range(
            self.config.source_num() + self.config.transship_node_num(),
            self.nodes.len() as i32 - self.config.pure_sink_num(),
        )
    }
    fn sinks(&self) -> Vec<usize> {
        self.range(
            self.config.source_num() + self.config.transship_node_num(),
            self.nodes.len() as i32,
        )
    }
}
