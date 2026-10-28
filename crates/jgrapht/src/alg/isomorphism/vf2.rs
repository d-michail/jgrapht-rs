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

use super::IsomorphismInspector;
use super::mapping::{IsomorphicGraphMapping, NULL_NODE};
use super::ordering::GraphOrdering;
use crate::error::{GraphError, Result};
use crate::graph::Graph;
use crate::graph::IncidenceGraph;
use std::cmp::Ordering;

/// Compares vertices of the two graphs; vertices may be matched only when it returns `Equal`.
pub type VertexComparator<'g, G> =
    Box<dyn Fn(&<G as Graph>::Vertex, &<G as Graph>::Vertex) -> Ordering + 'g>;
/// Compares edges of the two graphs; edges may be matched only when it returns `Equal`.
pub type EdgeComparator<'g, G> =
    Box<dyn Fn(&<G as Graph>::Edge, &<G as Graph>::Edge) -> Ordering + 'g>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Graph,
    Subgraph,
}

/// The scalar part of a VF2 state. States produced by [`Cursor::clone`] share the arrays in
/// [`Arrays`], exactly as the Java copy constructor shares them.
#[derive(Clone)]
struct Cursor {
    core_len: usize,
    t1_both: usize,
    t2_both: usize,
    t1_in: usize,
    t2_in: usize,
    t1_out: usize,
    t2_out: usize,
    added_vertex1: usize,
    add_vertex1: usize,
    add_vertex2: usize,
}

impl Cursor {
    fn new() -> Self {
        Cursor {
            core_len: 0,
            t1_both: 0,
            t2_both: 0,
            t1_in: 0,
            t2_in: 0,
            t1_out: 0,
            t2_out: 0,
            added_vertex1: NULL_NODE,
            add_vertex1: NULL_NODE,
            add_vertex2: NULL_NODE,
        }
    }

    fn reset_add_vertexes(&mut self) {
        self.add_vertex1 = NULL_NODE;
        self.add_vertex2 = NULL_NODE;
    }

    fn is_goal(&self, n2: usize) -> bool {
        self.core_len == n2
    }
}

struct Arrays {
    core1: Vec<usize>,
    core2: Vec<usize>,
    in1: Vec<usize>,
    in2: Vec<usize>,
    out1: Vec<usize>,
    out2: Vec<usize>,
}

/// The VF2 search shared by the graph and the subgraph variant.
struct Vf2<'a, 'g, G: IncidenceGraph> {
    g1: &'a GraphOrdering<'g, G>,
    g2: &'a GraphOrdering<'g, G>,
    vertex_comparator: Option<&'a VertexComparator<'g, G>>,
    edge_comparator: Option<&'a EdgeComparator<'g, G>>,
    kind: Kind,
    n1: usize,
    n2: usize,
    a: Arrays,
    stack: Vec<Cursor>,
}

impl<'a, 'g, G: IncidenceGraph> Vf2<'a, 'g, G> {
    fn next_pair(&self, s: &mut Cursor) -> bool {
        let (n1, n2, a) = (self.n1, self.n2, &self.a);
        if s.add_vertex2 == NULL_NODE {
            s.add_vertex2 = 0;
        }
        if s.add_vertex1 == NULL_NODE {
            s.add_vertex1 = 0;
        } else {
            s.add_vertex1 += 1;
        }
        // which terminal set the candidates must come from: 0 both, 1 out, 2 in, 3 none
        let mode = if s.t1_both > s.core_len && s.t2_both > s.core_len {
            0
        } else if s.t1_out > s.core_len && s.t2_out > s.core_len {
            1
        } else if s.t1_in > s.core_len && s.t2_in > s.core_len {
            2
        } else {
            3
        };
        let skip = |core: &[usize], inn: &[usize], out: &[usize], i: usize| {
            core[i] != NULL_NODE
                || match mode {
                    0 => out[i] == 0 || inn[i] == 0,
                    1 => out[i] == 0,
                    2 => inn[i] == 0,
                    _ => false,
                }
        };
        while s.add_vertex2 < n2 && skip(&a.core2, &a.in2, &a.out2, s.add_vertex2) {
            s.add_vertex2 += 1;
            s.add_vertex1 = 0;
        }
        while s.add_vertex1 < n1 && skip(&a.core1, &a.in1, &a.out1, s.add_vertex1) {
            s.add_vertex1 += 1;
        }
        if s.add_vertex1 < n1 && s.add_vertex2 < n2 {
            return true;
        }
        s.reset_add_vertexes();
        false
    }

