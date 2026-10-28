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

use crate::error::{GraphError, Result};
use crate::graph::IncidenceGraph;
use crate::graph::{Graph, GraphExt};
use crate::walk::GraphWalk;
use std::collections::{HashMap, HashSet};

/// Computes an Eulerian cycle of an Eulerian graph. An
/// [Eulerian graph](http://mathworld.wolfram.com/EulerianGraph.html) is a graph
/// containing an [Eulerian cycle](http://mathworld.wolfram.com/EulerianCycle.html).
///
/// # Rust notes
///
/// `EulerianCycleAlgorithm`.
pub trait EulerianCycleAlgorithm<G: Graph> {
    /// Returns the eulerian cycle.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidGraph`](crate::GraphError::InvalidGraph) with `Eulerian` if the graph has no Eulerian cycle, and `NotMixed` for a mixed graph.
    fn get_eulerian_cycle(&mut self, graph: &G) -> Result<GraphWalk<G::Vertex, G::Edge>>;
}

const NIL: usize = usize::MAX;

struct VertexNode {
    prev: usize,
    next: usize,
    insert_location: usize,
    adj_edges_head: usize,
    /// index into the graph's vertex list
    v: usize,
}

struct EdgeNode {
    /// index into the graph's edge list
    e: usize,
    next: usize,
    prev: usize,
    reverse: usize,
    source_node: usize,
    target_node: usize,
}

/// An implementation of Hierholzer's algorithm for finding an Eulerian cycle in Eulerian graphs. The
/// algorithm works with directed and undirected graphs which may contain loops and/or multiple
/// (parallel) edges. The running time is linear, i.e. $O(|E|)$ where $|E|$ is the cardinality of the
/// edge set of the graph.
///
/// See the [Wikipedia article](https://en.wikipedia.org/wiki/Eulerian_path) for details
/// and references about Eulerian cycles and a short description of Hierholzer's algorithm for the
/// construction of an Eulerian cycle. The original presentation of the algorithm dates back to 1873
/// and the following paper: Carl Hierholzer: Über die Möglichkeit, einen Linienzug ohne
/// Wiederholung und ohne Unterbrechung zu umfahren. Mathematische Annalen 6(1), 30–32, 1873.
///
/// # Rust notes
///
/// `HierholzerEulerianCycle`: Hierholzer's linear-time construction of an Eulerian cycle, with the
/// partial cycles spliced into the final one through doubly linked lists.
#[derive(Default)]
pub struct HierholzerEulerianCycle;

/// The state of one run: arenas of vertex and edge nodes.
struct Run {
    is_directed: bool,
    vnodes: Vec<VertexNode>,
    enodes: Vec<EdgeNode>,
    vertices_head: usize,
    eulerian_head: usize,
    start_vertex: usize,
}

impl Run {
    fn add_edge(&mut self, s_node: usize, t_node: usize, e: usize) {
        let s_head_old = self.vnodes[s_node].adj_edges_head;
        let s_head = self.new_edge_node(s_node, t_node, NIL, e, NIL, s_head_old);
        if s_head_old != NIL {
            self.enodes[s_head_old].prev = s_head;
        }
        self.vnodes[s_node].adj_edges_head = s_head;
        if !self.is_directed && s_node != t_node {
            let t_head_old = self.vnodes[t_node].adj_edges_head;
            let t_head = self.new_edge_node(t_node, s_node, NIL, e, s_head, t_head_old);
            if t_head_old != NIL {
                self.enodes[t_head_old].prev = t_head;
            }
            self.enodes[s_head].reverse = t_head;
            self.vnodes[t_node].adj_edges_head = t_head;
        }
    }

    fn new_edge_node(
        &mut self,
        source_node: usize,
        target_node: usize,
        prev: usize,
        e: usize,
        reverse: usize,
        next: usize,
    ) -> usize {
        self.enodes.push(EdgeNode {
            e,
            next,
            prev,
            reverse,
            source_node,
            target_node,
        });
        self.enodes.len() - 1
    }

