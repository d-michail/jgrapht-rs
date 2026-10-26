//! Transitive closure and reduction of directed graphs (in place).

use super::util::Indexed;
use crate::graph::{DirectedGraph, GraphMut};
use crate::{GraphError, Result};

fn reach_matrix<G: DirectedGraph>(ix: &Indexed<'_, G>) -> Vec<Vec<bool>> {
    let n = ix.n();
    let mut r = vec![vec![false; n]; n];
    for s in 0..n {
        let mut stack = vec![s];
        while let Some(v) = stack.pop() {
            for &(u, _) in &ix.out[v] {
                if !r[s][u] {
                    r[s][u] = true;
                    stack.push(u);
                }
            }
        }
    }
    r
}

/// Adds an edge `u -> v` for every pair with a path from `u` to `v`. No self-loops are added.
pub fn transitive_closure<G: DirectedGraph + GraphMut>(g: &mut G) -> Result<()> {
    let pairs = {
        let ix = Indexed::new(&*g);
        let r = reach_matrix(&ix);
        let mut pairs = Vec::new();
        for u in 0..ix.n() {
            for v in 0..ix.n() {
                if u != v && r[u][v] {
                    pairs.push((ix.vertex(u), ix.vertex(v)));
                }
            }
        }
        pairs
    };
    for (u, v) in pairs {
        if !g.contains_edge_between(&u, &v) {
            g.add_edge(&u, &v)?;
        }
    }
    Ok(())
}

/// Removes every edge implied by a longer path (transitive reduction). The graph must be acyclic.
pub fn transitive_reduction<G: DirectedGraph + GraphMut>(g: &mut G) -> Result<()> {
    let redundant: Vec<G::Edge> = {
        let ix = Indexed::new(&*g);
        let r = reach_matrix(&ix);
        if (0..ix.n()).any(|v| r[v][v]) {
            return Err(GraphError::InvalidArgument(
                "transitive reduction requires an acyclic graph".into(),
            ));
        }
        ix.ends
            .iter()
            .enumerate()
            .filter(|&(_, &(u, v))| ix.out[u].iter().any(|&(w, _)| w != v && r[w][v]))
            .map(|(k, _)| ix.es[k].clone())
            .collect()
    };
    for e in redundant {
        g.remove_edge(&e);
    }
    Ok(())
}
