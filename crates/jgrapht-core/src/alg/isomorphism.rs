//! Graph and subgraph isomorphism (backtracking with degree and adjacency pruning, VF2 style).

use super::util::Indexed;
use crate::graph::Graph;
use crate::walk::GraphMapping;
use std::collections::HashMap;

struct Side<'a, G: Graph> {
    ix: Indexed<'a, G>,
    /// number of edges from a to b
    mult: HashMap<(usize, usize), usize>,
    outdeg: Vec<usize>,
    indeg: Vec<usize>,
}

impl<'a, G: Graph> Side<'a, G> {
    fn new(g: &'a G) -> Self {
        let ix = Indexed::new(g);
        let mut mult = HashMap::new();
        for &(a, b) in &ix.ends {
            *mult.entry((a, b)).or_insert(0) += 1;
            if !ix.directed && a != b {
                *mult.entry((b, a)).or_insert(0) += 1;
            }
        }
        let n = ix.n();
        let (mut outdeg, mut indeg) = (vec![0; n], vec![0; n]);
        for &(a, b) in &ix.ends {
            outdeg[a] += 1;
            indeg[b] += 1;
            if !ix.directed {
                outdeg[b] += 1;
                indeg[a] += 1;
            }
        }
        Side {
            ix,
            mult,
            outdeg,
            indeg,
        }
    }
    fn m(&self, a: usize, b: usize) -> usize {
        self.mult.get(&(a, b)).copied().unwrap_or(0)
    }
    fn neighbors(&self, v: usize) -> impl Iterator<Item = usize> + '_ {
        self.ix.out[v]
            .iter()
            .chain(&self.ix.inn[v])
            .map(|&(u, _)| u)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    /// bijection preserving edges exactly
    Isomorphism,
    /// pattern edges must exist in the target, but the target may have more (non-induced)
    Monomorphism,
    /// induced subgraph isomorphism: edges and non-edges preserved
    Induced,
}

fn search<G1: Graph, G2: Graph>(
    p: &Side<'_, G1>,
    t: &Side<'_, G2>,
    mode: Mode,
    visit: &mut dyn FnMut(&[usize]) -> bool,
) {
    let (np, nt) = (p.ix.n(), t.ix.n());
    if np > nt || (mode == Mode::Isomorphism && (np != nt || p.ix.es.len() != t.ix.es.len())) {
        return;
    }
    // match high-degree vertices of already-reached regions first
    let mut order: Vec<usize> = Vec::with_capacity(np);
    let mut placed = vec![false; np];
    while order.len() < np {
        let next = (0..np)
            .filter(|&v| !placed[v])
            .max_by_key(|&v| {
                (
                    order.iter().filter(|&&o| p.m(o, v) + p.m(v, o) > 0).count(),
                    p.outdeg[v] + p.indeg[v],
                )
            })
            .unwrap();
        placed[next] = true;
        order.push(next);
    }
    let mut map = vec![usize::MAX; np];
    let mut used = vec![false; nt];

    fn rec<G1: Graph, G2: Graph>(
        depth: usize,
        order: &[usize],
        p: &Side<'_, G1>,
        t: &Side<'_, G2>,
        mode: Mode,
        map: &mut Vec<usize>,
        used: &mut Vec<bool>,
        visit: &mut dyn FnMut(&[usize]) -> bool,
    ) -> bool {
        if depth == order.len() {
            return visit(map);
        }
        let v = order[depth];
        let anchor = order[..depth]
            .iter()
            .find(|&&o| p.m(o, v) + p.m(v, o) > 0)
            .copied();
        let cands: Vec<usize> = match anchor {
            Some(a) => {
                let mut c: Vec<usize> = t.neighbors(map[a]).collect();
                c.sort_unstable();
                c.dedup();
                c
            }
            None => (0..t.ix.n()).collect(),
        };
        for c in cands {
            if used[c] {
                continue;
            }
            let degree_ok = match mode {
                Mode::Isomorphism => p.outdeg[v] == t.outdeg[c] && p.indeg[v] == t.indeg[c],
                _ => p.outdeg[v] <= t.outdeg[c] && p.indeg[v] <= t.indeg[c],
            };
            let rel = |a: usize, b: usize| -> bool {
                match mode {
                    Mode::Monomorphism => {
                        p.m(a, b) <= t.m(map_of(map, a, v, c), map_of(map, b, v, c))
                    }
                    _ => p.m(a, b) == t.m(map_of(map, a, v, c), map_of(map, b, v, c)),
                }
            };
            fn map_of(map: &[usize], x: usize, v: usize, c: usize) -> usize {
                if x == v { c } else { map[x] }
            }
            if !degree_ok || !rel(v, v) {
                continue;
            }
            if order[..depth].iter().all(|&o| rel(o, v) && rel(v, o)) {
                map[v] = c;
                used[c] = true;
                let stop = rec(depth + 1, order, p, t, mode, map, used, visit);
                used[c] = false;
                map[v] = usize::MAX;
                if stop {
                    return true;
                }
            }
        }
        false
    }
    rec(0, &order, p, t, mode, &mut map, &mut used, visit);
}

