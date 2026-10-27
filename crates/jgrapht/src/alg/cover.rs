//! Vertex cover.

use super::matching::hopcroft_karp_mates;
use super::partition::two_color;
use super::util::Indexed;
use crate::graph::Graph;
use crate::{GraphError, Result};
use std::collections::{HashSet, VecDeque};

/// A vertex cover with its total vertex weight.
#[derive(Debug, Clone)]
pub struct VertexCover<V> {
    pub vertices: Vec<V>,
    pub weight: f64,
}

fn finish<G: Graph>(
    ix: &Indexed<'_, G>,
    cover: &[bool],
    weight: &dyn Fn(&G::Vertex) -> f64,
) -> VertexCover<G::Vertex> {
    let vertices: Vec<_> = (0..ix.n())
        .filter(|&v| cover[v])
        .map(|v| ix.vertex(v))
        .collect();
    let weight = vertices.iter().map(weight).sum();
    VertexCover { vertices, weight }
}

/// Greedy: repeatedly take the vertex covering the most uncovered edges.
pub fn greedy_vertex_cover<G: Graph>(g: &G) -> VertexCover<G::Vertex> {
    let ix = Indexed::new(g);
    let mut covered = vec![false; ix.es.len()];
    let mut cover = vec![false; ix.n()];
    loop {
        let best = (0..ix.n())
            .map(|v| {
                (
                    v,
                    ix.out[v]
                        .iter()
                        .chain(if ix.directed { &ix.inn[v][..] } else { &[] })
                        .filter(|&&(_, k)| !covered[k])
                        .count(),
                )
            })
            .max_by_key(|&(_, c)| c);
        match best {
            Some((v, c)) if c > 0 => {
                cover[v] = true;
                ix.out[v]
                    .iter()
                    .chain(&ix.inn[v])
                    .for_each(|&(_, k)| covered[k] = true);
            }
            _ => break,
        }
    }
    finish(&ix, &cover, &|_| 1.0)
}

/// Classic 2-approximation: take both endpoints of a maximal matching.
pub fn two_approximation_vertex_cover<G: Graph>(g: &G) -> VertexCover<G::Vertex> {
    let ix = Indexed::new(g);
    let mut cover = vec![false; ix.n()];
    for &(a, b) in &ix.ends {
        if !cover[a] && !cover[b] {
            cover[a] = true;
            cover[b] = true;
        }
    }
    finish(&ix, &cover, &|_| 1.0)
}

/// Bar-Yehuda–Even local-ratio 2-approximation for weighted vertex cover.
pub fn bar_yehuda_even_vertex_cover<G: Graph>(
    g: &G,
    weight: impl Fn(&G::Vertex) -> f64,
) -> VertexCover<G::Vertex> {
    let ix = Indexed::new(g);
    let mut residual: Vec<f64> = (0..ix.n()).map(|v| weight(ix.vs[v])).collect();
    for &(a, b) in &ix.ends {
        let d = residual[a].min(residual[b]);
        residual[a] -= d;
        residual[b] -= d;
    }
    let cover: Vec<bool> = residual.iter().map(|&r| r <= 1e-12).collect();
    finish(&ix, &cover, &weight)
}

/// Clarkson's greedy 2-approximation for weighted vertex cover (best weight/degree ratio).
pub fn clarkson_vertex_cover<G: Graph>(
    g: &G,
    weight: impl Fn(&G::Vertex) -> f64,
) -> VertexCover<G::Vertex> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut w: Vec<f64> = (0..n).map(|v| weight(ix.vs[v])).collect();
    let mut alive: Vec<HashSet<usize>> = (0..n)
        .map(|v| {
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v)
                .collect()
        })
        .collect();
    let mut cover = vec![false; n];
    loop {
        let best = (0..n).filter(|&v| !alive[v].is_empty()).min_by(|&a, &b| {
            (w[a] / alive[a].len() as f64).total_cmp(&(w[b] / alive[b].len() as f64))
        });
        let Some(v) = best else { break };
        let ratio = w[v] / alive[v].len() as f64;
        let nbs: Vec<usize> = alive[v].iter().copied().collect();
        for u in nbs {
            w[u] -= ratio;
            alive[u].remove(&v);
        }
        cover[v] = true;
        alive[v].clear();
        w[v] = 0.0;
    }
    finish(&ix, &cover, &weight)
}

