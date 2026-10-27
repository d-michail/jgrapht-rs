use crate::graph::{GraphExt, UndirectedGraph};
use crate::traverse::{LexBreadthFirstIterator, MaximumCardinalityIterator};
use crate::walk::GraphWalk;
use std::collections::{HashMap, HashSet};

/// `ChordalityInspector.IterationOrder`: the ordering that is checked for being a perfect
/// elimination order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterationOrder {
    /// maximum cardinality search
    Mcs,
    /// lexicographic breadth-first search
    LexBfs,
}

/// `ChordalityInspector`: tests whether a graph is chordal (every cycle of length at least four
/// has a chord) by checking the order of a maximum cardinality or lexicographic BFS search.
/// Only undirected graphs are supported (the vertex-order iterators reject directed graphs, as in
/// Java).
pub struct ChordalityInspector<'g, G: UndirectedGraph> {
    graph: &'g G,
    iteration_order: IterationOrder,
    chordal: bool,
    order: Option<Vec<G::Vertex>>,
    hole: Option<GraphWalk<G::Vertex, G::Edge>>,
}

impl<'g, G: UndirectedGraph> ChordalityInspector<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self::with_order(graph, IterationOrder::Mcs)
    }

    pub fn with_order(graph: &'g G, iteration_order: IterationOrder) -> Self {
        ChordalityInspector {
            graph,
            iteration_order,
            chordal: false,
            order: None,
            hole: None,
        }
    }

    pub fn iteration_order(&self) -> IterationOrder {
        self.iteration_order
    }

    /// Whether the edge `{a, b}` exists in either direction.
    fn has_edge(&self, a: &G::Vertex, b: &G::Vertex) -> bool {
        self.graph.contains_edge_between(a, b) || self.graph.contains_edge_between(b, a)
    }

    pub fn is_chordal(&mut self) -> bool {
        if self.order.is_none() {
            let n = self.graph.vertex_count();
            let order: Vec<G::Vertex> = match self.iteration_order {
                IterationOrder::Mcs => MaximumCardinalityIterator::new(self.graph)
                    .take(n)
                    .cloned()
                    .collect(),
                IterationOrder::LexBfs => LexBreadthFirstIterator::new(self.graph)
                    .take(n)
                    .cloned()
                    .collect(),
            };
            self.chordal = self.is_perfect_elimination_order_impl(&order, true);
            self.order = Some(order);
        }
        self.chordal
    }

    /// A perfect elimination order, or `None` if the graph is not chordal.
    pub fn get_perfect_elimination_order(&mut self) -> Option<&[G::Vertex]> {
        self.is_chordal();
        if self.chordal {
            self.order.as_deref()
        } else {
            None
        }
    }

    /// An induced cycle of length at least four, if the graph is not chordal.
    pub fn get_hole(&mut self) -> Option<&GraphWalk<G::Vertex, G::Edge>> {
        self.is_chordal();
        self.hole.as_ref()
    }

    /// Whether `vertex_order` is a perfect elimination order of the graph.
    pub fn is_perfect_elimination_order(&mut self, vertex_order: &[G::Vertex]) -> bool {
        self.is_perfect_elimination_order_impl(vertex_order, false)
    }

    fn is_perfect_elimination_order_impl(
        &mut self,
        vertex_order: &[G::Vertex],
        compute_hole: bool,
    ) -> bool {
        let graph = self.graph;
        if graph.vertex_count() == vertex_order.len()
            && vertex_order.iter().all(|v| graph.contains_vertex(v))
        {
            let vertex_in_order: HashMap<&G::Vertex, usize> = vertex_order
                .iter()
                .enumerate()
                .map(|(i, v)| (v, i))
                .collect();
            for vertex in vertex_order {
                let predecessors = self.predecessors(&vertex_in_order, vertex);
                if !predecessors.is_empty() {
                    // the predecessor latest in the order
                    let max_predecessor = predecessors
                        .iter()
                        .max_by_key(|p| vertex_in_order[**p])
                        .expect("non-empty");
                    for predecessor in &predecessors {
                        if predecessor != max_predecessor
                            && !self.has_edge(predecessor, max_predecessor)
                        {
                            if compute_hole {
                                let (a, b, c) = (
                                    (*predecessor).clone(),
                                    vertex.clone(),
                                    (*max_predecessor).clone(),
                                );
                                self.find_hole(&a, &b, &c);
                            }
                            return false;
                        }
                    }
                }
            }
            true
        } else {
            false
        }
    }

    fn predecessors<'a>(
        &self,
        vertex_in_order: &HashMap<&'a G::Vertex, usize>,
        vertex: &'a G::Vertex,
    ) -> Vec<&'a G::Vertex>
    where
        'g: 'a,
    {
        let graph = self.graph;
        let vertex_position = vertex_in_order[vertex];
        let mut predecessors: Vec<&G::Vertex> = Vec::new();
        for edge in graph.edges_of(vertex).unwrap() {
            let opposite = graph
                .opposite_vertex(edge, vertex)
                .expect("edge of the graph");
            let dest_position = vertex_in_order[opposite];
            if dest_position < vertex_position && !predecessors.contains(&opposite) {
                predecessors.push(opposite);
            }
        }
        predecessors
    }

    fn find_hole(&mut self, a: &G::Vertex, b: &G::Vertex, c: &G::Vertex) {
        let mut cycle = vec![a.clone(), b.clone(), c.clone()];
        let mut visited: HashMap<G::Vertex, bool> =
            self.graph.vertices().map(|v| (v.clone(), false)).collect();
        visited.insert(a.clone(), true);
        visited.insert(b.clone(), true);
        self.dfs_visit(&mut cycle, &mut visited, a, b, c);
        let cycle = self.minimize_cycle(&cycle);
        self.hole = GraphWalk::from_vertex_list(self.graph, cycle, 0.0).ok();
    }

    fn dfs_visit(
        &self,
        cycle: &mut Vec<G::Vertex>,
        visited: &mut HashMap<G::Vertex, bool>,
        finish: &G::Vertex,
        middle: &G::Vertex,
        current: &G::Vertex,
    ) {
        visited.insert(current.clone(), true);
        let graph = self.graph;
        for edge in graph.edges_of(current).unwrap() {
            let opposite = graph
                .opposite_vertex(edge, current)
                .expect("edge of the graph");
            if (!visited[opposite] && !self.has_edge(opposite, middle)) || opposite == finish {
                cycle.push(opposite.clone());
                if opposite == finish {
                    return;
                }
                self.dfs_visit(cycle, visited, finish, middle, opposite);
                if cycle.last().expect("non-empty") == finish {
                    return;
                }
                cycle.pop();
            }
        }
    }

    fn minimize_cycle(&self, cycle: &[G::Vertex]) -> Vec<G::Vertex> {
        let graph = self.graph;
        let mut cycle_vertices: HashSet<&G::Vertex> = cycle.iter().collect();
        cycle_vertices.remove(&cycle[1]);
        let mut minimized = vec![cycle[0].clone(), cycle[1].clone()];
        let mut i = 2;
        while i + 1 < cycle.len() {
            let vertex = &cycle[i];
            minimized.push(vertex.clone());
            cycle_vertices.remove(vertex);
            let mut forward: Vec<&G::Vertex> = Vec::new();
            for edge in graph.edges_of(vertex).unwrap() {
                let opposite = graph
                    .opposite_vertex(edge, vertex)
                    .expect("edge of the graph");
                if cycle_vertices.contains(opposite) && !forward.contains(&opposite) {
                    forward.push(opposite);
                }
            }
            for forward_vertex in forward {
                if cycle_vertices.contains(forward_vertex) {
                    loop {
                        cycle_vertices.remove(&cycle[i]);
                        i += 1;
                        if !(i < cycle.len() && cycle[i] != *forward_vertex) {
                            break;
                        }
                    }
                }
            }
        }
        minimized.push(cycle[cycle.len() - 1].clone());
        minimized
    }
}