    fn unlink_vertex(&mut self, v_node: usize) {
        if self.vertices_head == NIL {
            return;
        }
        let (prev, next) = (self.vnodes[v_node].prev, self.vnodes[v_node].next);
        if self.vertices_head != v_node && prev == NIL && next == NIL {
            return;
        } else if prev != NIL {
            self.vnodes[prev].next = next;
            if next != NIL {
                self.vnodes[next].prev = prev;
            }
        } else {
            self.vertices_head = next;
            if self.vertices_head != NIL {
                self.vnodes[self.vertices_head].prev = NIL;
            }
        }
        self.vnodes[v_node].next = NIL;
        self.vnodes[v_node].prev = NIL;
    }

    fn move_to_front(&mut self, v_node: usize) {
        let (prev, next) = (self.vnodes[v_node].prev, self.vnodes[v_node].next);
        if prev != NIL {
            self.vnodes[prev].next = next;
            if next != NIL {
                self.vnodes[next].prev = prev;
            }
            self.vnodes[self.vertices_head].prev = v_node;
            self.vnodes[v_node].next = self.vertices_head;
            self.vnodes[v_node].prev = NIL;
            self.vertices_head = v_node;
        }
    }

    /// Removes `node` from its source's adjacency list.
    fn unlink_from_adjacency(&mut self, node: usize) {
        let v_node = self.enodes[node].source_node;
        let (prev, next) = (self.enodes[node].prev, self.enodes[node].next);
        if prev != NIL {
            self.enodes[prev].next = next;
            if next != NIL {
                self.enodes[next].prev = prev;
            }
        } else {
            if next != NIL {
                self.enodes[next].prev = NIL;
            }
            self.vnodes[v_node].adj_edges_head = next;
        }
    }

    fn unlink_edge(&mut self, e_node: usize) {
        self.unlink_from_adjacency(e_node);
        let rev = self.enodes[e_node].reverse;
        if !self.is_directed && rev != NIL {
            self.unlink_from_adjacency(rev);
        }
        let node = &mut self.enodes[e_node];
        node.next = NIL;
        node.prev = NIL;
        node.reverse = NIL;
    }

    fn opposite(&self, v: usize, e: usize) -> usize {
        if v == self.enodes[e].source_node {
            self.enodes[e].target_node
        } else {
            self.enodes[e].source_node
        }
    }

    /// Follows unused edges from the head vertex until it is back there; returns the first and
    /// last edge node of the partial cycle.
    fn compute_partial_cycle(&mut self) -> (usize, usize) {
        if self.start_vertex == NIL {
            self.start_vertex = self.vnodes[self.vertices_head].v;
        }
        let (mut partial_head, mut partial_tail) = (NIL, NIL);
        let mut v = self.vertices_head;
        loop {
            let e = self.vnodes[v].adj_edges_head;
            v = self.opposite(v, e);
            self.unlink_edge(e);
            if partial_tail == NIL {
                partial_tail = e;
                partial_head = partial_tail;
            } else {
                self.enodes[partial_tail].next = e;
                partial_tail = self.enodes[partial_tail].next;
            }
            if v == self.vertices_head {
                break;
            }
        }
        (partial_head, partial_tail)
    }

    fn update_graph_and_insert_locations(&mut self, partial_head: usize, source_vertex: usize) {
        let mut e = partial_head;
        let mut v = self.opposite(source_vertex, e);
        loop {
            if self.vnodes[v].adj_edges_head != NIL {
                self.vnodes[v].insert_location = e;
                self.move_to_front(v);
            } else {
                self.unlink_vertex(v);
            }
            e = self.enodes[e].next;
            if e == NIL {
                break;
            }
            v = self.opposite(v, e);
        }
    }
}

impl HierholzerEulerianCycle {
    /// Constructs a new Hierholzer Eulerian cycle algorithm.
    pub fn new() -> Self {
        HierholzerEulerianCycle
    }