/// Exact minimum vertex cover of a bipartite graph (König's theorem).
pub fn bipartite_vertex_cover<G: Graph>(g: &G) -> Result<VertexCover<G::Vertex>> {
    let ix = Indexed::new(g);
    let side = two_color(&ix)
        .ok_or_else(|| GraphError::InvalidArgument("graph is not bipartite".into()))?;
    let adj: Vec<Vec<usize>> = (0..ix.n())
        .map(|v| {
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v)
                .collect()
        })
        .collect();
    let left: Vec<usize> = (0..ix.n()).filter(|&v| !side[v]).collect();
    let mate = hopcroft_karp_mates(&adj, &left);
    // vertices reachable from unmatched left vertices by alternating paths
    let mut z = vec![false; ix.n()];
    let mut q: VecDeque<usize> = left
        .iter()
        .copied()
        .filter(|&v| mate[v].is_none())
        .collect();
    q.iter().for_each(|&v| z[v] = true);
    while let Some(v) = q.pop_front() {
        if !side[v] {
            for &u in &adj[v] {
                if mate[v] != Some(u) && !z[u] {
                    z[u] = true;
                    q.push_back(u);
                }
            }
        } else if let Some(u) = mate[v] {
            if !z[u] {
                z[u] = true;
                q.push_back(u);
            }
        }
    }
    let cover: Vec<bool> = (0..ix.n())
        .map(|v| if side[v] { z[v] } else { !z[v] })
        .collect();
    Ok(finish(&ix, &cover, &|_| 1.0))
}

/// Whether every edge has an endpoint in `vs`.
pub fn is_vertex_cover<G: Graph>(g: &G, vs: &[G::Vertex]) -> bool {
    let set: HashSet<_> = vs.iter().collect();
    g.edges()
        .all(|e| set.contains(g.edge_source(e).unwrap()) || set.contains(g.edge_target(e).unwrap()))
}

// ---------------------------------------------------------------------------------------------
// JGraphT's vertex cover classes. All of them need an undirected graph; parallel edges and
// self-loops are accepted. Weights default to 1 per vertex.

use super::interfaces::VertexCoverAlgorithm;
use crate::graph::{GraphExt, props};
use crate::util::OrderedF64;
use indexmap::IndexMap;
use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;

fn unit_weights<G: Graph>(g: &G) -> HashMap<G::Vertex, f64> {
    g.vertices().map(|v| (v.clone(), 1.0)).collect()
}

fn weight_of<V: Eq + Hash>(weights: &HashMap<V, f64>, v: &V) -> Result<f64> {
    weights
        .get(v)
        .copied()
        .ok_or_else(|| GraphError::InvalidArgument("missing vertex weight".into()))
}

