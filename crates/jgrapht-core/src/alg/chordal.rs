//! Chordal graphs (undirected, direction ignored).

use super::util::Indexed;
use crate::graph::Graph;
use crate::traverse::MaximumCardinality;
use std::collections::HashSet;

fn neighbor_sets<G: Graph>(ix: &Indexed<'_, G>) -> Vec<HashSet<usize>> {
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

/// A perfect elimination ordering, or `None` if the graph is not chordal.
pub fn perfect_elimination_order<G: Graph>(g: &G) -> Option<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let nb = neighbor_sets(&ix);
    // reverse of a maximum cardinality search is a PEO iff the graph is chordal
    let order: Vec<usize> = {
        let mut o: Vec<usize> = MaximumCardinality::new(g).map(|v| ix.idx[v]).collect();
        o.reverse();
        o
    };
    let mut pos = vec![0; ix.n()];
    order.iter().enumerate().for_each(|(i, &v)| pos[v] = i);
    for &v in &order {
        let later: Vec<usize> = nb[v].iter().copied().filter(|&u| pos[u] > pos[v]).collect();
        if let Some(&p) = later.iter().min_by_key(|&&u| pos[u]) {
            if later.iter().any(|&u| u != p && !nb[p].contains(&u)) {
                return None;
            }
        }
    }
    Some(ix.vertices_of(&order))
}

pub fn is_chordal<G: Graph>(g: &G) -> bool {
    perfect_elimination_order(g).is_some()
}

/// Maximal cliques of a chordal graph; `None` if not chordal.
pub fn chordal_maximal_cliques<G: Graph>(g: &G) -> Option<Vec<Vec<G::Vertex>>> {
    let peo = perfect_elimination_order(g)?;
    let ix = Indexed::new(g);
    let nb = neighbor_sets(&ix);
    let pos: Vec<usize> = {
        let mut p = vec![0; ix.n()];
        peo.iter().enumerate().for_each(|(i, v)| p[ix.idx[v]] = i);
        p
    };
    let cands: Vec<HashSet<usize>> = peo
        .iter()
        .map(|v| {
            let v = ix.idx[v];
            let mut c: HashSet<usize> =
                nb[v].iter().copied().filter(|&u| pos[u] > pos[v]).collect();
            c.insert(v);
            c
        })
        .collect();
    let mut out = Vec::new();
    for (i, c) in cands.iter().enumerate() {
        let dominated = cands
            .iter()
            .enumerate()
            .any(|(j, d)| j != i && c.len() < d.len() && c.is_subset(d))
            || cands.iter().take(i).any(|d| d == c);
        if !dominated {
            let mut c: Vec<_> = c.iter().copied().collect();
            c.sort_unstable();
            out.push(ix.vertices_of(&c));
        }
    }
    Some(out)
}

/// Chromatic number of a chordal graph (= size of its largest clique).
pub fn chordal_chromatic_number<G: Graph>(g: &G) -> Option<usize> {
    Some(
        chordal_maximal_cliques(g)?
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0),
    )
}

/// A maximum independent set of a chordal graph (greedy along the PEO is optimal).
pub fn chordal_maximum_independent_set<G: Graph>(g: &G) -> Option<Vec<G::Vertex>> {
    let peo = perfect_elimination_order(g)?;
    let ix = Indexed::new(g);
    let nb = neighbor_sets(&ix);
    let mut blocked = vec![false; ix.n()];
    let mut set = Vec::new();
    for v in peo {
        let i = ix.idx[&v];
        if !blocked[i] {
            set.push(v);
            blocked[i] = true;
            nb[i].iter().for_each(|&u| blocked[u] = true);
        }
    }
    Some(set)
}
