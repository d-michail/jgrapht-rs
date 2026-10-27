use crate::alg::connectivity::GabowStrongConnectivityInspector;
use crate::alg::interfaces::StrongConnectivityAlgorithm;
use crate::error::Result;
use crate::graph::{DirectedGraph, Graph, GraphExt};
use crate::util::ToleranceDoubleComparator;
use crate::walk::GraphWalk;
use std::collections::{HashMap, VecDeque};

/// `MinimumCycleMeanAlgorithm`: the cycle with the smallest mean edge weight.
pub trait MinimumCycleMeanAlgorithm<G: Graph> {
    /// The smallest cycle mean, `+infinity` if the graph has no cycle.
    fn get_cycle_mean(&mut self) -> f64;
    /// A cycle attaining the smallest mean, `None` if the graph is acyclic.
    fn get_cycle(&mut self) -> Option<GraphWalk<G::Vertex, G::Edge>>;
}

/// `HowardMinimumMeanCycle`: Howard's policy iteration for the minimum mean cycle of a directed
/// graph.
pub struct HowardMinimumMeanCycle<'g, G: DirectedGraph, S = GabowStrongConnectivityInspector<'g, G>>
{
    graph: &'g G,
    strong_connectivity_algorithm: S,
    maximum_iterations: usize,
    comparator: ToleranceDoubleComparator,
    is_current_cycle_found: bool,
    current_cycle_weight: f64,
    current_cycle_length: usize,
    current_cycle_vertex: Option<G::Vertex>,
    policy_graph: HashMap<G::Vertex, G::Edge>,
    vertex_level: HashMap<G::Vertex, i64>,
    vertex_distance: HashMap<G::Vertex, f64>,
}

impl<'g, G: DirectedGraph> HowardMinimumMeanCycle<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self::with_maximum_iterations(graph, usize::MAX)
    }

    pub fn with_maximum_iterations(graph: &'g G, maximum_iterations: usize) -> Self {
        Self::with_strong_connectivity(
            graph,
            maximum_iterations,
            GabowStrongConnectivityInspector::new(graph),
            1e-9,
        )
        .expect("the default tolerance is positive")
    }
}

impl<'g, G: DirectedGraph, S: StrongConnectivityAlgorithm<G = G>> HowardMinimumMeanCycle<'g, G, S> {
    pub fn with_strong_connectivity(
        graph: &'g G,
        maximum_iterations: usize,
        strong_connectivity_algorithm: S,
        tolerance_epsilon: f64,
    ) -> Result<Self> {
        Ok(HowardMinimumMeanCycle {
            graph,
            strong_connectivity_algorithm,
            maximum_iterations,
            comparator: ToleranceDoubleComparator::new(tolerance_epsilon)?,
            is_current_cycle_found: false,
            current_cycle_weight: 0.0,
            current_cycle_length: 0,
            current_cycle_vertex: None,
            policy_graph: HashMap::new(),
            vertex_level: HashMap::new(),
            vertex_distance: HashMap::new(),
        })
    }

    fn construct_policy_graph<C: Graph<Vertex = G::Vertex, Edge = G::Edge>>(
        &mut self,
        component: &C,
    ) {
        for v in component.vertices() {
            self.vertex_distance.insert(v.clone(), f64::INFINITY);
        }
        for u in component.vertices() {
            for e in component.in_edges_of(u).unwrap() {
                let v = component
                    .opposite_vertex(e, u)
                    .expect("edge of the component");
                let e_weight = component.edge_weight(e).unwrap_or(1.0);
                if e_weight < self.vertex_distance[v] {
                    self.vertex_distance.insert(v.clone(), e_weight);
                    self.policy_graph.insert(v.clone(), e.clone());
                }
            }
        }
    }

    fn policy_successor<C: Graph<Vertex = G::Vertex, Edge = G::Edge>>(
        &self,
        component: &C,
        u: &G::Vertex,
    ) -> G::Vertex {
        component
            .opposite_vertex(&self.policy_graph[u], u)
            .expect("policy edge of the component")
            .clone()
    }

    fn construct_cycle<C: Graph<Vertex = G::Vertex, Edge = G::Edge>>(&mut self, component: &C) {
        for v in component.vertices() {
            self.vertex_level.insert(v.clone(), -1);
        }
        self.is_current_cycle_found = false;
        let mut current_cycle_level = 0i64;
        for start in component.vertices() {
            let mut u = start.clone();
            if self.vertex_level[&u] >= 0 {
                continue; // vertex already belongs to a cycle
            }
            while self.vertex_level[&u] < 0 {
                self.vertex_level.insert(u.clone(), current_cycle_level);
                u = self.policy_successor(component, &u);
            }
            if self.vertex_level[&u] == current_cycle_level {
                let mut current_weight =
                    component.edge_weight(&self.policy_graph[&u]).unwrap_or(1.0);
                let mut current_size = 1;
                let mut v = self.policy_successor(component, &u);
                while v != u {
                    current_weight += component.edge_weight(&self.policy_graph[&v]).unwrap_or(1.0);
                    current_size += 1;
                    v = self.policy_successor(component, &v);
                }
                if !self.is_current_cycle_found
                    || current_weight * (self.current_cycle_length as f64)
                        < self.current_cycle_weight * (current_size as f64)
                {
                    self.is_current_cycle_found = true;
                    self.current_cycle_weight = current_weight;
                    self.current_cycle_length = current_size;
                    self.current_cycle_vertex = Some(u.clone());
                }
            }
            current_cycle_level += 1;
        }
    }