/// Shared engine of the greedy and Clarkson covers: repeatedly take the vertex of smallest
/// weight / degree ratio (ties by creation order).
fn ratio_cover<G: Graph>(
    g: &G,
    weights: &HashMap<G::Vertex, f64>,
    clarkson: bool,
) -> Result<VertexCover<G::Vertex>> {
    struct RatioVertex<V> {
        v: V,
        weight: f64,
        degree: usize,
        neighbors: IndexMap<usize, usize>,
    }
    impl<V> RatioVertex<V> {
        fn ratio(&self) -> f64 {
            self.weight / self.degree as f64
        }
    }
    let mut ids: HashMap<&G::Vertex, usize> = HashMap::new();
    let mut rv: Vec<RatioVertex<G::Vertex>> = Vec::new();
    for v in g.vertices() {
        if g.degree_of(v) > 0 {
            ids.insert(v, rv.len());
            rv.push(RatioVertex {
                v: v.clone(),
                weight: weight_of(weights, v)?,
                degree: 0,
                neighbors: IndexMap::new(),
            });
        }
    }
    for e in g.edges() {
        let (a, b) = (
            ids[g.edge_source(e).unwrap()],
            ids[g.edge_target(e).unwrap()],
        );
        for (x, y) in [(a, b), (b, a)] {
            *rv[x].neighbors.entry(y).or_insert(0) += 1;
            rv[x].degree += 1;
        }
    }
    let key = |r: &RatioVertex<G::Vertex>, id: usize| (OrderedF64(r.ratio()), id);
    let mut working: BTreeSet<(OrderedF64, usize)> =
        (0..rv.len()).map(|i| key(&rv[i], i)).collect();
    let mut cover = Vec::new();
    let mut weight = 0.0;
    while let Some(first) = working.pop_first() {
        let vx = first.1;
        let ratio = rv[vx].ratio();
        let neighbors: Vec<(usize, usize)> =
            rv[vx].neighbors.iter().map(|(&n, &c)| (n, c)).collect();
        for (nx, count) in neighbors {
            if nx == vx {
                continue;
            }
            working.remove(&key(&rv[nx], nx));
            if clarkson {
                rv[nx].weight -= ratio * count as f64;
            }
            let c = rv[nx].neighbors.shift_remove(&vx).unwrap_or(0);
            rv[nx].degree -= c;
            if rv[nx].degree > 0 {
                working.insert(key(&rv[nx], nx));
            }
        }
        weight += weight_of(weights, &rv[vx].v)?;
        cover.push(rv[vx].v.clone());
    }
    Ok(VertexCover {
        vertices: cover,
        weight,
    })
}

macro_rules! weighted_cover_class {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        pub struct $name<'g, G: Graph> {
            graph: &'g G,
            weights: HashMap<G::Vertex, f64>,
        }

        impl<'g, G: Graph> $name<'g, G> {
            pub fn new(graph: &'g G) -> Result<Self> {
                props::require_undirected(graph)?;
                Ok($name { graph, weights: unit_weights(graph) })
            }
            pub fn with_weights(graph: &'g G, weights: HashMap<G::Vertex, f64>) -> Result<Self> {
                props::require_undirected(graph)?;
                Ok($name { graph, weights })
            }
        }
    };
}

weighted_cover_class!(
    /// Greedy cover by weight / degree ratio (`GreedyVCImpl`).
    GreedyVcImpl
);
weighted_cover_class!(
    /// Clarkson's 2-approximation (`ClarksonTwoApproxVCImpl`).
    ClarksonTwoApproxVcImpl
);
weighted_cover_class!(
    /// Bar-Yehuda and Even's 2-approximation (`BarYehudaEvenTwoApproxVCImpl`).
    BarYehudaEvenTwoApproxVcImpl
);
weighted_cover_class!(
    /// Exact cover by branch and bound with memoization (`RecursiveExactVCImpl`); exponential.
    RecursiveExactVcImpl
);

impl<G: Graph> VertexCoverAlgorithm<G::Vertex> for GreedyVcImpl<'_, G> {
    fn vertex_cover(&self) -> Result<VertexCover<G::Vertex>> {
        ratio_cover(self.graph, &self.weights, false)
    }
}

impl<G: Graph> VertexCoverAlgorithm<G::Vertex> for ClarksonTwoApproxVcImpl<'_, G> {
    fn vertex_cover(&self) -> Result<VertexCover<G::Vertex>> {
        ratio_cover(self.graph, &self.weights, true)
    }
}

