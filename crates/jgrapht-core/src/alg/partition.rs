//! Bipartiteness.

use super::util::Indexed;
use crate::graph::Graph;
use std::collections::VecDeque;

/// 2-colouring of the vertex indices (direction ignored); `None` if an odd cycle exists.
pub(crate) fn two_color<G: Graph>(ix: &Indexed<'_, G>) -> Option<Vec<bool>> {
    let n = ix.n();
    let mut side: Vec<Option<bool>> = vec![None; n];
    for r in 0..n {
        if side[r].is_some() {
            continue;
        }
        side[r] = Some(false);
        let mut q = VecDeque::from([r]);
        while let Some(v) = q.pop_front() {
            for &(u, _) in ix.out[v].iter().chain(&ix.inn[v]) {
                match side[u] {
                    None => {
                        side[u] = Some(!side[v].unwrap());
                        q.push_back(u);
                    }
                    Some(s) if s == side[v].unwrap() => return None,
                    _ => {}
                }
            }
        }
    }
    Some(side.into_iter().map(Option::unwrap).collect())
}

/// Splits the vertices into two independent sets, or `None` if the graph is not bipartite.
pub fn bipartite_partition<G: Graph>(g: &G) -> Option<(Vec<G::Vertex>, Vec<G::Vertex>)> {
    let ix = Indexed::new(g);
    let side = two_color(&ix)?;
    let pick = |s: bool| {
        (0..ix.n())
            .filter(|&v| side[v] == s)
            .map(|v| ix.vertex(v))
            .collect()
    };
    Some((pick(false), pick(true)))
}

pub fn is_bipartite<G: Graph>(g: &G) -> bool {
    two_color(&Indexed::new(g)).is_some()
}