    fn add_pair(&mut self, s: &mut Cursor) {
        let (v1, v2) = (s.add_vertex1, s.add_vertex2);
        s.core_len += 1;
        let len = s.core_len;
        s.added_vertex1 = v1;
        let a = &mut self.a;

        if a.in1[v1] == 0 {
            a.in1[v1] = len;
            s.t1_in += 1;
            if a.out1[v1] > 0 {
                s.t1_both += 1;
            }
        }
        if a.out1[v1] == 0 {
            a.out1[v1] = len;
            s.t1_out += 1;
            if a.in1[v1] > 0 {
                s.t1_both += 1;
            }
        }
        if a.in2[v2] == 0 {
            a.in2[v2] = len;
            s.t2_in += 1;
            if a.out2[v2] > 0 {
                s.t2_both += 1;
            }
        }
        if a.out2[v2] == 0 {
            a.out2[v2] = len;
            s.t2_out += 1;
            if a.in2[v2] > 0 {
                s.t2_both += 1;
            }
        }

        a.core1[v1] = v2;
        a.core2[v2] = v1;

        for &o in self.g1.in_edges(v1).iter() {
            if a.in1[o] == 0 {
                a.in1[o] = len;
                s.t1_in += 1;
                if a.out1[o] > 0 {
                    s.t1_both += 1;
                }
            }
        }
        for &o in self.g1.out_edges(v1).iter() {
            if a.out1[o] == 0 {
                a.out1[o] = len;
                s.t1_out += 1;
                if a.in1[o] > 0 {
                    s.t1_both += 1;
                }
            }
        }
        for &o in self.g2.in_edges(v2).iter() {
            if a.in2[o] == 0 {
                a.in2[o] = len;
                s.t2_in += 1;
                if a.out2[o] > 0 {
                    s.t2_both += 1;
                }
            }
        }
        for &o in self.g2.out_edges(v2).iter() {
            if a.out2[o] == 0 {
                a.out2[o] = len;
                s.t2_out += 1;
                if a.in2[o] > 0 {
                    s.t2_both += 1;
                }
            }
        }
    }

    fn backtrack(&mut self, s: &mut Cursor) {
        let v1 = s.added_vertex1;
        let v2 = self.a.core1[v1];
        let len = s.core_len;
        let a = &mut self.a;

        if a.in1[v1] == len {
            a.in1[v1] = 0;
        }
        for &o in self.g1.in_edges(v1).iter() {
            if a.in1[o] == len {
                a.in1[o] = 0;
            }
        }
        if a.out1[v1] == len {
            a.out1[v1] = 0;
        }
        for &o in self.g1.out_edges(v1).iter() {
            if a.out1[o] == len {
                a.out1[o] = 0;
            }
        }
        if a.in2[v2] == len {
            a.in2[v2] = 0;
        }
        for &o in self.g2.in_edges(v2).iter() {
            if a.in2[o] == len {
                a.in2[o] = 0;
            }
        }
        if a.out2[v2] == len {
            a.out2[v2] = 0;
        }
        for &o in self.g2.out_edges(v2).iter() {
            if a.out2[o] == len {
                a.out2[o] = 0;
            }
        }
        a.core1[v1] = NULL_NODE;
        a.core2[v2] = NULL_NODE;
        s.core_len -= 1;
        s.added_vertex1 = NULL_NODE;
    }

