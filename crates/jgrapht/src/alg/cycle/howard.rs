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

use crate::alg::connectivity::GabowStrongConnectivityInspector;
use crate::alg::interfaces::StrongConnectivityAlgorithm;
use crate::error::Result;
use crate::graph::{DirectedGraph, Graph, GraphExt};
use crate::util::ToleranceDoubleComparator;
use crate::walk::GraphWalk;
use std::collections::{HashMap, VecDeque};

/// The algorithm for finding minimum cycle mean in a graph.
///
/// Consider a cycle $C$ in a graph. The mean of cycle $C$ is defined as $\lambda
/// (C)=\frac{w(C)}{|C|}$, where $w(C)$ is the total weight of $C$ and $|C|$ is the length of $C$.
///
/// # Rust notes
///
/// `MinimumCycleMeanAlgorithm`: the cycle with the smallest mean edge weight.
pub trait MinimumCycleMeanAlgorithm<G: Graph> {
    /// The smallest cycle mean, `+infinity` if the graph has no cycle.
    fn get_cycle_mean(&mut self) -> f64;
    /// A cycle attaining the smallest mean, `None` if the graph is acyclic.
    fn get_cycle(&mut self) -> Option<GraphWalk<G::Vertex, G::Edge>>;
}

/// Implementation of Howard's algorithm for finding minimum cycle mean in a graph.
///
/// The algorithm is described in the article: Ali Dasdan, Sandy S. Irani, and Rajesh K. Gupta. 1999.
/// Efficient algorithms for optimum cycle mean and optimum cost to time ratio problems. In
/// Proceedings of the 36th annual ACM/IEEE Design Automation Conference (DAC ’99). Association for
/// Computing Machinery, New York, NY, USA, 37–42. DOI:<https://doi.org/10.1145/309847.309862>
///
/// Firstly, the graph is divided into strongly connected components. The minimum cycle mean is then
/// computed as the globally minimum cycle mean over all components. In the process the necessary
/// information is recorded to be able to reconstruct the cycle with minimum mean.
///
/// The computations are divided into iterations. In each iteration the algorithm tries to update
/// current minimum cycle mean value. There is a possibility to limit the total number of iteration
/// via a constructor parameter.
///
/// # Rust notes
///
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

    /// Computes policy graph for `component` and stores result in `policyGraph` and
    /// `vertexDistance`. For every vertex in the policy graph an edge with the minimum weight
    /// is retained in the policy graph.
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

    /// Finds cycle in the `policyGraph` and computes computes its mean. The found cycle is
    /// identified by a vertex `currentCycleVertex`. The cycle returned by this method does not
    /// necessarily has the smalles mean over all cycles in the policy graph.
    ///
    /// To find cycles this methods assigns a level to each vertex. Initially every vertex has a
    /// level equal to $-1$ which means that the vertex has not been visited. During the computations
    /// this method starts DFS from every not visited vertex and assigns a unique positive level $l$
    /// to every traversed vertex. If DFS comes across a vertex with level $l$ this indicates that a
    /// cycle has been detected.
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

    /// This method runs the reverted BFS starting from `currentCycleVertex` to update data in
    /// `policyGraph` and `vertexDistance`. This step is needed to identify if current
    /// value of minimum mean is optimal for the `graph`. This method also uses
    /// `comparator` to find out if update value of minium mean is sufficiently smaller than
    /// the previous one.
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

    /// Constructs cycle with minimum mean using information in `policyGraph`.
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
    /// Computes minimum mean among all cycle. Returns `Double#POSITIVE_INFINITY` if no cycle
    /// has been found.
    fn get_cycle_mean(&mut self) -> f64 {
        match self.get_cycle() {
            None => f64::INFINITY,
            Some(cycle) => cycle.weight() / cycle.length() as f64,
        }
    }

    /// Computes cycle with minimum mean. Returns $null$ if no cycle has been found.
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
