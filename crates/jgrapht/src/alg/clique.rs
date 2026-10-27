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

//! Cliques and independent sets.

use super::color::ChordalGraphColoring;
use super::cycle::IterationOrder;
use super::util::Indexed;
use crate::graph::{Graph, UndirectedGraph};
use std::collections::HashSet;

fn adjacency<G: Graph>(ix: &Indexed<'_, G>) -> Vec<HashSet<usize>> {
    (0..ix.n())
        .map(|v| {
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v)
                .collect()
        })
        .collect()
}

fn bron_kerbosch(
    adj: &[HashSet<usize>],
    r: &mut Vec<usize>,
    mut p: Vec<usize>,
    mut x: Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    if p.is_empty() {
        if x.is_empty() {
            out.push(r.clone());
        }
        return;
    }
    // pivot maximising |P ∩ N(u)|
    let pivot = p
        .iter()
        .chain(&x)
        .copied()
        .max_by_key(|&u| p.iter().filter(|v| adj[u].contains(v)).count())
        .unwrap();
    let candidates: Vec<usize> = p
        .iter()
        .copied()
        .filter(|v| !adj[pivot].contains(v))
        .collect();
    for v in candidates {
        r.push(v);
        let np = p.iter().copied().filter(|u| adj[v].contains(u)).collect();
        let nx = x.iter().copied().filter(|u| adj[v].contains(u)).collect();
        bron_kerbosch(adj, r, np, nx, out);
        r.pop();
        p.retain(|&u| u != v);
        x.push(v);
    }
}

/// All maximal cliques (Bron–Kerbosch with pivoting). Direction is ignored.
pub fn maximal_cliques<G: Graph>(g: &G) -> Vec<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let adj = adjacency(&ix);
    let mut out = Vec::new();
    bron_kerbosch(
        &adj,
        &mut Vec::new(),
        (0..ix.n()).collect(),
        Vec::new(),
        &mut out,
    );
    out.iter().map(|c| ix.vertices_of(c)).collect()
}

/// A maximum clique (largest maximal clique).
pub fn maximum_clique<G: Graph>(g: &G) -> Vec<G::Vertex> {
    maximal_cliques(g)
        .into_iter()
        .max_by_key(Vec::len)
        .unwrap_or_default()
}

/// Whether the vertices are pairwise adjacent.
pub fn is_clique<G: Graph>(g: &G, vs: &[G::Vertex]) -> bool {
    vs.iter().enumerate().all(|(i, a)| {
        vs[i + 1..]
            .iter()
            .all(|b| g.contains_edge_between(a, b) || g.contains_edge_between(b, a))
    })
}

/// Greedy independent set: repeatedly take a minimum-degree vertex and drop its neighbours.
pub fn greedy_independent_set<G: Graph>(g: &G) -> Vec<G::Vertex> {
    let ix = Indexed::new(g);
    let adj = adjacency(&ix);
    let mut alive = vec![true; ix.n()];
    let mut set = Vec::new();
    while let Some(v) = (0..ix.n())
        .filter(|&v| alive[v])
        .min_by_key(|&v| adj[v].iter().filter(|&&u| alive[u]).count())
    {
        set.push(v);
        alive[v] = false;
        adj[v].iter().for_each(|&u| alive[u] = false);
    }
    ix.vertices_of(&set)
}

/// A maximum independent set, computed exactly as a maximum clique of the complement
/// (exponential in the worst case; intended for small graphs).
pub fn maximum_independent_set<G: Graph>(g: &G) -> Vec<G::Vertex> {
    let ix = Indexed::new(g);
    let adj = adjacency(&ix);
    let co: Vec<HashSet<usize>> = (0..ix.n())
        .map(|v| {
            (0..ix.n())
                .filter(|&u| u != v && !adj[v].contains(&u))
                .collect()
        })
        .collect();
    let mut out = Vec::new();
    bron_kerbosch(
        &co,
        &mut Vec::new(),
        (0..ix.n()).collect(),
        Vec::new(),
        &mut out,
    );
    let best = out.into_iter().max_by_key(Vec::len).unwrap_or_default();
    ix.vertices_of(&best)
}