    fn compatible_vertices(&self, v1: usize, v2: usize) -> bool {
        self.vertex_comparator
            .is_none_or(|c| c(self.g1.vertex(v1), self.g2.vertex(v2)) == Ordering::Equal)
    }

    fn compatible_edges(&self, v1: usize, v2: usize, u1: usize, u2: usize) -> bool {
        self.edge_comparator
            .is_none_or(|c| match (self.g1.edge(v1, v2), self.g2.edge(u1, u2)) {
                (Some(e1), Some(e2)) => c(&e1, &e2) == Ordering::Equal,
                _ => false,
            })
    }

    /// Counts of the not-yet-matched neighbours of `v` in terminal-in, terminal-out and
    /// fresh position; `None` when a matched neighbour breaks the edge structure.
    fn is_feasible(&self, s: &Cursor) -> bool {
        let (v1, v2) = (s.add_vertex1, s.add_vertex2);
        if !self.compatible_vertices(v1, v2) {
            return false;
        }
        let a = &self.a;
        let (mut term_out_pred1, mut term_out_pred2) = (0, 0);
        let (mut term_in_pred1, mut term_in_pred2) = (0, 0);
        let (mut new_pred1, mut new_pred2) = (0, 0);
        let (mut term_out_succ1, mut term_out_succ2) = (0, 0);
        let (mut term_in_succ1, mut term_in_succ2) = (0, 0);
        let (mut new_succ1, mut new_succ2) = (0, 0);

        for &o1 in self.g1.out_edges(v1).iter() {
            if a.core1[o1] != NULL_NODE {
                let o2 = a.core1[o1];
                if !self.g2.has_edge(v2, o2) || !self.compatible_edges(v1, o1, v2, o2) {
                    return false;
                }
            } else {
                let (i, o) = (a.in1[o1], a.out1[o1]);
                if i == 0 && o == 0 {
                    new_succ1 += 1;
                    continue;
                }
                if i > 0 {
                    term_in_succ1 += 1;
                }
                if o > 0 {
                    term_out_succ1 += 1;
                }
            }
        }
        for &o2 in self.g2.out_edges(v2).iter() {
            if a.core2[o2] != NULL_NODE {
                if !self.g1.has_edge(v1, a.core2[o2]) {
                    return false;
                }
            } else {
                let (i, o) = (a.in2[o2], a.out2[o2]);
                if i == 0 && o == 0 {
                    new_succ2 += 1;
                    continue;
                }
                if i > 0 {
                    term_in_succ2 += 1;
                }
                if o > 0 {
                    term_out_succ2 += 1;
                }
            }
        }
        let succ_ok = match self.kind {
            Kind::Graph => {
                term_in_succ1 == term_in_succ2
                    && term_out_succ1 == term_out_succ2
                    && new_succ1 == new_succ2
            }
            Kind::Subgraph => {
                term_in_succ1 >= term_in_succ2
                    && term_out_succ1 >= term_out_succ2
                    && new_succ1 >= new_succ2
            }
        };
        if !succ_ok {
            return false;
        }

        for &o1 in self.g1.in_edges(v1).iter() {
            if a.core1[o1] != NULL_NODE {
                let o2 = a.core1[o1];
                if !self.g2.has_edge(o2, v2) || !self.compatible_edges(o1, v1, o2, v2) {
                    return false;
                }
            } else {
                let (i, o) = (a.in1[o1], a.out1[o1]);
                if i == 0 && o == 0 {
                    new_pred1 += 1;
                    continue;
                }
                if i > 0 {
                    term_in_pred1 += 1;
                }
                if o > 0 {
                    term_out_pred1 += 1;
                }
            }
        }
        for &o2 in self.g2.in_edges(v2).iter() {
            if a.core2[o2] != NULL_NODE {
                if !self.g1.has_edge(a.core2[o2], v1) {
                    return false;
                }
            } else {
                let (i, o) = (a.in2[o2], a.out2[o2]);
                if i == 0 && o == 0 {
                    new_pred2 += 1;
                    continue;
                }
                if i > 0 {
                    term_in_pred2 += 1;
                }
                if o > 0 {
                    term_out_pred2 += 1;
                }
            }
        }
        match self.kind {
            Kind::Graph => {
                term_in_pred1 == term_in_pred2
                    && term_out_pred1 == term_out_pred2
                    && new_pred1 == new_pred2
            }
            Kind::Subgraph => {
                term_in_pred1 >= term_in_pred2
                    && term_out_pred1 >= term_out_pred2
                    && new_pred1 >= new_pred2
            }
        }
    }

