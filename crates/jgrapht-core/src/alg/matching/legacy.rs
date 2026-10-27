//! Matchings: bipartite (Hopcroft–Karp, Hungarian), general (Edmonds' blossom), greedy.

use crate::alg::partition::two_color;
use crate::alg::util::Indexed;
use crate::graph::Graph;
use crate::{GraphError, Result};
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyMatching<E> {
    pub edges: Vec<E>,
    pub weight: f64,
}

impl<E> LegacyMatching<E> {
    pub fn len(&self) -> usize {
        self.edges.len()
    }
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
    /// Whether every vertex of `g` is matched.
    pub fn is_perfect<G: Graph<Edge = E>>(&self, g: &G) -> bool {
        2 * self.edges.len() == g.vertex_count()
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

fn matching_of<G: Graph>(ix: &Indexed<'_, G>, mate: &[Option<usize>]) -> LegacyMatching<G::Edge> {
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
    LegacyMatching { edges, weight }
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
pub fn hopcroft_karp<G: Graph>(g: &G) -> Result<LegacyMatching<G::Edge>> {
    let ix = Indexed::new(g);
    let side = two_color(&ix)
        .ok_or_else(|| GraphError::InvalidArgument("graph is not bipartite".into()))?;
    let left: Vec<usize> = (0..ix.n()).filter(|&v| !side[v]).collect();
    let mate = hopcroft_karp_mates(&simple_adj(&ix), &left);
    Ok(matching_of(&ix, &mate))
}

/// Maximum cardinality matching of a bipartite graph whose two sides are given
/// (`HopcroftKarpMaximumCardinalityBipartiteMatching(graph, partition1, partition2)`).
pub fn hopcroft_karp_with_partitions<G: Graph>(
    g: &G,
    partition1: &HashSet<G::Vertex>,
    partition2: &HashSet<G::Vertex>,
) -> Result<LegacyMatching<G::Edge>> {
    if partition1.len() + partition2.len() != g.vertex_count()
        || g.vertices()
            .any(|v| !partition1.contains(v) && !partition2.contains(v))
        || partition1.iter().any(|v| partition2.contains(v))
    {
        return Err(GraphError::InvalidArgument(
            "the partitions do not partition the vertices".into(),
        ));
    }
    let ix = Indexed::new(g);
    let adj = simple_adj(&ix);
    for (v, nbrs) in adj.iter().enumerate() {
        let side = partition1.contains(ix.vs[v]);
        if nbrs.iter().any(|&u| partition1.contains(ix.vs[u]) == side) {
            return Err(GraphError::InvalidArgument(
                "the partitions are not a bipartition of the graph".into(),
            ));
        }
    }
    let left: Vec<usize> = (0..ix.n())
        .filter(|&v| partition1.contains(ix.vs[v]))
        .collect();
    let mate = hopcroft_karp_mates(&adj, &left);
    Ok(matching_of(&ix, &mate))
}

/// Maximum weight matching of a bipartite graph (Hungarian algorithm, `O(n^3)`).
/// Edges of non-positive weight are never used.
pub fn maximum_weight_bipartite_matching<G: Graph>(g: &G) -> Result<LegacyMatching<G::Edge>> {
    let ix = Indexed::new(g);
    let side = two_color(&ix)
        .ok_or_else(|| GraphError::InvalidArgument("graph is not bipartite".into()))?;
    let left: Vec<usize> = (0..ix.n()).filter(|&v| !side[v]).collect();
    let right: Vec<usize> = (0..ix.n()).filter(|&v| side[v]).collect();
    let n = left.len().max(right.len());
    if n == 0 {
        return Ok(LegacyMatching {
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
pub fn edmonds_blossom<G: Graph>(g: &G) -> LegacyMatching<G::Edge> {
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
pub fn greedy_matching<G: Graph>(g: &G) -> LegacyMatching<G::Edge> {
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

/// Path-growing 1/2-approximation of maximum weight matching (Drake & Hougardy), linear time.
pub fn path_growing_matching<G: Graph>(g: &G) -> LegacyMatching<G::Edge> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut removed = vec![false; n];
    let (mut m1, mut m2): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
    let (mut w1, mut w2) = (0.0, 0.0);
    for start in 0..n {
        if removed[start] {
            continue;
        }
        let (mut x, mut to_first) = (start, true);
        loop {
            let heaviest = ix.out[x]
                .iter()
                .chain(&ix.inn[x])
                .filter(|&&(u, _)| u != x && !removed[u])
                .max_by(|a, b| ix.w[a.1].total_cmp(&ix.w[b.1]))
                .copied();
            removed[x] = true;
            let Some((y, k)) = heaviest else { break };
            if to_first {
                m1.push(k);
                w1 += ix.w[k];
            } else {
                m2.push(k);
                w2 += ix.w[k];
            }
            to_first = !to_first;
            x = y;
        }
    }
    // each list is a union of paths: extract a maximum-weight matching from the heavier one
    let chosen = if w1 >= w2 { m1 } else { m2 };
    let mut taken = vec![false; n];
    let mut by_weight = chosen;
    by_weight.sort_by(|&a, &b| ix.w[b].total_cmp(&ix.w[a]));
    let mut mate = vec![None; n];
    for k in by_weight {
        let (a, b) = ix.ends[k];
        if !taken[a] && !taken[b] {
            taken[a] = true;
            taken[b] = true;
            mate[a] = Some(b);
            mate[b] = Some(a);
        }
    }
    matching_of(&ix, &mate)
}

/// Exact maximum weight matching by dynamic programming over vertex subsets, `O(2^n n)`.
/// Limited to 24 vertices; meant for small instances and as a test oracle.
pub fn maximum_weight_matching_exact<G: Graph>(g: &G) -> Result<LegacyMatching<G::Edge>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    if n > 24 {
        return Err(GraphError::InvalidArgument(
            "exact matching is limited to 24 vertices".into(),
        ));
    }
    let mut best = vec![vec![f64::NEG_INFINITY; n]; n];
    for (k, &(a, b)) in ix.ends.iter().enumerate() {
        if a != b && ix.w[k] > best[a][b] {
            best[a][b] = ix.w[k];
            best[b][a] = ix.w[k];
        }
    }
    let full = 1usize << n;
    // value[mask] = best matching weight using only vertices in mask; choice for reconstruction
    let mut value = vec![0.0f64; full];
    let mut choice = vec![usize::MAX; full];
    for mask in 1..full {
        let v = mask.trailing_zeros() as usize;
        let rest = mask & (mask - 1);
        value[mask] = value[rest]; // leave v unmatched
        let mut others = rest;
        while others != 0 {
            let u = others.trailing_zeros() as usize;
            others &= others - 1;
            if best[v][u] > 0.0 {
                let cand = best[v][u] + value[rest & !(1 << u)];
                if cand > value[mask] {
                    value[mask] = cand;
                    choice[mask] = u;
                }
            }
        }
    }
    let mut mate = vec![None; n];
    let mut mask = full - 1;
    while mask != 0 {
        let v = mask.trailing_zeros() as usize;
        let rest = mask & (mask - 1);
        if choice[mask] == usize::MAX {
            mask = rest;
        } else {
            let u = choice[mask];
            mate[v] = Some(u);
            mate[u] = Some(v);
            mask = rest & !(1 << u);
        }
    }
    Ok(matching_of(&ix, &mate))
}
