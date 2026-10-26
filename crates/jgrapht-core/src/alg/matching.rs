//! Matchings: bipartite (Hopcroft–Karp, Hungarian), general (Edmonds' blossom), greedy.

use super::partition::two_color;
use super::util::Indexed;
use crate::graph::Graph;
use crate::{GraphError, Result};
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub struct Matching<E> {
    pub edges: Vec<E>,
    pub weight: f64,
}

impl<E> Matching<E> {
    pub fn len(&self) -> usize {
        self.edges.len()
    }
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

/// The heaviest edge between two vertex indices.
fn edge_between<G: Graph>(ix: &Indexed<'_, G>, a: usize, b: usize) -> usize {
    ix.out[a]
        .iter()
        .chain(&ix.inn[a])
        .filter(|&&(u, _)| u == b)
        .map(|&(_, k)| k)
        .max_by(|&x, &y| ix.w[x].total_cmp(&ix.w[y]))
        .expect("matched vertices are adjacent")
}

fn matching_of<G: Graph>(ix: &Indexed<'_, G>, mate: &[Option<usize>]) -> Matching<G::Edge> {
    let mut edges = Vec::new();
    let mut weight = 0.0;
    for a in 0..ix.n() {
        if let Some(b) = mate[a] {
            if a < b {
                let k = edge_between(ix, a, b);
                edges.push(ix.es[k].clone());
                weight += ix.w[k];
            }
        }
    }
    Matching { edges, weight }
}

fn simple_adj<G: Graph>(ix: &Indexed<'_, G>) -> Vec<Vec<usize>> {
    (0..ix.n())
        .map(|v| {
            let mut seen = HashSet::new();
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v && seen.insert(u))
                .collect()
        })
        .collect()
}

/// Hopcroft–Karp on index adjacency; returns the mate of every vertex.
pub(crate) fn hopcroft_karp_mates(adj: &[Vec<usize>], left: &[usize]) -> Vec<Option<usize>> {
    let n = adj.len();
    let mut mate: Vec<Option<usize>> = vec![None; n];
    fn dfs(
        v: usize,
        adj: &[Vec<usize>],
        mate: &mut [Option<usize>],
        dist: &mut [usize],
        it: &mut [usize],
    ) -> bool {
        while it[v] < adj[v].len() {
            let u = adj[v][it[v]];
            it[v] += 1;
            let next = mate[u];
            if next.is_none_or(|w| dist[w] == dist[v] + 1 && dfs(w, adj, mate, dist, it)) {
                mate[v] = Some(u);
                mate[u] = Some(v);
                return true;
            }
        }
        dist[v] = usize::MAX;
        false
    }
    loop {
        let mut dist = vec![usize::MAX; n];
        let mut q = VecDeque::new();
        for &v in left {
            if mate[v].is_none() {
                dist[v] = 0;
                q.push_back(v);
            }
        }
        let mut found = false;
        while let Some(v) = q.pop_front() {
            for &u in &adj[v] {
                match mate[u] {
                    None => found = true,
                    Some(w) if dist[w] == usize::MAX => {
                        dist[w] = dist[v] + 1;
                        q.push_back(w);
                    }
                    _ => {}
                }
            }
        }
        if !found {
            return mate;
        }
        let mut it = vec![0; n];
        let mut progress = false;
        for &v in left {
            if mate[v].is_none() && dfs(v, adj, &mut mate, &mut dist, &mut it) {
                progress = true;
            }
        }
        if !progress {
            return mate;
        }
    }
}

/// Maximum cardinality matching of a bipartite graph (Hopcroft–Karp).
pub fn hopcroft_karp<G: Graph>(g: &G) -> Result<Matching<G::Edge>> {
    let ix = Indexed::new(g);
    let side = two_color(&ix)
        .ok_or_else(|| GraphError::InvalidArgument("graph is not bipartite".into()))?;
    let left: Vec<usize> = (0..ix.n()).filter(|&v| !side[v]).collect();
    let mate = hopcroft_karp_mates(&simple_adj(&ix), &left);
    Ok(matching_of(&ix, &mate))
}