pub fn is_independent_set<G: Graph>(g: &G, vs: &[G::Vertex]) -> bool {
    vs.iter().enumerate().all(|(i, a)| {
        vs[i + 1..]
            .iter()
            .all(|b| !g.contains_edge_between(a, b) && !g.contains_edge_between(b, a))
    })
}

// ---------------------------------------------------------------------------------------------
// JGraphT's clique classes

use super::interfaces::MaximalCliqueEnumerationAlgorithm;
use crate::graph::{GraphExt, GraphMut, GraphMutExt, props};
use crate::traverse::DegeneracyOrderingIterator;
use crate::util::HashableSet;
use crate::{GraphError, Result};
use indexmap::IndexSet;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Shared state of the Bron–Kerbosch finders (`BaseBronKerboschCliqueFinder`).
struct BronKerboschBase<'g, G: Graph> {
    graph: &'g G,
    /// `None`: no time limit
    timeout: Option<Duration>,
    time_limit_reached: bool,
    cliques: Option<Vec<HashSet<G::Vertex>>>,
}

impl<'g, G: Graph> BronKerboschBase<'g, G> {
    fn new(graph: &'g G, timeout: Option<Duration>) -> Self {
        BronKerboschBase {
            graph,
            timeout,
            time_limit_reached: false,
            cliques: None,
        }
    }

    fn deadline(&self) -> Option<Instant> {
        self.timeout.and_then(|t| Instant::now().checked_add(t))
    }

    fn check_simple(&self) -> Result<()> {
        if props::is_simple(self.graph) {
            Ok(())
        } else {
            Err(GraphError::InvalidArgument("Graph must be simple".into()))
        }
    }

    fn neighbors(&self, v: &G::Vertex) -> HashSet<G::Vertex> {
        self.graph
            .edges_of(v).unwrap()
            .filter_map(|e| self.graph.opposite_vertex(e, v).cloned())
            .collect()
    }
}

fn expired(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|d| Instant::now() > d)
}

/// The classic Bron–Kerbosch algorithm (`BronKerboschCliqueFinder`).
pub struct BronKerboschCliqueFinder<'g, G: Graph> {
    base: BronKerboschBase<'g, G>,
}

impl<'g, G: Graph> BronKerboschCliqueFinder<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        BronKerboschCliqueFinder {
            base: BronKerboschBase::new(graph, None),
        }
    }
    /// Stops enumerating after `timeout` ([`is_time_limit_reached`](Self::is_time_limit_reached)
    /// then reports whether that happened).
    pub fn with_timeout(graph: &'g G, timeout: Duration) -> Result<Self> {
        if timeout.is_zero() {
            return Ok(Self::new(graph));
        }
        Ok(BronKerboschCliqueFinder {
            base: BronKerboschBase::new(graph, Some(timeout)),
        })
    }
    pub fn is_time_limit_reached(&self) -> bool {
        self.base.time_limit_reached
    }

    fn find_cliques(
        &mut self,
        potential: &mut Vec<G::Vertex>,
        candidates: &mut Vec<G::Vertex>,
        already_found: &mut Vec<G::Vertex>,
        deadline: Option<Instant>,
    ) {
        let g = self.base.graph;
        // a vertex already examined and adjacent to all candidates: nothing new down this branch
        for v in already_found.iter() {
            if candidates.iter().all(|c| g.contains_edge_between(v, c)) {
                return;
            }
        }
        for candidate in candidates.clone() {
            if expired(deadline) {
                self.base.time_limit_reached = true;
                return;
            }
            potential.push(candidate.clone());
            if let Some(pos) = candidates.iter().position(|c| *c == candidate) {
                candidates.remove(pos);
            }
            let mut new_candidates: Vec<G::Vertex> = candidates
                .iter()
                .filter(|c| g.contains_edge_between(&candidate, c))
                .cloned()
                .collect();
            let mut new_already: Vec<G::Vertex> = already_found
                .iter()
                .filter(|f| g.contains_edge_between(&candidate, f))
                .cloned()
                .collect();
            if new_candidates.is_empty() && new_already.is_empty() {
                self.base
                    .cliques
                    .as_mut()
                    .unwrap()
                    .push(potential.iter().cloned().collect());
            } else {
                self.find_cliques(potential, &mut new_candidates, &mut new_already, deadline);
            }
            already_found.push(candidate.clone());
            if let Some(pos) = potential.iter().position(|c| *c == candidate) {
                potential.remove(pos);
            }
        }
    }
}