    fn compute_vertex_distance<C: Graph<Vertex = G::Vertex, Edge = G::Edge>>(
        &mut self,
        component: &C,
    ) -> bool {
        let mut queue: VecDeque<G::Vertex> = VecDeque::new();
        let mut reached: HashMap<G::Vertex, bool> =
            component.vertices().map(|v| (v.clone(), false)).collect();
        let cycle_vertex = self
            .current_cycle_vertex
            .clone()
            .expect("a cycle was found");
        queue.push_back(cycle_vertex.clone());
        reached.insert(cycle_vertex, true);
        let current_mean = self.current_cycle_weight / self.current_cycle_length as f64;
        while let Some(u) = queue.pop_front() {
            for e in component.in_edges_of(&u).unwrap() {
                let v = component
                    .opposite_vertex(e, &u)
                    .expect("edge of the component");
                if self.policy_graph[v] == *e && !reached[v] {
                    reached.insert(v.clone(), true);
                    let updated_distance = self.vertex_distance[&u]
                        + component.edge_weight(e).unwrap_or(1.0)
                        - current_mean;
                    self.vertex_distance.insert(v.clone(), updated_distance);
                    queue.push_back(v.clone());
                }
            }
        }
        let mut improved = false;
        for u in component.vertices() {
            for e in component.in_edges_of(u).unwrap() {
                let v = component
                    .opposite_vertex(e, u)
                    .expect("edge of the component");
                let old_distance = self.vertex_distance[v];
                let updated_distance = self.vertex_distance[u]
                    + component.edge_weight(e).unwrap_or(1.0)
                    - current_mean;
                if old_distance > updated_distance {
                    if self.comparator.compare(old_distance, updated_distance) > 0 {
                        improved = true;
                    }
                    self.vertex_distance.insert(v.clone(), updated_distance);
                    self.policy_graph.insert(v.clone(), e.clone());
                }
            }
        }
        improved
    }

    fn build_path(
        &self,
        best_cycle_vertex: &G::Vertex,
        best_cycle_length: usize,
        best_cycle_weight: f64,
    ) -> GraphWalk<G::Vertex, G::Edge> {
        let mut path_edges = Vec::with_capacity(best_cycle_length);
        let mut path_vertices = Vec::with_capacity(best_cycle_length + 1);
        let mut v = best_cycle_vertex.clone();
        path_vertices.push(v.clone());
        loop {
            let e = self.policy_graph[&v].clone();
            v = self
                .graph
                .opposite_vertex(&e, &v)
                .expect("edge of the graph")
                .clone();
            path_edges.push(e);
            path_vertices.push(v.clone());
            if v == *best_cycle_vertex {
                break;
            }
        }
        GraphWalk::new(path_vertices, path_edges, best_cycle_weight)
    }
}

impl<'g, G: DirectedGraph, S: StrongConnectivityAlgorithm<G = G>> MinimumCycleMeanAlgorithm<G>
    for HowardMinimumMeanCycle<'g, G, S>
{
    fn get_cycle_mean(&mut self) -> f64 {
        match self.get_cycle() {
            None => f64::INFINITY,
            Some(cycle) => cycle.weight() / cycle.length() as f64,
        }
    }

    fn get_cycle(&mut self) -> Option<GraphWalk<G::Vertex, G::Edge>> {
        let mut is_best_cycle_found = false;
        let mut best_cycle_weight = 0.0;
        let mut best_cycle_length = 1usize;
        let mut best_cycle_vertex: Option<G::Vertex> = None;
        let mut number_of_iterations = 0usize;
        let graph = self.graph;
        let sets: Vec<_> = self
            .strong_connectivity_algorithm
            .strongly_connected_sets()
            .to_vec();
        for set in sets {
            let component = crate::view::AsSubgraph::induced(graph, set);

            let mut skip = component.vertex_count() == 0;
            skip |= component.vertex_count() == 1
                && component
                    .in_edges_of(component.vertices().next().expect("one vertex")).unwrap()
                    .next()
                    .is_none();
            if skip {
                continue;
            }
            self.construct_policy_graph(&component);
            let mut improved = true;
            while number_of_iterations < self.maximum_iterations && improved {
                self.construct_cycle(&component);
                improved = self.compute_vertex_distance(&component);
                number_of_iterations += 1;
            }
            if self.is_current_cycle_found
                && (!is_best_cycle_found
                    || self.current_cycle_weight * (best_cycle_length as f64)
                        < best_cycle_weight * (self.current_cycle_length as f64))
            {
                is_best_cycle_found = true;
                best_cycle_weight = self.current_cycle_weight;
                best_cycle_length = self.current_cycle_length;
                best_cycle_vertex = self.current_cycle_vertex.clone();
            }
            if number_of_iterations == self.maximum_iterations {
                break;
            }
        }
        if is_best_cycle_found {
            return Some(self.build_path(
                best_cycle_vertex.as_ref().expect("best cycle vertex"),
                best_cycle_length,
                best_cycle_weight,
            ));
        }
        None
    }
}
