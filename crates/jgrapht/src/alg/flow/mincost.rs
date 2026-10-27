//! Minimum cost flow (`org.jgrapht.alg.flow.mincost`).

use crate::error::{GraphError, Result};
use crate::graph::Graph;
use crate::util::OrderedF64;
use crate::util::heap::{JavaPairingHandle, JavaPairingHeap};
use rheaps::{AddressableHeap, DecreaseKeyHeap};
use std::collections::{HashMap, HashSet};

/// `MinimumCostFlowProblem`: a directed network with node supplies and arc capacity bounds and
/// costs.
pub trait MinimumCostFlowProblem {
    type Graph: Graph;

    fn graph(&self) -> &Self::Graph;
    /// Supply of a vertex (negative for a demand).
    fn node_supply(&self, v: &<Self::Graph as Graph>::Vertex) -> i32;
    fn arc_capacity_lower_bound(&self, e: &<Self::Graph as Graph>::Edge) -> i32;
    fn arc_capacity_upper_bound(&self, e: &<Self::Graph as Graph>::Edge) -> i32;
    fn arc_cost(&self, e: &<Self::Graph as Graph>::Edge) -> f64;
}

type VertexFn<'g, G> = Box<dyn Fn(&<G as Graph>::Vertex) -> i32 + 'g>;
type EdgeIntFn<'g, G> = Box<dyn Fn(&<G as Graph>::Edge) -> i32 + 'g>;
type EdgeCostFn<'g, G> = Box<dyn Fn(&<G as Graph>::Edge) -> f64 + 'g>;

/// `MinimumCostFlowProblemImpl`.
pub struct MinimumCostFlowProblemImpl<'g, G: Graph> {
    graph: &'g G,
    node_supplies: VertexFn<'g, G>,
    arc_capacity_upper_bounds: EdgeIntFn<'g, G>,
    arc_capacity_lower_bounds: EdgeIntFn<'g, G>,
    arc_costs: EdgeCostFn<'g, G>,
}

impl<'g, G: Graph> MinimumCostFlowProblemImpl<'g, G> {
    /// Lower bounds are zero and the costs are the edge weights.
    pub fn new(
        graph: &'g G,
        node_supplies: impl Fn(&G::Vertex) -> i32 + 'g,
        arc_capacity_upper_bounds: impl Fn(&G::Edge) -> i32 + 'g,
    ) -> Self {
        Self::with_lower_bounds(graph, node_supplies, arc_capacity_upper_bounds, |_| 0)
    }

    /// The costs are the edge weights.
    pub fn with_lower_bounds(
        graph: &'g G,
        node_supplies: impl Fn(&G::Vertex) -> i32 + 'g,
        arc_capacity_upper_bounds: impl Fn(&G::Edge) -> i32 + 'g,
        arc_capacity_lower_bounds: impl Fn(&G::Edge) -> i32 + 'g,
    ) -> Self {
        Self::with_costs(
            graph,
            node_supplies,
            arc_capacity_upper_bounds,
            arc_capacity_lower_bounds,
            move |e| graph.edge_weight(e).unwrap_or(1.0),
        )
    }

    pub fn with_costs(
        graph: &'g G,
        node_supplies: impl Fn(&G::Vertex) -> i32 + 'g,
        arc_capacity_upper_bounds: impl Fn(&G::Edge) -> i32 + 'g,
        arc_capacity_lower_bounds: impl Fn(&G::Edge) -> i32 + 'g,
        arc_costs: impl Fn(&G::Edge) -> f64 + 'g,
    ) -> Self {
        MinimumCostFlowProblemImpl {
            graph,
            node_supplies: Box::new(node_supplies),
            arc_capacity_upper_bounds: Box::new(arc_capacity_upper_bounds),
            arc_capacity_lower_bounds: Box::new(arc_capacity_lower_bounds),
            arc_costs: Box::new(arc_costs),
        }
    }
}

impl<'g, G: Graph> MinimumCostFlowProblem for MinimumCostFlowProblemImpl<'g, G> {
    type Graph = G;

    fn graph(&self) -> &G {
        self.graph
    }
    fn node_supply(&self, v: &G::Vertex) -> i32 {
        (self.node_supplies)(v)
    }
    fn arc_capacity_lower_bound(&self, e: &G::Edge) -> i32 {
        (self.arc_capacity_lower_bounds)(e)
    }
    fn arc_capacity_upper_bound(&self, e: &G::Edge) -> i32 {
        (self.arc_capacity_upper_bounds)(e)
    }
    fn arc_cost(&self, e: &G::Edge) -> f64 {
        (self.arc_costs)(e)
    }
}