impl<G: Graph> MaximalCliqueEnumerationAlgorithm<G::Vertex> for BronKerboschCliqueFinder<'_, G> {
    fn cliques(&mut self) -> Result<&[HashSet<G::Vertex>]> {
        if self.base.cliques.is_none() {
            self.base.check_simple()?;
            self.base.cliques = Some(Vec::new());
            let deadline = self.base.deadline();
            let mut candidates: Vec<G::Vertex> = self.base.graph.vertices().cloned().collect();
            self.find_cliques(&mut Vec::new(), &mut candidates, &mut Vec::new(), deadline);
        }
        Ok(self.base.cliques.as_deref().unwrap())
    }
}

/// Bron–Kerbosch with pivoting (`PivotBronKerboschCliqueFinder`).
pub struct PivotBronKerboschCliqueFinder<'g, G: Graph> {
    base: BronKerboschBase<'g, G>,
}

impl<'g, G: Graph> PivotBronKerboschCliqueFinder<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        PivotBronKerboschCliqueFinder {
            base: BronKerboschBase::new(graph, None),
        }
    }
    pub fn with_timeout(graph: &'g G, timeout: Duration) -> Result<Self> {
        if timeout.is_zero() {
            return Ok(Self::new(graph));
        }
        Ok(PivotBronKerboschCliqueFinder {
            base: BronKerboschBase::new(graph, Some(timeout)),
        })
    }
    pub fn is_time_limit_reached(&self) -> bool {
        self.base.time_limit_reached
    }

    fn choose_pivot(&self, p: &IndexSet<G::Vertex>, x: &IndexSet<G::Vertex>) -> Option<G::Vertex> {
        let g = self.base.graph;
        let mut best: Option<(usize, &G::Vertex)> = None;
        for u in p.iter().chain(x.iter()) {
            let count = g
                .edges_of(u).unwrap()
                .filter(|e| g.opposite_vertex(e, u).is_some_and(|w| p.contains(w)))
                .count();
            if best.is_none_or(|(c, _)| count > c) {
                best = Some((count, u));
            }
        }
        best.map(|(_, u)| u.clone())
    }

    fn find_cliques(
        &mut self,
        p: &mut IndexSet<G::Vertex>,
        r: &IndexSet<G::Vertex>,
        x: &mut IndexSet<G::Vertex>,
        deadline: Option<Instant>,
    ) {
        if p.is_empty() && x.is_empty() {
            self.base
                .cliques
                .as_mut()
                .unwrap()
                .push(r.iter().cloned().collect());
            return;
        }
        if expired(deadline) {
            self.base.time_limit_reached = true;
            return;
        }
        let Some(u) = self.choose_pivot(p, x) else {
            return;
        };
        let u_neighbors = self.base.neighbors(&u);
        let candidates: Vec<G::Vertex> = p
            .iter()
            .filter(|v| !u_neighbors.contains(*v))
            .cloned()
            .collect();
        for v in candidates {
            let v_neighbors = self.base.neighbors(&v);
            let mut new_p: IndexSet<G::Vertex> = p
                .iter()
                .filter(|w| v_neighbors.contains(*w))
                .cloned()
                .collect();
            let mut new_x: IndexSet<G::Vertex> = x
                .iter()
                .filter(|w| v_neighbors.contains(*w))
                .cloned()
                .collect();
            let mut new_r = r.clone();
            new_r.insert(v.clone());
            self.find_cliques(&mut new_p, &new_r, &mut new_x, deadline);
            p.shift_remove(&v);
            x.insert(v);
        }
    }
}

impl<G: Graph> MaximalCliqueEnumerationAlgorithm<G::Vertex>
    for PivotBronKerboschCliqueFinder<'_, G>
{
    fn cliques(&mut self) -> Result<&[HashSet<G::Vertex>]> {
        if self.base.cliques.is_none() {
            self.base.check_simple()?;
            self.base.cliques = Some(Vec::new());
            let deadline = self.base.deadline();
            let mut p: IndexSet<G::Vertex> = self.base.graph.vertices().cloned().collect();
            self.find_cliques(&mut p, &IndexSet::new(), &mut IndexSet::new(), deadline);
        }
        Ok(self.base.cliques.as_deref().unwrap())
    }
}