fn to_mapping<G1: Graph, G2: Graph>(
    p: &Side<'_, G1>,
    t: &Side<'_, G2>,
    map: &[usize],
) -> GraphMapping<G1::Vertex, G1::Edge, G2::Vertex, G2::Edge> {
    let mut gm = GraphMapping::new();
    for (v, &c) in map.iter().enumerate() {
        gm.map_vertex(p.ix.vertex(v), t.ix.vertex(c));
    }
    let mut taken = vec![false; t.ix.es.len()];
    for (k, &(a, b)) in p.ix.ends.iter().enumerate() {
        let (ta, tb) = (map[a], map[b]);
        let target = (0..t.ix.es.len()).find(|&j| {
            !taken[j] && {
                let (x, y) = t.ix.ends[j];
                (x, y) == (ta, tb) || (!t.ix.directed && (y, x) == (ta, tb))
            }
        });
        if let Some(j) = target {
            taken[j] = true;
            gm.map_edge(p.ix.es[k].clone(), t.ix.es[j].clone());
        }
    }
    gm
}

fn first<G1: Graph, G2: Graph>(
    g1: &G1,
    g2: &G2,
    mode: Mode,
) -> Option<GraphMapping<G1::Vertex, G1::Edge, G2::Vertex, G2::Edge>> {
    if g1.graph_type().directed != g2.graph_type().directed {
        return None;
    }
    let (p, t) = (Side::new(g1), Side::new(g2));
    let mut found = None;
    search(&p, &t, mode, &mut |m| {
        found = Some(m.to_vec());
        true
    });
    found.map(|m| to_mapping(&p, &t, &m))
}

fn count<G1: Graph, G2: Graph>(g1: &G1, g2: &G2, mode: Mode) -> usize {
    if g1.graph_type().directed != g2.graph_type().directed {
        return 0;
    }
    let (p, t) = (Side::new(g1), Side::new(g2));
    let mut n = 0;
    search(&p, &t, mode, &mut |_| {
        n += 1;
        false
    });
    n
}

/// An isomorphism from `g1` to `g2` (vertex and edge mapping), if one exists.
pub fn find_isomorphism<G1: Graph, G2: Graph>(
    g1: &G1,
    g2: &G2,
) -> Option<GraphMapping<G1::Vertex, G1::Edge, G2::Vertex, G2::Edge>> {
    first(g1, g2, Mode::Isomorphism)
}

pub fn is_isomorphic<G1: Graph, G2: Graph>(g1: &G1, g2: &G2) -> bool {
    find_isomorphism(g1, g2).is_some()
}

/// Number of distinct isomorphisms (= automorphisms when `g1` and `g2` are the same graph).
pub fn count_isomorphisms<G1: Graph, G2: Graph>(g1: &G1, g2: &G2) -> usize {
    count(g1, g2, Mode::Isomorphism)
}

/// Embeds `pattern` into `target` as an *induced* subgraph.
pub fn find_induced_subgraph<P: Graph, T: Graph>(
    pattern: &P,
    target: &T,
) -> Option<GraphMapping<P::Vertex, P::Edge, T::Vertex, T::Edge>> {
    first(pattern, target, Mode::Induced)
}

/// Embeds `pattern` into `target` allowing extra edges in the target (subgraph monomorphism).
pub fn find_subgraph<P: Graph, T: Graph>(
    pattern: &P,
    target: &T,
) -> Option<GraphMapping<P::Vertex, P::Edge, T::Vertex, T::Edge>> {
    first(pattern, target, Mode::Monomorphism)
}

/// Number of induced embeddings of `pattern` in `target`.
pub fn count_induced_subgraphs<P: Graph, T: Graph>(pattern: &P, target: &T) -> usize {
    count(pattern, target, Mode::Induced)
}

/// Number of (non-induced) embeddings of `pattern` in `target`.
pub fn count_subgraphs<P: Graph, T: Graph>(pattern: &P, target: &T) -> usize {
    count(pattern, target, Mode::Monomorphism)
}