impl<G: Graph> VertexCoverAlgorithm<G::Vertex> for BarYehudaEvenTwoApproxVcImpl<'_, G> {
    fn vertex_cover(&self) -> Result<VertexCover<G::Vertex>> {
        let g = self.graph;
        let mut w: HashMap<G::Vertex, f64> = HashMap::new();
        for v in g.vertices() {
            w.insert(v.clone(), weight_of(&self.weights, v)?);
        }
        let mut removed: HashSet<G::Vertex> = HashSet::new();
        let mut cover = Vec::new();
        let mut weight = 0.0;
        // the first remaining edge, repeatedly = a scan over the edges that skips removed vertices
        for e in g.edges() {
            let (p, q) = (g.edge_source(e).unwrap(), g.edge_target(e).unwrap());
            if removed.contains(p) || removed.contains(q) {
                continue;
            }
            let (wp, wq) = (w[p], w[q]);
            let chosen = if wp <= wq {
                *w.get_mut(q).unwrap() = wq - wp;
                p
            } else {
                *w.get_mut(p).unwrap() = wp - wq;
                q
            };
            cover.push(chosen.clone());
            weight += weight_of(&self.weights, chosen)?;
            removed.insert(chosen.clone());
        }
        Ok(VertexCover {
            vertices: cover,
            weight,
        })
    }
}

/// Takes both endpoints of a maximal matching (`EdgeBasedTwoApproxVCImpl`); the weight is the
/// number of vertices.
pub struct EdgeBasedTwoApproxVcImpl<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> EdgeBasedTwoApproxVcImpl<'g, G> {
    pub fn new(graph: &'g G) -> Result<Self> {
        props::require_undirected(graph)?;
        Ok(EdgeBasedTwoApproxVcImpl { graph })
    }
}

impl<G: Graph> VertexCoverAlgorithm<G::Vertex> for EdgeBasedTwoApproxVcImpl<'_, G> {
    fn vertex_cover(&self) -> Result<VertexCover<G::Vertex>> {
        let g = self.graph;
        let mut removed: HashSet<G::Vertex> = HashSet::new();
        let mut cover: indexmap::IndexSet<G::Vertex> = indexmap::IndexSet::new();
        for e in g.edges() {
            let (u, v) = (g.edge_source(e).unwrap(), g.edge_target(e).unwrap());
            if removed.contains(u) || removed.contains(v) {
                continue;
            }
            cover.insert(u.clone());
            cover.insert(v.clone());
            removed.insert(u.clone());
            removed.insert(v.clone());
        }
        let weight = cover.len() as f64;
        Ok(VertexCover {
            vertices: cover.into_iter().collect(),
            weight,
        })
    }
}

/// Bit set over vertex indices used as the memoization key of the exact solver.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Bits(Vec<u64>);

impl Bits {
    fn new(n: usize) -> Self {
        Bits(vec![0; n.div_ceil(64).max(1)])
    }
    fn set(&mut self, i: usize) {
        self.0[i / 64] |= 1 << (i % 64);
    }
    fn get(&self, i: usize) -> bool {
        self.0[i / 64] >> (i % 64) & 1 == 1
    }
    fn next_clear(&self, from: usize, n: usize) -> Option<usize> {
        (from..n).find(|&i| !self.get(i))
    }
}

#[derive(Clone)]
struct BitSetCover {
    bits: Bits,
    weight: f64,
}

struct ExactState<'a, V> {
    n: usize,
    /// neighbours (distinct, no self) of each vertex index
    neighbors: Vec<Vec<usize>>,
    weight: Vec<f64>,
    weighted: bool,
    memo: HashMap<Bits, BitSetCover>,
    upper_bound: f64,
    _vertices: std::marker::PhantomData<&'a V>,
}