/// Bron–Kerbosch over a degeneracy ordering with pivoting
/// (`DegeneracyBronKerboschCliqueFinder`).
pub struct DegeneracyBronKerboschCliqueFinder<'g, G: Graph> {
    inner: PivotBronKerboschCliqueFinder<'g, G>,
}

impl<'g, G: Graph> DegeneracyBronKerboschCliqueFinder<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        DegeneracyBronKerboschCliqueFinder {
            inner: PivotBronKerboschCliqueFinder::new(graph),
        }
    }
    pub fn with_timeout(graph: &'g G, timeout: Duration) -> Result<Self> {
        Ok(DegeneracyBronKerboschCliqueFinder {
            inner: PivotBronKerboschCliqueFinder::with_timeout(graph, timeout)?,
        })
    }
    pub fn is_time_limit_reached(&self) -> bool {
        self.inner.is_time_limit_reached()
    }
}

impl<G: Graph> MaximalCliqueEnumerationAlgorithm<G::Vertex>
    for DegeneracyBronKerboschCliqueFinder<'_, G>
{
    fn cliques(&mut self) -> Result<&[HashSet<G::Vertex>]> {
        let inner = &mut self.inner;
        if inner.base.cliques.is_none() {
            inner.base.check_simple()?;
            inner.base.cliques = Some(Vec::new());
            let deadline = inner.base.deadline();
            let ordering: Vec<G::Vertex> = DegeneracyOrderingIterator::new(inner.base.graph)
                .cloned()
                .collect();
            for i in 0..ordering.len() {
                let vi_neighbors = inner.base.neighbors(&ordering[i]);
                let mut p: IndexSet<G::Vertex> = ordering[i + 1..]
                    .iter()
                    .filter(|v| vi_neighbors.contains(*v))
                    .cloned()
                    .collect();
                let r: IndexSet<G::Vertex> = IndexSet::from([ordering[i].clone()]);
                let mut x: IndexSet<G::Vertex> = ordering[..i]
                    .iter()
                    .filter(|v| vi_neighbors.contains(*v))
                    .cloned()
                    .collect();
                inner.find_cliques(&mut p, &r, &mut x, deadline);
            }
        }
        Ok(inner.base.cliques.as_deref().unwrap())
    }
}

/// Maximum clique of a chordal graph (`ChordalGraphMaxCliqueFinder`): the vertex of the highest
/// colour in the optimal colouring of [`ChordalGraphColoring`], together with its predecessors in
/// the perfect elimination order.
pub struct ChordalGraphMaxCliqueFinder<'g, G: UndirectedGraph> {
    graph: &'g G,
    iteration_order: IterationOrder,
    maximum_clique: Option<HashSet<G::Vertex>>,
    is_chordal: bool,
}

impl<'g, G: UndirectedGraph> ChordalGraphMaxCliqueFinder<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self::with_order(graph, IterationOrder::Mcs)
    }

    pub fn with_order(graph: &'g G, iteration_order: IterationOrder) -> Self {
        ChordalGraphMaxCliqueFinder {
            graph,
            iteration_order,
            maximum_clique: None,
            is_chordal: true,
        }
    }

    fn lazy_compute_maximum_clique(&mut self) {
        if self.maximum_clique.is_none() && self.is_chordal {
            let mut cgc = ChordalGraphColoring::with_order(self.graph, self.iteration_order);
            let coloring = cgc.coloring();
            let perfect_elimination_order = cgc.perfect_elimination_order();
            let (Some(coloring), Some(perfect_elimination_order)) =
                (coloring, perfect_elimination_order)
            else {
                self.is_chordal = false; // graph isn't chordal
                return;
            };

            // finds the vertex with the maximum cardinality predecessor list
            let vertex_in_order = super::color::vertex_in_order(&perfect_elimination_order);
            let max_vertex = perfect_elimination_order
                .iter()
                .reduce(|best, v| if coloring.colors[v] > coloring.colors[best] { v } else { best });
            self.maximum_clique = Some(match max_vertex {
                None => HashSet::new(),
                Some(v) => {
                    let mut clique = super::color::predecessors(self.graph, &vertex_in_order, v);
                    clique.insert(v.clone());
                    clique
                }
            });
        }
    }

    /// A maximum clique, or `None` if the graph is not chordal.
    pub fn clique(&mut self) -> Option<HashSet<G::Vertex>> {
        self.lazy_compute_maximum_clique();
        self.maximum_clique.clone()
    }
}

