//! Vertex colouring heuristics.

use super::util::Indexed;
use crate::graph::Graph;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Coloring<V> {
    pub colors: HashMap<V, usize>,
    pub count: usize,
}

fn neighbor_lists<G: Graph>(ix: &Indexed<'_, G>) -> Vec<Vec<usize>> {
    (0..ix.n())
        .map(|v| {
            let mut s = HashSet::new();
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v && s.insert(u))
                .collect()
        })
        .collect()
}

fn greedy_in_order<G: Graph>(
    ix: &Indexed<'_, G>,
    nb: &[Vec<usize>],
    order: &[usize],
) -> Coloring<G::Vertex> {
    let mut color = vec![usize::MAX; ix.n()];
    let mut count = 0;
    for &v in order {
        let used: HashSet<usize> = nb[v]
            .iter()
            .map(|&u| color[u])
            .filter(|&c| c != usize::MAX)
            .collect();
        let c = (0..).find(|c| !used.contains(c)).unwrap();
        color[v] = c;
        count = count.max(c + 1);
    }
    Coloring {
        colors: (0..ix.n()).map(|v| (ix.vertex(v), color[v])).collect(),
        count,
    }
}

/// Greedy colouring in vertex iteration order.
pub fn greedy_coloring<G: Graph>(g: &G) -> Coloring<G::Vertex> {
    let ix = Indexed::new(g);
    let nb = neighbor_lists(&ix);
    greedy_in_order(&ix, &nb, &(0..ix.n()).collect::<Vec<_>>())
}

/// Welsh–Powell: greedy in order of decreasing degree.
pub fn largest_degree_first_coloring<G: Graph>(g: &G) -> Coloring<G::Vertex> {
    let ix = Indexed::new(g);
    let nb = neighbor_lists(&ix);
    let mut order: Vec<usize> = (0..ix.n()).collect();
    order.sort_by_key(|&v| std::cmp::Reverse(nb[v].len()));
    greedy_in_order(&ix, &nb, &order)
}

/// Matula–Beck: greedy in reverse of the repeated-minimum-degree removal order.
pub fn smallest_degree_last_coloring<G: Graph>(g: &G) -> Coloring<G::Vertex> {
    let ix = Indexed::new(g);
    let nb = neighbor_lists(&ix);
    let mut deg: Vec<usize> = nb.iter().map(Vec::len).collect();
    let mut removed = vec![false; ix.n()];
    let mut order = Vec::with_capacity(ix.n());
    for _ in 0..ix.n() {
        let v = (0..ix.n())
            .filter(|&v| !removed[v])
            .min_by_key(|&v| deg[v])
            .unwrap();
        removed[v] = true;
        order.push(v);
        nb[v]
            .iter()
            .for_each(|&u| deg[u] = deg[u].saturating_sub(1));
    }
    order.reverse();
    greedy_in_order(&ix, &nb, &order)
}

/// Brélaz' DSATUR heuristic: colour the vertex with most distinct neighbour colours next.
pub fn saturation_degree_coloring<G: Graph>(g: &G) -> Coloring<G::Vertex> {
    let ix = Indexed::new(g);
    let nb = neighbor_lists(&ix);
    let n = ix.n();
    let mut color = vec![usize::MAX; n];
    let mut sat: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    let mut count = 0;
    for _ in 0..n {
        let v = (0..n)
            .filter(|&v| color[v] == usize::MAX)
            .max_by_key(|&v| (sat[v].len(), nb[v].len(), std::cmp::Reverse(v)))
            .unwrap();
        let c = (0..).find(|c| !sat[v].contains(c)).unwrap();
        color[v] = c;
        count = count.max(c + 1);
        nb[v].iter().for_each(|&u| {
            sat[u].insert(c);
        });
    }
    Coloring {
        colors: (0..n).map(|v| (ix.vertex(v), color[v])).collect(),
        count,
    }
}

/// Optimal colouring of a chordal graph, `None` if the graph is not chordal.
pub fn chordal_coloring<G: Graph>(g: &G) -> Option<Coloring<G::Vertex>> {
    let peo = super::chordal::perfect_elimination_order(g)?;
    let ix = Indexed::new(g);
    let nb = neighbor_lists(&ix);
    let order: Vec<usize> = peo.iter().rev().map(|v| ix.idx[v]).collect();
    Some(greedy_in_order(&ix, &nb, &order))
}

/// Whether `colors` assigns a colour to every vertex with no monochromatic edge.
pub fn is_proper_coloring<G: Graph>(g: &G, colors: &HashMap<G::Vertex, usize>) -> bool {
    g.vertices().all(|v| colors.contains_key(v))
        && g.edges().all(|e| {
            let (s, t) = (g.edge_source(e).unwrap(), g.edge_target(e).unwrap());
            s == t || colors[s] != colors[t]
        })
}

/// 1-dimensional Weisfeiler–Leman colour refinement. Returns the stable colouring; vertices
/// with different colours cannot be mapped to each other by an isomorphism.
pub fn color_refinement<G: Graph>(g: &G) -> HashMap<G::Vertex, usize> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut color = vec![0usize; n];
    let mut classes = 1;
    loop {
        let sigs: Vec<(usize, Vec<usize>, Vec<usize>)> = (0..n)
            .map(|v| {
                let mut o: Vec<usize> = ix.out[v].iter().map(|&(u, _)| color[u]).collect();
                let mut i: Vec<usize> = if ix.directed {
                    ix.inn[v].iter().map(|&(u, _)| color[u]).collect()
                } else {
                    vec![]
                };
                o.sort_unstable();
                i.sort_unstable();
                (color[v], o, i)
            })
            .collect();
        let mut ids: HashMap<&(usize, Vec<usize>, Vec<usize>), usize> = HashMap::new();
        let next: Vec<usize> = sigs
            .iter()
            .map(|s| {
                let l = ids.len();
                *ids.entry(s).or_insert(l)
            })
            .collect();
        let new_classes = ids.len();
        color = next;
        if new_classes == classes {
            break;
        }
        classes = new_classes;
    }
    (0..n).map(|v| (ix.vertex(v), color[v])).collect()
}