/// `MinimumCostFlowAlgorithm.MinimumCostFlow`: the cost and the flow on every edge.
#[derive(Debug, Clone)]
pub struct MinimumCostFlow<E> {
    pub cost: f64,
    pub flow_map: HashMap<E, f64>,
}

impl<E: Eq + std::hash::Hash> MinimumCostFlow<E> {
    pub fn flow(&self, edge: &E) -> Option<f64> {
        self.flow_map.get(edge).copied()
    }
}

/// `MinimumCostFlowAlgorithm`.
pub trait MinimumCostFlowAlgorithm<V, E> {
    fn minimum_cost_flow<P>(&mut self, problem: &P) -> Result<MinimumCostFlow<E>>
    where
        P: MinimumCostFlowProblem,
        P::Graph: Graph<Vertex = V, Edge = E>;

    fn flow_cost<P>(&mut self, problem: &P) -> Result<f64>
    where
        P: MinimumCostFlowProblem,
        P::Graph: Graph<Vertex = V, Edge = E>,
    {
        Ok(self.minimum_cost_flow(problem)?.cost)
    }

    /// The flow map of the last computation.
    fn flow_map(&self) -> Option<&HashMap<E, f64>>;
    /// The head of an edge of the last problem.
    fn flow_direction(&self, edge: &E) -> Option<&V>;
}

/// Capacity of an arc that is regarded as infinite.
pub const CAP_INF: i32 = 1000 * 1000 * 1000;
/// Cost of the artificial arcs.
pub const COST_INF: f64 = 1e9;
pub const DEFAULT_SCALING_FACTOR: i32 = 8;

struct Node {
    handle: Option<JavaPairingHandle>,
    parent_arc: Option<usize>,
    label_type: i32,
    excess: i32,
    potential: f64,
    first_saturated: Option<usize>,
    first_non_saturated: Option<usize>,
}

impl Node {
    fn new(excess: i32) -> Self {
        Node {
            handle: None,
            parent_arc: None,
            label_type: 0,
            excess,
            potential: 0.0,
            first_saturated: None,
            first_non_saturated: None,
        }
    }
}

struct Arc {
    head: usize,
    cost: f64,
    rev: usize,
    prev: Option<usize>,
    next: Option<usize>,
    residual_capacity: i32,
}

/// The network of one computation: nodes and arcs in arenas, arcs in intrusive lists of
/// saturated and non-saturated arcs per tail node.
struct Network {
    nodes: Vec<Node>,
    arcs: Vec<Arc>,
}

impl Network {
    fn new_arc(&mut self, head: usize, residual_capacity: i32, cost: f64) -> usize {
        self.arcs.push(Arc {
            head,
            cost,
            rev: usize::MAX,
            prev: None,
            next: None,
            residual_capacity,
        });
        self.arcs.len() - 1
    }

    /// `Node.addArcTo`: a forward arc and its zero-capacity reverse arc.
    fn add_arc_to(&mut self, node: usize, opposite: usize, capacity: i32, cost: f64) -> usize {
        let forward = self.new_arc(opposite, capacity, cost);
        if capacity > 0 {
            if let Some(f) = self.nodes[node].first_non_saturated {
                self.arcs[f].prev = Some(forward);
            }
            self.arcs[forward].next = self.nodes[node].first_non_saturated;
            self.nodes[node].first_non_saturated = Some(forward);
        } else {
            if let Some(f) = self.nodes[node].first_saturated {
                self.arcs[f].prev = Some(forward);
            }
            self.arcs[forward].next = self.nodes[node].first_saturated;
            self.nodes[node].first_saturated = Some(forward);
        }
        let reverse = self.new_arc(node, 0, -cost);
        if let Some(f) = self.nodes[opposite].first_saturated {
            self.arcs[f].prev = Some(reverse);
        }
        self.arcs[reverse].next = self.nodes[opposite].first_saturated;
        self.nodes[opposite].first_saturated = Some(reverse);
        self.arcs[forward].rev = reverse;
        self.arcs[reverse].rev = forward;
        forward
    }

    fn reduced_cost(&self, a: usize) -> f64 {
        let arc = &self.arcs[a];
        arc.cost + self.nodes[arc.head].potential - self.nodes[self.arcs[arc.rev].head].potential
    }

