//! Neighbourhood-based vertex similarity and link prediction scores (direction ignored).

use super::util::Indexed;
use crate::graph::Graph;
use std::collections::{HashMap, HashSet};

fn nbrs<G: Graph>(ix: &Indexed<'_, G>) -> Vec<HashSet<usize>> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Measure {
    CommonNeighbors,
    TotalNeighbors,
    Jaccard,
    /// Sørensen–Dice
    Sorensen,
    /// Salton (cosine)
    Salton,
    HubPromoted,
    HubDepressed,
    LeichtHolmeNewman,
    PreferentialAttachment,
    AdamicAdar,
    ResourceAllocation,
}

fn score(nb: &[HashSet<usize>], m: Measure, u: usize, v: usize) -> f64 {
    let (a, b) = (&nb[u], &nb[v]);
    let common: Vec<usize> = a.intersection(b).copied().collect();
    let c = common.len() as f64;
    let (da, db) = (a.len() as f64, b.len() as f64);
    let ratio = |x: f64, y: f64| if y == 0.0 { 0.0 } else { x / y };
    match m {
        Measure::CommonNeighbors => c,
        Measure::TotalNeighbors => a.union(b).count() as f64,
        Measure::Jaccard => ratio(c, a.union(b).count() as f64),
        Measure::Sorensen => ratio(2.0 * c, da + db),
        Measure::Salton => ratio(c, (da * db).sqrt()),
        Measure::HubPromoted => ratio(c, da.min(db)),
        Measure::HubDepressed => ratio(c, da.max(db)),
        Measure::LeichtHolmeNewman => ratio(c, da * db),
        Measure::PreferentialAttachment => da * db,
        Measure::AdamicAdar => common
            .iter()
            .filter(|&&w| nb[w].len() > 1)
            .map(|&w| 1.0 / (nb[w].len() as f64).ln())
            .sum(),
        Measure::ResourceAllocation => common.iter().map(|&w| 1.0 / nb[w].len() as f64).sum(),
    }
}

/// Score of the pair `(u, v)` under the given measure; `None` if a vertex is missing.
pub fn pair_score<G: Graph>(g: &G, m: Measure, u: &G::Vertex, v: &G::Vertex) -> Option<f64> {
    let ix = Indexed::new(g);
    Some(score(&nbrs(&ix), m, *ix.idx.get(u)?, *ix.idx.get(v)?))
}

/// Scores `u` against every vertex it is not yet adjacent to, best first.
pub fn predict<G: Graph>(g: &G, m: Measure, u: &G::Vertex) -> Option<Vec<(G::Vertex, f64)>> {
    let ix = Indexed::new(g);
    let nb = nbrs(&ix);
    let ui = *ix.idx.get(u)?;
    let mut out: Vec<(G::Vertex, f64)> = (0..ix.n())
        .filter(|&v| v != ui && !nb[ui].contains(&v))
        .map(|v| (ix.vertex(v), score(&nb, m, ui, v)))
        .collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1));
    Some(out)
}

/// SimRank similarity of all vertex pairs (in-neighbour based), `c` the decay factor.
pub fn simrank<G: Graph>(g: &G, c: f64, iterations: usize) -> HashMap<(G::Vertex, G::Vertex), f64> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let inn: Vec<Vec<usize>> = (0..n)
        .map(|v| ix.inn[v].iter().map(|&(u, _)| u).collect())
        .collect();
    let mut s = vec![vec![0.0; n]; n];
    (0..n).for_each(|i| s[i][i] = 1.0);
    for _ in 0..iterations {
        let mut next = vec![vec![0.0; n]; n];
        for a in 0..n {
            next[a][a] = 1.0;
            for b in 0..n {
                if a != b && !inn[a].is_empty() && !inn[b].is_empty() {
                    let sum: f64 = inn[a]
                        .iter()
                        .map(|&x| inn[b].iter().map(|&y| s[x][y]).sum::<f64>())
                        .sum();
                    next[a][b] = c * sum / (inn[a].len() * inn[b].len()) as f64;
                }
            }
        }
        s = next;
    }
    let mut out = HashMap::new();
    for a in 0..n {
        for b in 0..n {
            out.insert((ix.vertex(a), ix.vertex(b)), s[a][b]);
        }
    }
    out
}