    /// Whether the graph has an Eulerian cycle: balanced degrees and a single component with edges.
    /// The empty graph (no vertices) is not Eulerian.
    pub fn is_eulerian<G: IncidenceGraph>(&self, graph: &G) -> bool {
        if graph.graph_type().mixed {
            return false;
        }
        if graph.vertex_count() == 0 {
            return false;
        } else if graph.edge_count() == 0 {
            return true;
        }
        let directed = graph.graph_type().directed;
        if !directed {
            for vertex in graph.vertices() {
                if graph.degree_of(vertex) % 2 == 1 {
                    return false;
                }
            }
        } else {
            for vertex in graph.vertices() {
                if graph.in_degree_of(vertex) != graph.out_degree_of(vertex) {
                    return false;
                }
            }
        }
        // In a balanced digraph the weakly and strongly connected components coincide, so the
        // undirected component structure decides both cases.
        let mut found_component_with_edges = false;
        let mut seen = HashSet::new();
        for root in graph.vertices() {
            if !seen.insert(root.clone()) {
                continue;
            }
            let mut component_has_edges = false;
            let mut stack = vec![root.clone()];
            while let Some(vertex) = stack.pop() {
                component_has_edges |= graph.degree_of(&vertex) > 0;
                for neighbor in graph.neighbors_of(&vertex) {
                    if seen.insert(neighbor.clone()) {
                        stack.push(neighbor.clone());
                    }
                }
            }
            if component_has_edges {
                if found_component_with_edges {
                    return false;
                }
                found_component_with_edges = true;
            }
        }
        true
    }
}

impl<G: IncidenceGraph> EulerianCycleAlgorithm<G> for HierholzerEulerianCycle {
    /// Compute an Eulerian cycle of a graph.
    fn get_eulerian_cycle(&mut self, g: &G) -> Result<GraphWalk<G::Vertex, G::Edge>> {
        if !self.is_eulerian(g) {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::Eulerian));
        } else if g.vertex_count() == 0 {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::NonEmpty));
        } else if g.edge_count() == 0 {
            return Ok(GraphWalk::empty());
        }
        let vs: Vec<&G::Vertex> = g.vertices().collect();
        let es: Vec<&G::Edge> = g.edges().collect();
        let mut run = Run {
            is_directed: g.graph_type().directed,
            vnodes: Vec::new(),
            enodes: Vec::new(),
            vertices_head: NIL,
            eulerian_head: NIL,
            start_vertex: NIL,
        };
        // initialize
        let mut vertices: HashMap<&G::Vertex, usize> = HashMap::new();
        let mut vertex_index: HashMap<&G::Vertex, usize> = HashMap::new();
        for (i, v) in vs.iter().enumerate() {
            vertex_index.insert(*v, i);
            if g.out_degree_of(v) > 0 {
                let n = run.vnodes.len();
                run.vnodes.push(VertexNode {
                    prev: NIL,
                    next: run.vertices_head,
                    insert_location: NIL,
                    adj_edges_head: NIL,
                    v: i,
                });
                if run.vertices_head != NIL {
                    run.vnodes[run.vertices_head].prev = n;
                }
                run.vertices_head = n;
                vertices.insert(*v, n);
            }
        }
        for (i, e) in es.iter().enumerate() {
            let s_node = vertices[g.edge_source(e).expect("edge of the graph")];
            let t_node = vertices[g.edge_target(e).expect("edge of the graph")];
            run.add_edge(s_node, t_node, i);
        }
        while run.vertices_head != NIL {
            let where_to_insert = run.vnodes[run.vertices_head].insert_location;
            let (partial_head, partial_tail) = run.compute_partial_cycle();
            let head = run.vertices_head;
            run.update_graph_and_insert_locations(partial_head, head);
            if where_to_insert == NIL {
                run.eulerian_head = partial_head;
            } else {
                run.enodes[partial_tail].next = run.enodes[where_to_insert].next;
                run.enodes[where_to_insert].next = partial_head;
            }
        }
        // buildWalk
        let start = vs[run.start_vertex].clone();
        let mut total_weight = 0.0;
        let mut edges = Vec::with_capacity(es.len());
        let mut vertex_list = vec![start.clone()];
        let mut current = start;
        let mut it = run.eulerian_head;
        while it != NIL {
            let e = es[run.enodes[it].e];
            total_weight += g.weight_of(e);
            current = g
                .opposite_vertex(e, &current)
                .expect("edge of the graph")
                .clone();
            vertex_list.push(current.clone());
            edges.push(e.clone());
            it = run.enodes[it].next;
        }
        let _ = vertex_index;
        Ok(GraphWalk::new_unchecked(vertex_list, edges, total_weight))
    }
}
