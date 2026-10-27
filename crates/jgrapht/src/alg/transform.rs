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

//! Graph transformations that build a new graph into a caller-provided target.
//! (Reversal and undirected views live in [`crate::view`].)

use crate::graph::IncidenceGraph;
use crate::Result;
use crate::graph::{Graph, GraphMut, GraphMutExt};

/// Complement: same vertices, an edge wherever `g` has none (no loops unless `loops`).
pub fn complement_into<G, T>(g: &G, target: &mut T, loops: bool) -> Result<()>
where
    G: Graph,
    T: GraphMut<Vertex = G::Vertex>,
{
    target.add_vertices(g.vertices().cloned());
    let directed = g.graph_type().directed;
    let vs: Vec<_> = g.vertices().collect();
    for (i, u) in vs.iter().enumerate() {
        for (j, v) in vs.iter().enumerate() {
            if (!directed && j < i) || (i == j && !loops) {
                continue;
            }
            if !g.contains_edge_between(u, v) {
                target.add_edge(u, v)?;
            }
        }
    }
    Ok(())
}

/// Line graph: one vertex per edge of `g`; two are adjacent if the edges share an endpoint
/// (undirected) or one ends where the other starts (directed). Follows JGraphT's
/// `LineGraphConverter`: a self-loop of a directed graph becomes a loop of the line graph, so the
/// target must allow loops in that case.
pub fn line_graph_into<G, T>(g: &G, target: &mut T) -> Result<()>
where
    G: IncidenceGraph,
    T: GraphMut<Vertex = G::Edge>,
{
    line_graph_impl(g, target, None::<fn(&G::Edge, &G::Edge) -> f64>)
}

/// [`line_graph_into`] with the weight of each line-graph edge given by `weight(e1, e2)`.
pub fn line_graph_weighted_into<G, T>(
    g: &G,
    target: &mut T,
    weight: impl Fn(&G::Edge, &G::Edge) -> f64,
) -> Result<()>
where
    G: IncidenceGraph,
    T: GraphMut<Vertex = G::Edge>,
{
    line_graph_impl(g, target, Some(weight))
}

fn line_graph_impl<G, T, F>(g: &G, target: &mut T, weight: Option<F>) -> Result<()>
where
    G: IncidenceGraph,
    T: GraphMut<Vertex = G::Edge>,
    F: Fn(&G::Edge, &G::Edge) -> f64,
{
    target.add_vertices(g.edges().cloned());
    let link = |target: &mut T, e1: &G::Edge, e2: &G::Edge| -> Result<()> {
        if let Some(edge) = target.add_edge(e1, e2)? {
            if let Some(f) = &weight {
                target.set_edge_weight(&edge, f(e1, e2))?;
            }
        }
        Ok(())
    };
    if g.graph_type().directed {
        for v in g.vertices() {
            for e1 in g.in_edges_of(v) {
                for e2 in g.out_edges_of(v) {
                    link(target, e1, e2)?;
                }
            }
        }
    } else {
        for v in g.vertices() {
            for e1 in g.edges_of(v) {
                for e2 in g.edges_of(v) {
                    if e1 != e2 {
                        link(target, e1, e2)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Contracts `e` in place: its target is merged into its source, edges are re-attached
/// (edges that would become loops are dropped unless the graph allows them).
pub fn contract_edge<G: GraphMut + IncidenceGraph>(g: &mut G, e: &G::Edge) -> Result<()> {
    let (Some(s), Some(t)) = (g.edge_source(e).cloned(), g.edge_target(e).cloned()) else {
        return Err(crate::GraphError::EdgeNotFound);
    };
    if s == t {
        g.remove_edge(e);
        return Ok(());
    }
    let incident: Vec<(G::Edge, G::Vertex, G::Vertex, f64)> = g
        .edges_of(&t)
        .filter(|x| *x != e)
        .map(|x| {
            (
                x.clone(),
                g.edge_source(x).unwrap().clone(),
                g.edge_target(x).unwrap().clone(),
                g.edge_weight(x).unwrap_or(1.0),
            )
        })
        .collect();
    g.remove_vertex(&t);
    for (x, a, b, w) in incident {
        let a = if a == t { s.clone() } else { a };
        let b = if b == t { s.clone() } else { b };
        if a == b && !g.graph_type().self_loops {
            continue;
        }
        if g.add_edge_with(&a, &b, x.clone())? && g.graph_type().weighted {
            g.set_edge_weight(&x, w)?;
        }
    }
    Ok(())
}