    fn current_mapping(&self) -> IsomorphicGraphMapping<'g, G> {
        IsomorphicGraphMapping::from_cores(self.g1, self.g2, &self.a.core1, &self.a.core2)
    }
}

/// `VF2GraphMappingIterator` / `VF2SubgraphMappingIterator`: lazily enumerates the mappings of
/// the second graph's vertices into the first graph's by depth-first search over VF2 states.
pub struct VF2MappingIterator<'a, 'g, G: IncidenceGraph> {
    vf2: Vf2<'a, 'g, G>,
    next_mapping: Option<IsomorphicGraphMapping<'g, G>>,
    had_one_mapping: bool,
}

impl<'a, 'g, G: IncidenceGraph> VF2MappingIterator<'a, 'g, G> {
    fn new(
        g1: &'a GraphOrdering<'g, G>,
        g2: &'a GraphOrdering<'g, G>,
        vertex_comparator: Option<&'a VertexComparator<'g, G>>,
        edge_comparator: Option<&'a EdgeComparator<'g, G>>,
        kind: Kind,
    ) -> Self {
        let (n1, n2) = (g1.vertex_count(), g2.vertex_count());
        VF2MappingIterator {
            vf2: Vf2 {
                g1,
                g2,
                vertex_comparator,
                edge_comparator,
                kind,
                n1,
                n2,
                a: Arrays {
                    core1: vec![NULL_NODE; n1],
                    core2: vec![NULL_NODE; n2],
                    in1: vec![0; n1],
                    in2: vec![0; n2],
                    out1: vec![0; n1],
                    out2: vec![0; n2],
                },
                stack: Vec::new(),
            },
            next_mapping: None,
            had_one_mapping: false,
        }
    }

    fn match_next(&mut self) -> Option<IsomorphicGraphMapping<'g, G>> {
        let vf = &mut self.vf2;
        let mut s;
        if vf.stack.is_empty() {
            let (g1, g2) = (vf.g1.graph(), vf.g2.graph());
            let too_small = match vf.kind {
                Kind::Graph => {
                    g1.vertex_count() != g2.vertex_count() || g1.edge_count() != g2.edge_count()
                }
                Kind::Subgraph => {
                    g1.vertex_count() < g2.vertex_count() || g1.edge_count() < g2.edge_count()
                }
            };
            if too_small {
                return None;
            }
            s = Cursor::new();
            if g2.vertex_count() == 0 {
                return if self.had_one_mapping {
                    None
                } else {
                    Some(vf.current_mapping())
                };
            }
        } else {
            let mut child = vf.stack.pop().unwrap();
            vf.backtrack(&mut child);
            s = vf.stack.pop().unwrap();
        }
        loop {
            while vf.next_pair(&mut s) {
                if vf.is_feasible(&s) {
                    let mut child = s.clone();
                    vf.stack.push(s);
                    vf.add_pair(&mut child);
                    s = child;
                    if s.is_goal(vf.n2) {
                        let m = vf.current_mapping();
                        vf.stack.push(s);
                        return Some(m);
                    }
                    s.reset_add_vertexes();
                }
            }
            if vf.stack.is_empty() {
                return None;
            }
            vf.backtrack(&mut s);
            s = vf.stack.pop().unwrap();
        }
    }

    fn match_and_check(&mut self) -> Option<IsomorphicGraphMapping<'g, G>> {
        let m = self.match_next();
        if m.is_some() {
            self.had_one_mapping = true;
        }
        m
    }

    /// `Iterator.hasNext`: looks ahead one mapping.
    pub fn has_next(&mut self) -> bool {
        if self.next_mapping.is_none() {
            self.next_mapping = self.match_and_check();
        }
        self.next_mapping.is_some()
    }
}

