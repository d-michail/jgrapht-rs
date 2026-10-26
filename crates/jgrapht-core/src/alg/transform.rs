//! Graph transformations that build a new graph into a caller-provided target.
//! (Reversal and undirected views live in [`crate::view`].)

use super::util::Indexed;
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
/// (undirected) or one ends where the other starts (directed).
pub fn line_graph_into<G, T>(g: &G, target: &mut T) -> Result<()>
where
    G: Graph,
    T: GraphMut<Vertex = G::Edge>,
{
    let ix = Indexed::new(g);
    for e in &ix.es {
        target.add_vertex((*e).clone());
    }
    for v in 0..ix.n() {
        if ix.directed {
            for &(_, a) in &ix.inn[v] {
                for &(_, b) in &ix.out[v] {
                    if a != b {
                        target.add_edge(ix.es[a], ix.es[b])?;
                    }
                }
            }
        } else {
            let inc = &ix.out[v];
            for (i, &(_, a)) in inc.iter().enumerate() {
                for &(_, b) in &inc[i + 1..] {
                    target.add_edge(ix.es[a], ix.es[b])?;
                }
            }
        }
    }
    Ok(())
}

/// Contracts `e` in place: its target is merged into its source, edges are re-attached
/// (edges that would become loops are dropped unless the graph allows them).
pub fn contract_edge<G: GraphMut>(g: &mut G, e: &G::Edge) -> Result<()> {
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