/// Decomposition of a graph into atoms by clique minimal separators, via a minimal triangulation
/// (MCS-M); Berry, Pogorelcnik, Simonet (`CliqueMinimalSeparatorDecomposition`).
pub struct CliqueMinimalSeparatorDecomposition<'g, G: Graph> {
    graph: &'g G,
    chordal_graph: Option<crate::UGraph<G::Vertex>>,
    fill_edges: Vec<(G::Vertex, G::Vertex)>,
    meo: Vec<G::Vertex>,
    generators: Vec<G::Vertex>,
    separators: Option<Vec<HashableSet<G::Vertex>>>,
    atoms: Option<Vec<HashableSet<G::Vertex>>>,
    full_component_count: HashMap<HashableSet<G::Vertex>, usize>,
}

impl<'g, G: Graph> CliqueMinimalSeparatorDecomposition<'g, G> {
    pub fn new(graph: &'g G) -> Result<Self> {
        props::require_undirected(graph)?;
        Ok(CliqueMinimalSeparatorDecomposition {
            graph,
            chordal_graph: None,
            fill_edges: Vec::new(),
            meo: Vec::new(),
            generators: Vec::new(),
            separators: None,
            atoms: None,
            full_component_count: HashMap::new(),
        })
    }

    /// Simple copy of the graph: no loops, parallel edges collapsed.
    fn simple_copy(g: &G) -> crate::UGraph<G::Vertex> {
        let mut copy = crate::UGraph::<G::Vertex>::simple();
        copy.add_vertices(g.vertices().cloned());
        for e in g.edges() {
            let (a, b) = (g.edge_source(e).unwrap(), g.edge_target(e).unwrap());
            if a != b {
                let _ = copy.add_edge(a, b);
            }
        }
        copy
    }

    fn compute_minimal_triangulation(&mut self) {
        if self.chordal_graph.is_some() {
            return;
        }
        let mut chordal = crate::UGraph::<G::Vertex>::simple();
        chordal.add_vertices(self.graph.vertices().cloned());
        let mut gprime = Self::simple_copy(self.graph);
        let mut s: i64 = -1;
        let mut labels: indexmap::IndexMap<G::Vertex, i64> =
            gprime.vertices().map(|v| (v.clone(), 0)).collect();
        let n = self.graph.vertex_count();
        for _ in 1..=n {
            // vertex of maximum label (first one wins ties)
            let mut max: Option<(&G::Vertex, i64)> = None;
            for (v, &l) in &labels {
                if max.is_none_or(|(_, m)| l > m) {
                    max = Some((v, l));
                }
            }
            let v = max.unwrap().0.clone();
            let mut neighbors_y: Vec<G::Vertex> = gprime.neighbor_list_of(&v);
            if labels[&v] <= s {
                self.generators.push(v.clone());
            }
            s = labels[&v];
            let mut reached: HashSet<G::Vertex> = HashSet::from([v.clone()]);
            let mut reach: HashMap<i64, IndexSet<G::Vertex>> = HashMap::new();
            for y in &neighbors_y {
                reached.insert(y.clone());
                reach.entry(labels[y]).or_default().insert(y.clone());
            }
            for j in 0..n as i64 {
                if !reach.contains_key(&j) {
                    continue;
                }
                while let Some(y) = reach.get_mut(&j).and_then(|set| set.shift_remove_index(0)) {
                    for z in gprime.neighbor_list_of(&y) {
                        if !reached.contains(&z) {
                            reached.insert(z.clone());
                            if labels[&z] > j {
                                neighbors_y.push(z.clone());
                                self.fill_edges.push((v.clone(), z.clone()));
                                reach.entry(labels[&z]).or_default().insert(z);
                            } else {
                                reach.entry(j).or_default().insert(z);
                            }
                        }
                    }
                }
            }
            for y in &neighbors_y {
                let _ = chordal.add_edge(&v, y);
                *labels.get_mut(y).unwrap() += 1;
            }
            self.meo.push(v.clone());
            gprime.remove_vertex(&v);
            labels.shift_remove(&v);
        }
        self.chordal_graph = Some(chordal);
    }