impl<V> ExactState<'_, V> {
    fn get_weight(&self, vs: &[usize]) -> f64 {
        if self.weighted {
            vs.iter().map(|&i| self.weight[i]).sum()
        } else {
            vs.len() as f64
        }
    }

    fn recurse(
        &mut self,
        index_next_candidate: usize,
        visited: &Bits,
        accumulated: f64,
    ) -> BitSetCover {
        if let Some(hit) = self.memo.get(visited) {
            return hit.clone();
        }
        // next unvisited vertex that still has unvisited neighbours
        let mut next: Option<(usize, Vec<usize>)> = None;
        let mut index = visited.next_clear(index_next_candidate, self.n);
        while let Some(i) = index {
            let nbrs: Vec<usize> = self.neighbors[i]
                .iter()
                .copied()
                .filter(|&u| !visited.get(u))
                .collect();
            if !nbrs.is_empty() {
                next = Some((i, nbrs));
                break;
            }
            index = visited.next_clear(i + 1, self.n);
        }
        let Some((index_next, neighbors)) = next else {
            if accumulated <= self.upper_bound {
                self.upper_bound = accumulated - 1.0;
            }
            return BitSetCover {
                bits: Bits::new(self.n),
                weight: 0.0,
            };
        };
        if accumulated >= self.upper_bound {
            return BitSetCover {
                bits: Bits::new(self.n),
                weight: self.n as f64,
            };
        }
        // right branch: the neighbours join the cover
        let mut visited_right = visited.clone();
        visited_right.set(index_next);
        for &u in &neighbors {
            visited_right.set(u);
        }
        let weight = self.get_weight(&neighbors);
        let mut right = self.recurse(index_next + 1, &visited_right, accumulated + weight);
        for &u in &neighbors {
            right.bits.set(u);
        }
        right.weight += weight;
        // left branch: the vertex itself joins the cover
        let mut visited_left = visited.clone();
        visited_left.set(index_next);
        let w = self.weight[index_next];
        let mut left = self.recurse(index_next + 1, &visited_left, accumulated + w);
        left.bits.set(index_next);
        left.weight += w;
        let best = if left.weight <= right.weight {
            left
        } else {
            right
        };
        self.memo.insert(visited.clone(), best.clone());
        best
    }
}

impl<G: Graph> VertexCoverAlgorithm<G::Vertex> for RecursiveExactVcImpl<'_, G> {
    fn vertex_cover(&self) -> Result<VertexCover<G::Vertex>> {
        let g = self.graph;
        // (for unit weights the weighted sum equals the vertex count, so one code path suffices)
        let weighted = true;
        // vertices sorted by weight / degree, ascending (stable)
        let mut vertices: Vec<G::Vertex> = g.vertices().cloned().collect();
        let mut keyed: Vec<(f64, G::Vertex)> = Vec::new();
        for v in vertices.drain(..) {
            let ratio = weight_of(&self.weights, &v)? / g.degree_of(&v) as f64;
            keyed.push((ratio, v));
        }
        keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
        let vertices: Vec<G::Vertex> = keyed.into_iter().map(|(_, v)| v).collect();
        let index: HashMap<&G::Vertex, usize> =
            vertices.iter().enumerate().map(|(i, v)| (v, i)).collect();
        let neighbors: Vec<Vec<usize>> = vertices
            .iter()
            .map(|v| {
                let mut seen = indexmap::IndexSet::new();
                for e in g.edges_of(v) {
                    if let Some(w) = g.opposite_vertex(e, v) {
                        if w != v {
                            seen.insert(index[w]);
                        }
                    }
                }
                seen.into_iter().collect()
            })
            .collect();
        let weight: Vec<f64> = vertices
            .iter()
            .map(|v| weight_of(&self.weights, v))
            .collect::<Result<_>>()?;
        let upper_bound = GreedyVcImpl::with_weights(g, self.weights.clone())?
            .vertex_cover()?
            .weight
            .min(
                ClarksonTwoApproxVcImpl::with_weights(g, self.weights.clone())?
                    .vertex_cover()?
                    .weight,
            );
        let n = vertices.len();
        let mut state = ExactState::<G::Vertex> {
            n,
            neighbors,
            weight,
            weighted,
            memo: HashMap::new(),
            upper_bound,
            _vertices: std::marker::PhantomData,
        };
        let cover = state.recurse(0, &Bits::new(n), 0.0);
        let vertices_in_cover: Vec<G::Vertex> = (0..n)
            .filter(|&i| cover.bits.get(i))
            .map(|i| vertices[i].clone())
            .collect();
        Ok(VertexCover {
            vertices: vertices_in_cover,
            weight: cover.weight,
        })
    }
}