impl<'a, 'g, G: IncidenceGraph> Iterator for VF2MappingIterator<'a, 'g, G> {
    type Item = IsomorphicGraphMapping<'g, G>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_mapping.take().or_else(|| self.match_and_check())
    }
}

pub(super) fn check_types<G: IncidenceGraph>(g1: &G, g2: &G) -> Result<()> {
    let (t1, t2) = (g1.graph_type(), g2.graph_type());
    if t1.multiple_edges || t2.multiple_edges {
        return Err(GraphError::InvalidGraph(
            crate::GraphProperty::NoParallelEdges,
        ));
    }
    if t1.mixed || t2.mixed {
        return Err(GraphError::InvalidGraph(crate::GraphProperty::NotMixed));
    }
    if t1.directed != t2.directed {
        return Err(GraphError::InvalidGraph(
            crate::GraphProperty::SameDirection,
        ));
    }
    Ok(())
}

macro_rules! vf2_inspector {
    ($(#[$doc:meta])* $name:ident, $kind:expr) => {
        $(#[$doc])*
        pub struct $name<'g, G: $crate::graph::Graph> {
            vertex_comparator: Option<VertexComparator<'g, G>>,
            edge_comparator: Option<EdgeComparator<'g, G>>,
            ordering1: GraphOrdering<'g, G>,
            ordering2: GraphOrdering<'g, G>,
        }

        impl<'g, G: $crate::graph::IncidenceGraph> $name<'g, G> {
            /// Fails for multigraphs, mixed graphs and a directed/undirected pair.
            ///
            /// # Errors
            ///
            /// Errors of the graph operations it performs are propagated.
            pub fn new(graph1: &'g G, graph2: &'g G) -> Result<Self> {
                Self::with_options(graph1, graph2, None, None, true)
            }

            /// `cache_edges` trades `O(n²)` memory for faster adjacency queries.
            ///
            /// # Errors
            ///
            /// Errors of the graph operations it performs are propagated.
            pub fn with_options(
                graph1: &'g G,
                graph2: &'g G,
                vertex_comparator: Option<VertexComparator<'g, G>>,
                edge_comparator: Option<EdgeComparator<'g, G>>,
                cache_edges: bool,
            ) -> Result<Self> {
                check_types(graph1, graph2)?;
                Ok($name {
                    vertex_comparator,
                    edge_comparator,
                    ordering1: GraphOrdering::with_options(graph1, true, cache_edges),
                    ordering2: GraphOrdering::with_options(graph2, true, cache_edges),
                })
            }

            /// The lazy iterator over all mappings.
            pub fn iter(&self) -> VF2MappingIterator<'_, 'g, G> {
                VF2MappingIterator::new(
                    &self.ordering1,
                    &self.ordering2,
                    self.vertex_comparator.as_ref(),
                    self.edge_comparator.as_ref(),
                    $kind,
                )
            }
        }

        impl<'g, G: $crate::graph::IncidenceGraph> IsomorphismInspector<'g, G> for $name<'g, G> {
            fn mappings(&self) -> Box<dyn Iterator<Item = IsomorphicGraphMapping<'g, G>> + '_> {
                Box::new(self.iter())
            }
        }
    };
}

vf2_inspector!(
    /// `VF2GraphIsomorphismInspector`: all isomorphisms between two graphs.
    VF2GraphIsomorphismInspector,
    Kind::Graph
);
vf2_inspector!(
    /// `VF2SubgraphIsomorphismInspector`: all embeddings of the second graph as an induced
    /// subgraph of the first.
    VF2SubgraphIsomorphismInspector,
    Kind::Subgraph
);
