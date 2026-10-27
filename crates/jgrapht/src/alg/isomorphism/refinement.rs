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

use crate::graph::IncidenceGraph;
use crate::graph::OutgoingGraph;
use super::mapping::IsomorphicGraphMapping;
use super::vf2::check_types;
use crate::alg::color::refine_colors;
use crate::error::Result;
use crate::graph::props::is_forest;
use crate::graph::{GraphExt};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// `IsomorphismUndecidableException`: colour refinement cannot tell whether the graphs are
/// isomorphic.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Color refinement cannot decide whether the two graphs are isomorphic or not.")]
pub struct IsomorphismUndecidable;

/// Implementation of the color refinement algorithm isomorphism test using its feature of detecting
/// [isomorphism between two graphs](http://mathworld.wolfram.com/GraphIsomorphism.html)
/// as described in C. Berkholz, P. Bonsma, and M. Grohe. Tight lower and upper bounds for the
/// complexity of canonical colour refinement. Theory of Computing
/// Systems,doi:10.1007/s00224-016-9686-0, 2016 (color refinement) The complexity of this algorithm
/// is O(|V| + |E| log |V|).
///
/// # Rust notes
///
/// `ColorRefinementIsomorphismInspector`: refines the colours of the disjoint union of both
/// graphs. It decides isomorphism when the colouring is discrete or both graphs are forests, and
/// otherwise reports the question as undecidable (unless the colour classes already differ).
pub struct ColorRefinementIsomorphismInspector<'g, G: IncidenceGraph> {
    graph1: &'g G,
    graph2: &'g G,
    mapping: Option<IsomorphicGraphMapping<'g, G>>,
    /// `None` until executed, then `Some(None)` when undecidable.
    outcome: Option<Option<bool>>,
    coloring_discrete: bool,
    forest: bool,
}

/// Distinct successors of every vertex, over the disjoint union of `g1` and `g2`.
fn union_successors<G: OutgoingGraph>(g1: &G, g2: &G) -> Vec<Vec<usize>> {
    let mut succ = Vec::new();
    let mut offset = 0;
    for g in [g1, g2] {
        let index: HashMap<&G::Vertex, usize> =
            g.vertices().enumerate().map(|(i, v)| (v, i)).collect();
        for v in g.vertices() {
            let mut seen = HashSet::new();
            succ.push(
                g.out_edges_of(v)
                    .filter_map(|e| g.opposite_vertex(e, v))
                    .map(|w| index[w] + offset)
                    .filter(|w| seen.insert(*w))
                    .collect(),
            );
        }
        offset += g.vertex_count();
    }
    succ
}

impl<'g, G: OutgoingGraph + IncidenceGraph> ColorRefinementIsomorphismInspector<'g, G> {
    /// Fails for multigraphs, mixed graphs and a directed/undirected pair.
    pub fn new(graph1: &'g G, graph2: &'g G) -> Result<Self> {
        check_types(graph1, graph2)?;
        Ok(ColorRefinementIsomorphismInspector {
            graph1,
            graph2,
            mapping: None,
            outcome: None,
            coloring_discrete: false,
            forest: false,
        })
    }

    fn execute(&mut self) {
        let (g1, g2) = (self.graph1, self.graph2);
        if std::ptr::eq(g1, g2) {
            self.mapping = Some(IsomorphicGraphMapping::identity(g1));
            self.outcome = Some(Some(true));
            return;
        }
        if g1.vertex_count() != g2.vertex_count() {
            self.outcome = Some(Some(false));
            return;
        }
        let n = g1.vertex_count();
        let succ = union_successors(g1, g2);
        let (color, classes) = refine_colors(&succ, vec![0; 2 * n]);
        self.outcome = Some(self.compare(&color, classes));
    }

    /// Whether the colour classes of both graphs have equal sizes, and if so whether that
    /// settles the question.
    fn compare(&mut self, color: &[usize], classes: usize) -> Option<bool> {
        let n = self.graph1.vertex_count();
        let mut classes1: Vec<Vec<usize>> = vec![Vec::new(); classes];
        let mut classes2: Vec<Vec<usize>> = vec![Vec::new(); classes];
        for i in 0..n {
            classes1[color[i]].push(i);
            classes2[color[n + i]].push(i);
        }
        if classes1
            .iter()
            .zip(&classes2)
            .any(|(a, b)| a.len() != b.len())
        {
            return Some(false);
        }
        let classes_used = classes1.iter().filter(|c| !c.is_empty()).count();
        if classes_used == n {
            self.coloring_discrete = true;
        } else if is_forest(self.graph1) && is_forest(self.graph2) {
            self.forest = true;
        } else {
            return None;
        }
        let vs1: Vec<&G::Vertex> = self.graph1.vertices().collect();
        let vs2: Vec<&G::Vertex> = self.graph2.vertices().collect();
        let mut forward = IndexMap::new();
        let mut backward = IndexMap::new();
        for (c1, c2) in classes1.iter().zip(&classes2) {
            for (&i, &j) in c1.iter().zip(c2) {
                forward.insert(vs1[i].clone(), vs2[j].clone());
                backward.insert(vs2[j].clone(), vs1[i].clone());
            }
        }
        self.mapping = Some(IsomorphicGraphMapping::from_maps(
            forward,
            backward,
            self.graph1,
            self.graph2,
        ));
        Some(true)
    }

    /// Whether the graphs are isomorphic; fails when colour refinement cannot decide.
    pub fn isomorphism_exists(&mut self) -> std::result::Result<bool, IsomorphismUndecidable> {
        if self.outcome.is_none() {
            self.execute();
        }
        self.outcome.unwrap().ok_or(IsomorphismUndecidable)
    }

    /// The (at most one) mapping found; empty when not isomorphic or undecidable.
    pub fn mappings(&mut self) -> Vec<IsomorphicGraphMapping<'g, G>> {
        if self.outcome.is_none() {
            self.execute();
        }
        match (&self.outcome, &self.mapping) {
            (Some(Some(true)), Some(m)) => vec![m.clone()],
            _ => Vec::new(),
        }
    }

    /// Whether every vertex got its own colour.
    pub fn is_coloring_discrete(&mut self) -> bool {
        if self.outcome.is_none() {
            self.execute();
        }
        self.coloring_discrete
    }

    /// Whether the answer was settled because both graphs are forests.
    pub fn is_forest(&mut self) -> bool {
        if self.outcome.is_none() {
            self.execute();
        }
        self.forest
    }
}