    fn is_clique(g: &G, vertices: &HashSet<G::Vertex>) -> bool {
        vertices.iter().all(|a| {
            vertices
                .iter()
                .all(|b| a == b || g.contains_edge_between(a, b))
        })
    }

    fn compute_atoms(&mut self) {
        if self.atoms.is_some() {
            return;
        }
        self.compute_minimal_triangulation();
        let mut separators: Vec<HashableSet<G::Vertex>> = Vec::new();
        let mut gprime = Self::simple_copy(self.graph);
        let mut hprime = Self::simple_copy_of_ugraph(self.chordal_graph.as_ref().unwrap());
        let mut atoms: Vec<HashableSet<G::Vertex>> = Vec::new();
        for v in self.meo.iter().rev() {
            if self.generators.contains(v) {
                let separator: HashableSet<G::Vertex> =
                    hprime.neighbor_list_of(v).into_iter().collect();
                if Self::is_clique(self.graph, &separator.0) {
                    if !separator.is_empty() {
                        if let Some(c) = self.full_component_count.get_mut(&separator) {
                            *c += 1;
                        } else {
                            self.full_component_count.insert(separator.clone(), 2);
                            separators.push(separator.clone());
                        }
                    }
                    let mut tmp = gprime.clone();
                    for s in separator.iter() {
                        tmp.remove_vertex(s);
                    }
                    let components = crate::alg::connectivity::connected_components(&tmp);
                    assert!(components.len() > 1, "separator did not separate the graph");
                    for component in components {
                        if component.contains(v) {
                            for c in &component {
                                gprime.remove_vertex(c);
                            }
                            let mut atom: HashableSet<G::Vertex> = component.into_iter().collect();
                            atom.extend(separator.iter().cloned());
                            if !atoms.contains(&atom) {
                                atoms.push(atom);
                            }
                            break;
                        }
                    }
                }
            }
            hprime.remove_vertex(v);
        }
        if gprime.vertex_count() > 0 {
            let atom: HashableSet<G::Vertex> = gprime.vertices().cloned().collect();
            if !atoms.contains(&atom) {
                atoms.push(atom);
            }
        }
        self.separators = Some(separators);
        self.atoms = Some(atoms);
    }

    fn simple_copy_of_ugraph(g: &crate::UGraph<G::Vertex>) -> crate::UGraph<G::Vertex> {
        g.clone()
    }

    pub fn is_chordal(&mut self) -> bool {
        self.compute_minimal_triangulation();
        self.chordal_graph.as_ref().unwrap().edge_count() == self.graph.edge_count()
    }

    /// The edges added by the triangulation, as endpoint pairs.
    pub fn fill_edges(&mut self) -> &[(G::Vertex, G::Vertex)] {
        self.compute_minimal_triangulation();
        &self.fill_edges
    }

    /// A minimal triangulation (chordal supergraph) of the graph.
    pub fn minimal_triangulation(&mut self) -> &crate::UGraph<G::Vertex> {
        self.compute_minimal_triangulation();
        self.chordal_graph.as_ref().unwrap()
    }

    pub fn generators(&mut self) -> &[G::Vertex] {
        self.compute_minimal_triangulation();
        &self.generators
    }

    /// The minimal elimination ordering.
    pub fn meo(&mut self) -> &[G::Vertex] {
        self.compute_minimal_triangulation();
        &self.meo
    }

    pub fn atoms(&mut self) -> &[HashableSet<G::Vertex>] {
        self.compute_atoms();
        self.atoms.as_deref().unwrap()
    }

    pub fn separators(&mut self) -> &[HashableSet<G::Vertex>] {
        self.compute_atoms();
        self.separators.as_deref().unwrap()
    }

    /// For each clique minimal separator, how many full components it has.
    pub fn full_component_count(&mut self) -> &HashMap<HashableSet<G::Vertex>, usize> {
        self.compute_atoms();
        &self.full_component_count
    }

    pub fn graph(&self) -> &G {
        self.graph
    }
}
