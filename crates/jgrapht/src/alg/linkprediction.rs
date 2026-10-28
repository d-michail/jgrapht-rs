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

//! Neighbourhood-based vertex similarity and link prediction scores (direction ignored).

use super::interfaces::LinkPredictionAlgorithm;
use super::util::Indexed;
use crate::graph::OutgoingGraph;
use crate::graph::{Graph, GraphExt};
use crate::{GraphError, Result};
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

#[non_exhaustive]
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

// ---------------------------------------------------------------------------------------------
// JGraphT's link prediction classes. They use successors / out-degrees, so on directed graphs
// they look at outgoing edges only.

fn not_well_defined<T>(msg: &str) -> Result<T> {
    Err(GraphError::IndexNotWellDefined(msg.to_string()))
}

fn common_successors<G: OutgoingGraph>(
    g: &G,
    u: &G::Vertex,
    v: &G::Vertex,
) -> Result<Vec<G::Vertex>> {
    let gu: HashSet<G::Vertex> = g
        .out_edges_of(u)
        .filter_map(|e| g.opposite_vertex(e, u).cloned())
        .collect();
    let gv: HashSet<G::Vertex> = g
        .out_edges_of(v)
        .filter_map(|e| g.opposite_vertex(e, v).cloned())
        .collect();
    Ok(gu.intersection(&gv).cloned().collect())
}

fn successor_set<G: OutgoingGraph>(g: &G, u: &G::Vertex) -> Result<HashSet<G::Vertex>> {
    Ok(g.out_edges_of(u)
        .filter_map(|e| g.opposite_vertex(e, u).cloned())
        .collect())
}

macro_rules! link_prediction {
    ($(#[$doc:meta])* $name:ident, |$g:ident, $u:ident, $v:ident| $body:block) => {
        $(#[$doc])*
        pub struct $name<'g, G: Graph> {
            graph: &'g G,
        }

        impl<'g, G: Graph> $name<'g, G> {
            pub fn new(graph: &'g G) -> Self {
                $name { graph }
            }
        }

        impl<G: $crate::graph::OutgoingGraph> LinkPredictionAlgorithm<G::Vertex> for $name<'_, G> {
            fn predict(&self, $u: &G::Vertex, $v: &G::Vertex) -> Result<f64> {
                let $g = self.graph;
                $body
            }
        }
    };
}

link_prediction!(
    /// `|N(u) ∩ N(v)| / |N(u) ∪ N(v)|` (1 for `u == v`).
    JaccardCoefficientLinkPrediction, |g, u, v| {
        if u == v {
            return Ok(1.0);
        }
        let (gu, gv) = (successor_set(g, u)?, successor_set(g, v)?);
        let union = gu.union(&gv).count();
        if union == 0 {
            return not_well_defined("Query nodes have no neighbor in common");
        }
        Ok(gu.intersection(&gv).count() as f64 / union as f64)
    }
);

link_prediction!(
    /// `Σ 1 / ln(deg(z))` over the common neighbours `z`.
    AdamicAdarIndexLinkPrediction, |g, u, v| {
        let mut result = 0.0;
        for z in common_successors(g, u, v)? {
            let dz = g.out_degree_of(&z);
            if dz < 2 {
                return not_well_defined("Vertex has less than 2 degree");
            }
            result += 1.0 / (dz as f64).ln();
        }
        Ok(result)
    }
);

link_prediction!(
    /// `|N(u) ∩ N(v)|`.
    CommonNeighborsLinkPrediction, |g, u, v| {
        Ok(common_successors(g, u, v)?.len() as f64)
    }
);

link_prediction!(
    /// `|N(u) ∩ N(v)| / max(deg u, deg v)`.
    HubDepressedIndexLinkPrediction, |g, u, v| {
        let (du, dv) = (g.out_degree_of(u), g.out_degree_of(v));
        if du == 0 && dv == 0 {
            return not_well_defined("Both vertices have zero neighbors");
        }
        Ok(common_successors(g, u, v)?.len() as f64 / du.max(dv) as f64)
    }
);

link_prediction!(
    /// `|N(u) ∩ N(v)| / min(deg u, deg v)`.
    HubPromotedIndexLinkPrediction, |g, u, v| {
        let (du, dv) = (g.out_degree_of(u), g.out_degree_of(v));
        if du == 0 || dv == 0 {
            return not_well_defined("Query vertex with zero neighbors");
        }
        Ok(common_successors(g, u, v)?.len() as f64 / du.min(dv) as f64)
    }
);

link_prediction!(
    /// JGraphT computes `|N(u) ∩ N(v)| / deg u * deg v` (a product, not the textbook quotient);
    /// this port keeps that behaviour.
    LeichtHolmeNewmanIndexLinkPrediction, |g, u, v| {
        let (du, dv) = (g.out_degree_of(u), g.out_degree_of(v));
        if du == 0 || dv == 0 {
            return not_well_defined("Query vertex with zero neighbors");
        }
        Ok(common_successors(g, u, v)?.len() as f64 / du as f64 * dv as f64)
    }
);

link_prediction!(
    /// `deg u * deg v`.
    PreferentialAttachmentLinkPrediction, |g, u, v| {
        Ok((g.out_degree_of(u) * g.out_degree_of(v)) as f64)
    }
);

link_prediction!(
    /// `Σ 1 / deg(z)` over the common neighbours `z`.
    ResourceAllocationIndexLinkPrediction, |g, u, v| {
        let mut result = 0.0;
        for z in common_successors(g, u, v)? {
            let dz = g.out_degree_of(&z);
            if dz == 0 {
                return not_well_defined("Index not well defined");
            }
            result += 1.0 / dz as f64;
        }
        Ok(result)
    }
);

link_prediction!(
    /// `|N(u) ∩ N(v)| / sqrt(deg u * deg v)`.
    SaltonIndexLinkPrediction, |g, u, v| {
        let (du, dv) = (g.out_degree_of(u), g.out_degree_of(v));
        if du == 0 || dv == 0 {
            return not_well_defined("Query vertex with zero neighbors");
        }
        Ok(common_successors(g, u, v)?.len() as f64 / ((du * dv) as f64).sqrt())
    }
);

link_prediction!(
    /// `2 |N(u) ∩ N(v)| / (deg u + deg v)`.
    SorensenIndexLinkPrediction, |g, u, v| {
        let (du, dv) = (g.out_degree_of(u), g.out_degree_of(v));
        if du + dv == 0 {
            return not_well_defined("Both vertices have zero neighbors");
        }
        Ok(2.0 * common_successors(g, u, v)?.len() as f64 / (du + dv) as f64)
    }
);