    fn send_flow(&mut self, a: usize, value: i32) {
        self.decrease_residual_capacity(a, value);
        let rev = self.arcs[a].rev;
        self.increase_residual_capacity(rev, value);
    }

    fn decrease_residual_capacity(&mut self, a: usize, value: i32) {
        if self.arcs[a].residual_capacity >= CAP_INF {
            return;
        }
        self.arcs[a].residual_capacity -= value;
        if self.arcs[a].residual_capacity == 0 {
            let tail = self.arcs[self.arcs[a].rev].head;
            let (next, prev) = (self.arcs[a].next, self.arcs[a].prev);
            if let Some(n) = next {
                self.arcs[n].prev = prev;
            }
            match prev {
                Some(p) => self.arcs[p].next = next,
                None => self.nodes[tail].first_non_saturated = next,
            }
            self.arcs[a].next = self.nodes[tail].first_saturated;
            if let Some(f) = self.nodes[tail].first_saturated {
                self.arcs[f].prev = Some(a);
            }
            self.nodes[tail].first_saturated = Some(a);
            self.arcs[a].prev = None;
        }
    }

    fn increase_residual_capacity(&mut self, a: usize, value: i32) {
        if self.arcs[a].residual_capacity >= CAP_INF {
            return;
        }
        if self.arcs[a].residual_capacity == 0 {
            let tail = self.arcs[self.arcs[a].rev].head;
            let (next, prev) = (self.arcs[a].next, self.arcs[a].prev);
            if let Some(n) = next {
                self.arcs[n].prev = prev;
            }
            match prev {
                Some(p) => self.arcs[p].next = next,
                None => self.nodes[tail].first_saturated = next,
            }
            self.arcs[a].next = self.nodes[tail].first_non_saturated;
            if let Some(f) = self.nodes[tail].first_non_saturated {
                self.arcs[f].prev = Some(a);
            }
            self.nodes[tail].first_non_saturated = Some(a);
            self.arcs[a].prev = None;
        }
        self.arcs[a].residual_capacity += value;
    }

    fn is_infinite_capacity_arc(&self, a: usize) -> bool {
        self.arcs[a].residual_capacity >= CAP_INF
    }
}

/// `CapacityScalingMinimumCostFlow`: the capacity scaling successive shortest path algorithm of
/// Ahuja, Magnanti and Orlin, `O(m log U (m + n log n))`. Only directed networks, no self-loops
/// and no negative-cost arcs of infinite capacity.
pub struct CapacityScalingMinimumCostFlow<V, E> {
    scaling_factor: i32,
    counter: i32,
    minimum_cost_flow: Option<MinimumCostFlow<E>>,
    net: Network,
    n: usize,
    graph_vertices: Vec<V>,
    edge_target: HashMap<E, V>,
    arcs: Vec<usize>,
}