/// Maximum weight matching of a bipartite graph (Hungarian algorithm, `O(n^3)`).
/// Edges of non-positive weight are never used.
pub fn maximum_weight_bipartite_matching<G: Graph>(g: &G) -> Result<Matching<G::Edge>> {
    let ix = Indexed::new(g);
    let side = two_color(&ix)
        .ok_or_else(|| GraphError::InvalidArgument("graph is not bipartite".into()))?;
    let left: Vec<usize> = (0..ix.n()).filter(|&v| !side[v]).collect();
    let right: Vec<usize> = (0..ix.n()).filter(|&v| side[v]).collect();
    let n = left.len().max(right.len());
    if n == 0 {
        return Ok(Matching {
            edges: vec![],
            weight: 0.0,
        });
    }
    let (lpos, rpos): (Vec<_>, Vec<_>) = (0..ix.n())
        .map(|v| {
            (
                left.iter().position(|&x| x == v),
                right.iter().position(|&x| x == v),
            )
        })
        .unzip();
    let mut cost = vec![vec![0.0f64; n + 1]; n + 1];
    for (k, &(a, b)) in ix.ends.iter().enumerate() {
        let (i, j) = match (lpos[a], rpos[b], lpos[b], rpos[a]) {
            (Some(i), Some(j), _, _) => (i, j),
            (_, _, Some(i), Some(j)) => (i, j),
            _ => continue,
        };
        if ix.w[k] > 0.0 {
            cost[i + 1][j + 1] = cost[i + 1][j + 1].min(-ix.w[k]);
        }
    }
    let (mut u, mut v) = (vec![0.0; n + 1], vec![0.0; n + 1]);
    let (mut p, mut way) = (vec![0usize; n + 1], vec![0usize; n + 1]);
    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0;
        let mut minv = vec![f64::INFINITY; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            used[j0] = true;
            let (i0, mut delta, mut j1) = (p[j0], f64::INFINITY, 0);
            for j in 1..=n {
                if !used[j] {
                    let cur = cost[i0][j] - u[i0] - v[j];
                    if cur < minv[j] {
                        minv[j] = cur;
                        way[j] = j0;
                    }
                    if minv[j] < delta {
                        delta = minv[j];
                        j1 = j;
                    }
                }
            }
            for j in 0..=n {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut mate = vec![None; ix.n()];
    for j in 1..=n {
        let i = p[j];
        if i >= 1 && i <= left.len() && j <= right.len() && cost[i][j] < 0.0 {
            mate[left[i - 1]] = Some(right[j - 1]);
            mate[right[j - 1]] = Some(left[i - 1]);
        }
    }
    Ok(matching_of(&ix, &mate))
}

/// Maximum cardinality matching of a general graph (Edmonds' blossom algorithm, `O(V^3)`).
pub fn edmonds_blossom<G: Graph>(g: &G) -> Matching<G::Edge> {
    let ix = Indexed::new(g);
    let adj = simple_adj(&ix);
    let n = ix.n();
    let mut mate = vec![usize::MAX; n];

    let lca = |a: usize, b: usize, base: &[usize], mate: &[usize], p: &[usize]| {
        let mut used = vec![false; n];
        let mut a = a;
        loop {
            a = base[a];
            used[a] = true;
            if mate[a] == usize::MAX {
                break;
            }
            a = p[mate[a]];
        }
        let mut b = b;
        loop {
            b = base[b];
            if used[b] {
                return b;
            }
            b = p[mate[b]];
        }
    };

    for root in 0..n {
        if mate[root] != usize::MAX {
            continue;
        }
        let (mut used, mut p, mut base) = (
            vec![false; n],
            vec![usize::MAX; n],
            (0..n).collect::<Vec<_>>(),
        );
        used[root] = true;
        let mut q = VecDeque::from([root]);
        let mut end = usize::MAX;
        'search: while let Some(v) = q.pop_front() {
            for &to in &adj[v] {
                if base[v] == base[to] || mate[v] == to {
                    continue;
                }
                if to == root || (mate[to] != usize::MAX && p[mate[to]] != usize::MAX) {
                    let cur = lca(v, to, &base, &mate, &p);
                    let mut blossom = vec![false; n];
                    for (start, child) in [(v, to), (to, v)] {
                        let (mut x, mut ch) = (start, child);
                        while base[x] != cur {
                            blossom[base[x]] = true;
                            blossom[base[mate[x]]] = true;
                            p[x] = ch;
                            ch = mate[x];
                            x = p[mate[x]];
                        }
                    }
                    for i in 0..n {
                        if blossom[base[i]] {
                            base[i] = cur;
                            if !used[i] {
                                used[i] = true;
                                q.push_back(i);
                            }
                        }
                    }
                } else if p[to] == usize::MAX {
                    p[to] = v;
                    if mate[to] == usize::MAX {
                        end = to;
                        break 'search;
                    }
                    used[mate[to]] = true;
                    q.push_back(mate[to]);
                }
            }
        }
        let mut v = end;
        while v != usize::MAX {
            let pv = p[v];
            let ppv = mate[pv];
            mate[v] = pv;
            mate[pv] = v;
            v = ppv;
        }
    }
    let mates: Vec<_> = mate
        .iter()
        .map(|&m| (m != usize::MAX).then_some(m))
        .collect();
    matching_of(&ix, &mates)
}

/// Greedy matching by decreasing weight: a 1/2-approximation of the maximum weight matching.
pub fn greedy_matching<G: Graph>(g: &G) -> Matching<G::Edge> {
    let ix = Indexed::new(g);
    let mut order: Vec<usize> = (0..ix.es.len()).collect();
    order.sort_by(|&a, &b| ix.w[b].total_cmp(&ix.w[a]));
    let mut mate = vec![None; ix.n()];
    for k in order {
        let (a, b) = ix.ends[k];
        if a != b && mate[a].is_none() && mate[b].is_none() {
            mate[a] = Some(b);
            mate[b] = Some(a);
        }
    }
    matching_of(&ix, &mate)
}

/// Whether the edges form a matching in `g`.
pub fn is_matching<G: Graph>(g: &G, edges: &[G::Edge]) -> bool {
    let mut used = HashSet::new();
    edges
        .iter()
        .all(|e| match g.edge_source(e).zip(g.edge_target(e)) {
            Some((s, t)) => s != t && used.insert(s) && used.insert(t),
            None => false,
        })
}