impl<V: Clone + Eq + std::hash::Hash, E: Clone + Eq + std::hash::Hash> Default
    for CapacityScalingMinimumCostFlow<V, E>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + Eq + std::hash::Hash, E: Clone + Eq + std::hash::Hash>
    CapacityScalingMinimumCostFlow<V, E>
{
    pub fn new() -> Self {
        Self::with_scaling_factor(DEFAULT_SCALING_FACTOR)
    }

    pub fn with_scaling_factor(scaling_factor: i32) -> Self {
        CapacityScalingMinimumCostFlow {
            scaling_factor,
            counter: 1,
            minimum_cost_flow: None,
            net: Network {
                nodes: Vec::new(),
                arcs: Vec::new(),
            },
            n: 0,
            graph_vertices: Vec::new(),
            edge_target: HashMap::new(),
            arcs: Vec::new(),
        }
    }

    /// The dual variable (node potential) of every vertex after a computation.
    pub fn dual_solution(&self) -> Option<HashMap<V, f64>> {
        self.minimum_cost_flow.as_ref()?;
        Some(
            (0..self.n)
                .map(|i| (self.graph_vertices[i].clone(), self.net.nodes[i].potential))
                .collect(),
        )
    }

    /// Whether no residual arc has a reduced cost below `-eps`.
    pub fn test_optimality(&self, eps: f64) -> Result<bool> {
        if self.minimum_cost_flow.is_none() {
            return Err(GraphError::InvalidArgument(
                "Cannot return a dual solution before the minimum cost flow is computed!".into(),
            ));
        }
        for node in &self.net.nodes {
            let mut arc = node.first_non_saturated;
            while let Some(a) = arc {
                if self.net.reduced_cost(a) < -eps {
                    return Ok(false);
                }
                arc = self.net.arcs[a].next;
            }
        }
        Ok(true)
    }

    /// Solves `problem` (`getMinimumCostFlow`).
    pub fn get_minimum_cost_flow<P>(&mut self, problem: &P) -> Result<MinimumCostFlow<E>>
    where
        P: MinimumCostFlowProblem,
        P::Graph: Graph<Vertex = V, Edge = E>,
    {
        let graph = problem.graph();
        if !graph.graph_type().directed {
            return Err(GraphError::InvalidArgument(
                "The algorithm doesn't support undirected flow networks".into(),
            ));
        }
        self.n = graph.vertex_count();
        self.calculate_minimum_cost_flow(problem)?;
        Ok(self.minimum_cost_flow.clone().unwrap())
    }

    fn calculate_minimum_cost_flow<P>(&mut self, problem: &P) -> Result<()>
    where
        P: MinimumCostFlowProblem,
        P::Graph: Graph<Vertex = V, Edge = E>,
    {
        self.minimum_cost_flow = None;
        let edges = self.init(problem)?;
        if self.scaling_factor > 1 {
            let u = self.get_u();
            let mut delta = self.scaling_factor;
            while u >= delta {
                delta = delta.wrapping_mul(self.scaling_factor);
            }
            delta /= self.scaling_factor;
            while delta >= 1 {
                let (positive, mut negative) = self.scale(delta);
                self.push_all_flow(&positive, &mut negative, delta);
                delta /= self.scaling_factor;
            }
        } else {
            let (positive, mut negative) = self.scale(1);
            self.push_all_flow(&positive, &mut negative, 1);
        }
        self.minimum_cost_flow = Some(self.finish(problem, &edges)?);
        Ok(())
    }

    fn init<P>(&mut self, problem: &P) -> Result<Vec<E>>
    where
        P: MinimumCostFlowProblem,
        P::Graph: Graph<Vertex = V, Edge = E>,
    {
        let graph = problem.graph();
        let n = self.n;
        self.net = Network {
            nodes: Vec::with_capacity(n + 1),
            arcs: Vec::new(),
        };
        self.arcs = Vec::with_capacity(graph.edge_count());
        self.graph_vertices = Vec::with_capacity(n);
        self.edge_target.clear();
        let mut node_map: HashMap<&V, usize> = HashMap::with_capacity(n);
        let mut supply_sum: i32 = 0;
        // nodes 0..n are the vertices, node n is the dummy node
        for _ in 0..=n {
            self.net.nodes.push(Node::new(0));
        }
        for (i, vertex) in graph.vertices().enumerate() {
            self.graph_vertices.push(vertex.clone());
            let supply = problem.node_supply(vertex);
            supply_sum += supply;
            self.net.nodes[i].excess = supply;
            node_map.insert(vertex, i);
            self.net.add_arc_to(i, n, CAP_INF, COST_INF);
            self.net.add_arc_to(n, i, CAP_INF, COST_INF);
        }
        if supply_sum.abs() > 0 {
            return Err(GraphError::InvalidArgument(
                "Total node supply isn't equal to 0".into(),
            ));
        }
        let mut graph_edges = Vec::with_capacity(graph.edge_count());
        for edge in graph.edges() {
            graph_edges.push(edge.clone());
            let mut node = node_map[graph.edge_source(edge).unwrap()];
            let mut opposite = node_map[graph.edge_target(edge).unwrap()];
            let upper_cap = problem.arc_capacity_upper_bound(edge);
            let lower_cap = problem.arc_capacity_lower_bound(edge);
            let mut cost = graph.edge_weight(edge).unwrap_or(1.0);
            if upper_cap < 0 {
                return Err(GraphError::InvalidArgument(
                    "Negative edge capacities are not allowed".into(),
                ));
            } else if lower_cap > upper_cap {
                return Err(GraphError::InvalidArgument(
                    "Lower edge capacity must not exceed upper edge capacity".into(),
                ));
            } else if lower_cap >= CAP_INF {
                return Err(GraphError::InvalidArgument(
                    "The problem is unbounded due to the infinite lower capacity".into(),
                ));
            } else if upper_cap >= CAP_INF && cost < 0.0 {
                return Err(GraphError::InvalidArgument(
                    "The algorithm doesn't support infinite capacity arcs with negative cost"
                        .into(),
                ));
            } else if cost.abs() >= COST_INF {
                return Err(GraphError::InvalidArgument(
                    "Specified flow network contains an edge of infinite cost".into(),
                ));
            } else if node == opposite {
                return Err(GraphError::InvalidArgument(
                    "Self-loops aren't allowed".into(),
                ));
            }
            self.net.nodes[node].excess -= lower_cap;
            self.net.nodes[opposite].excess += lower_cap;
            if cost < 0.0 {
                self.net.nodes[node].excess -= upper_cap - lower_cap;
                self.net.nodes[opposite].excess += upper_cap - lower_cap;
                std::mem::swap(&mut node, &mut opposite);
                cost *= -1.0;
            }
            let arc = self
                .net
                .add_arc_to(node, opposite, upper_cap - lower_cap, cost);
            self.arcs.push(arc);
            self.edge_target
                .insert(edge.clone(), graph.edge_target(edge).unwrap().clone());
        }
        Ok(graph_edges)
    }

    fn get_u(&self) -> i32 {
        let mut result = 0;
        for node in &self.net.nodes {
            result = result.max(node.excess.abs());
        }
        for &arc in &self.arcs {
            if !self.net.is_infinite_capacity_arc(arc) {
                result = result.max(self.net.arcs[arc].residual_capacity);
            }
        }
        result
    }

    fn scale(&mut self, delta: i32) -> (Vec<usize>, HashSet<usize>) {
        for node in 0..self.net.nodes.len() {
            let mut next_arc = self.net.nodes[node].first_non_saturated;
            while let Some(arc) = next_arc {
                next_arc = self.net.arcs[arc].next;
                let residual_capacity = self.net.arcs[arc].residual_capacity;
                if residual_capacity >= delta && self.net.reduced_cost(arc) < 0.0 {
                    self.net.send_flow(arc, residual_capacity);
                    let head = self.net.arcs[arc].head;
                    let rev_head = self.net.arcs[self.net.arcs[arc].rev].head;
                    self.net.nodes[head].excess += residual_capacity;
                    self.net.nodes[rev_head].excess -= residual_capacity;
                }
            }
        }
        let mut positive_excess_nodes = Vec::new();
        let mut negative_excess_nodes = HashSet::new();
        for (i, node) in self.net.nodes.iter().enumerate() {
            if node.excess >= delta {
                positive_excess_nodes.push(i);
            } else if node.excess <= -delta {
                negative_excess_nodes.insert(i);
            }
        }
        (positive_excess_nodes, negative_excess_nodes)
    }

    fn push_all_flow(
        &mut self,
        positive_excess_nodes: &[usize],
        negative_excess_nodes: &mut HashSet<usize>,
        delta: i32,
    ) {
        for &node in positive_excess_nodes {
            while self.net.nodes[node].excess >= delta {
                if negative_excess_nodes.is_empty() {
                    return;
                }
                self.push_dijkstra(node, negative_excess_nodes, delta);
            }
        }
    }

    fn push_dijkstra(
        &mut self,
        start: usize,
        negative_excess_nodes: &mut HashSet<usize>,
        delta: i32,
    ) {
        let temporarily_labeled_type = self.counter;
        self.counter += 1;
        let permanently_labeled_type = self.counter;
        self.counter += 1;
        let mut heap: JavaPairingHeap<OrderedF64, usize> = JavaPairingHeap::new();
        let mut permanently_labeled: Vec<usize> = Vec::new();
        self.net.nodes[start].parent_arc = None;
        self.net.nodes[start].handle = Some(heap.insert(OrderedF64(0.0), start));
        while let Some((key, current_node)) = heap.pop() {
            let distance = key.0;
            if negative_excess_nodes.contains(&current_node) {
                self.augment_path(start, current_node);
                if self.net.nodes[current_node].excess > -delta {
                    negative_excess_nodes.remove(&current_node);
                }
                for &node in &permanently_labeled {
                    self.net.nodes[node].potential += distance;
                }
                return;
            }
            self.net.nodes[current_node].label_type = permanently_labeled_type;
            permanently_labeled.push(current_node);
            let mut current_arc = self.net.nodes[current_node].first_non_saturated;
            while let Some(arc) = current_arc {
                current_arc = self.net.arcs[arc].next;
                if self.net.arcs[arc].residual_capacity < delta {
                    continue;
                }
                let opposite = self.net.arcs[arc].head;
                if self.net.nodes[opposite].label_type != permanently_labeled_type {
                    let candidate = distance + self.net.reduced_cost(arc);
                    if self.net.nodes[opposite].label_type == temporarily_labeled_type {
                        let handle = self.net.nodes[opposite]
                            .handle
                            .expect("labeled node is in the heap");
                        if candidate < heap.key(handle).expect("live handle").0 {
                            heap.decrease_key(handle, OrderedF64(candidate))
                                .expect("smaller key");
                            self.net.nodes[opposite].parent_arc = Some(arc);
                        }
                    } else {
                        self.net.nodes[opposite].label_type = temporarily_labeled_type;
                        self.net.nodes[opposite].handle =
                            Some(heap.insert(OrderedF64(candidate), opposite));
                        self.net.nodes[opposite].parent_arc = Some(arc);
                    }
                }
            }
            self.net.nodes[current_node].potential -= distance;
        }
    }

    fn augment_path(&mut self, start: usize, end: usize) {
        let mut value_to_augment = self.net.nodes[start]
            .excess
            .min(-self.net.nodes[end].excess);
        let mut arc = self.net.nodes[end].parent_arc;
        while let Some(a) = arc {
            value_to_augment = value_to_augment.min(self.net.arcs[a].residual_capacity);
            arc = self.net.nodes[self.net.arcs[self.net.arcs[a].rev].head].parent_arc;
        }
        self.net.nodes[end].excess += value_to_augment;
        let mut arc = self.net.nodes[end].parent_arc;
        while let Some(a) = arc {
            self.net.send_flow(a, value_to_augment);
            arc = self.net.nodes[self.net.arcs[self.net.arcs[a].rev].head].parent_arc;
        }
        self.net.nodes[start].excess -= value_to_augment;
    }

    fn finish<P>(&self, problem: &P, graph_edges: &[E]) -> Result<MinimumCostFlow<E>>
    where
        P: MinimumCostFlowProblem,
        P::Graph: Graph<Vertex = V, Edge = E>,
    {
        let graph = problem.graph();
        let mut arc = self.net.nodes[self.n].first_non_saturated;
        while let Some(a) = arc {
            if self.net.arcs[self.net.arcs[a].rev].residual_capacity > 0 {
                return Err(GraphError::InvalidArgument(
                    "Specified flow network problem has no feasible solution".into(),
                ));
            }
            arc = self.net.arcs[a].next;
        }
        let mut flow_map = HashMap::with_capacity(graph_edges.len());
        let mut total_cost = 0.0;
        for (i, graph_edge) in graph_edges.iter().enumerate() {
            let arc = self.arcs[i];
            // the residual capacity of the reverse arc equals the flow on the arc
            let mut flow_on_arc = self.net.arcs[self.net.arcs[arc].rev].residual_capacity as f64;
            let weight = graph.edge_weight(graph_edge).unwrap_or(1.0);
            if weight < 0.0 {
                flow_on_arc = (problem.arc_capacity_upper_bound(graph_edge)
                    - problem.arc_capacity_lower_bound(graph_edge))
                    as f64
                    - flow_on_arc;
            }
            flow_on_arc += problem.arc_capacity_lower_bound(graph_edge) as f64;
            flow_map.insert(graph_edge.clone(), flow_on_arc);
            total_cost += flow_on_arc * weight;
        }
        Ok(MinimumCostFlow {
            cost: total_cost,
            flow_map,
        })
    }
}

impl<V: Clone + Eq + std::hash::Hash, E: Clone + Eq + std::hash::Hash>
    MinimumCostFlowAlgorithm<V, E> for CapacityScalingMinimumCostFlow<V, E>
{
    fn minimum_cost_flow<P>(&mut self, problem: &P) -> Result<MinimumCostFlow<E>>
    where
        P: MinimumCostFlowProblem,
        P::Graph: Graph<Vertex = V, Edge = E>,
    {
        self.get_minimum_cost_flow(problem)
    }

    fn flow_map(&self) -> Option<&HashMap<E, f64>> {
        self.minimum_cost_flow.as_ref().map(|f| &f.flow_map)
    }

    fn flow_direction(&self, edge: &E) -> Option<&V> {
        self.edge_target.get(edge)
    }
}
