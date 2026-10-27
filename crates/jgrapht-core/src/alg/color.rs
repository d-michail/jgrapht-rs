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

// ---------------------------------------------------------------------------------------------
// JGraphT's colouring classes

use super::interfaces::VertexColoringAlgorithm;
use crate::graph::GraphExt;
use crate::util::java::{RandomSource, java_shuffle};
use crate::{GraphError, Result};
use std::collections::BTreeSet;
use std::hash::Hash;

const SELF_LOOPS_NOT_ALLOWED: &str = "Self-loops not allowed";

/// Colours the vertices in `order` with the smallest colour not used by an already coloured
/// neighbour (`GreedyColoring.getColoring`).
fn color_in_order<G: Graph>(g: &G, order: &[G::Vertex]) -> Result<Coloring<G::Vertex>> {
    let mut colors: HashMap<G::Vertex, usize> = HashMap::new();
    let mut max_color: i64 = -1;
    let mut used: HashSet<usize> = HashSet::new();
    for v in order {
        for e in g.edges_of(v) {
            let u = g.opposite_vertex(e, v).unwrap();
            if v == u {
                return Err(GraphError::InvalidArgument(SELF_LOOPS_NOT_ALLOWED.into()));
            }
            if let Some(&c) = colors.get(u) {
                used.insert(c);
            }
        }
        let mut candidate = 0;
        while used.contains(&candidate) {
            candidate += 1;
        }
        used.clear();
        colors.insert(v.clone(), candidate);
        max_color = max_color.max(candidate as i64);
    }
    Ok(Coloring {
        colors,
        count: (max_color + 1) as usize,
    })
}

fn degrees<G: Graph>(g: &G) -> (Vec<&G::Vertex>, Vec<usize>, usize) {
    let vs: Vec<&G::Vertex> = g.vertices().collect();
    let degree: Vec<usize> = vs.iter().map(|v| g.edges_of(v).count()).collect();
    let max = degree.iter().copied().max().unwrap_or(0);
    (vs, degree, max)
}

/// Greedy colouring in vertex iteration order (`GreedyColoring`).
pub struct GreedyColoring<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> GreedyColoring<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        GreedyColoring { graph }
    }
}

impl<G: Graph> VertexColoringAlgorithm<G::Vertex> for GreedyColoring<'_, G> {
    fn coloring(&self) -> Result<Coloring<G::Vertex>> {
        let order: Vec<G::Vertex> = self.graph.vertices().cloned().collect();
        color_in_order(self.graph, &order)
    }
}

/// Greedy colouring, largest degree first (`LargestDegreeFirstColoring`). Vertices of equal
/// degree stay in graph order.
pub struct LargestDegreeFirstColoring<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> LargestDegreeFirstColoring<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        LargestDegreeFirstColoring { graph }
    }
}

impl<G: Graph> VertexColoringAlgorithm<G::Vertex> for LargestDegreeFirstColoring<'_, G> {
    fn coloring(&self) -> Result<Coloring<G::Vertex>> {
        let (vs, degree, max_degree) = degrees(self.graph);
        let n = vs.len();
        let order: Vec<G::Vertex> = if max_degree > 3 * n {
            // too many parallel edges for bucket sort: comparison sort (stable)
            let mut idx: Vec<usize> = (0..n).collect();
            idx.sort_by(|&a, &b| degree[b].cmp(&degree[a]));
            idx.into_iter().map(|i| vs[i].clone()).collect()
        } else {
            let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); max_degree + 1];
            for i in 0..n {
                buckets[degree[i]].push(i);
            }
            buckets
                .into_iter()
                .rev()
                .flatten()
                .map(|i| vs[i].clone())
                .collect()
        };
        color_in_order(self.graph, &order)
    }
}

/// Greedy colouring along a smallest-degree-last ordering (`SmallestDegreeLastColoring`).
pub struct SmallestDegreeLastColoring<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> SmallestDegreeLastColoring<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        SmallestDegreeLastColoring { graph }
    }
}

impl<G: Graph> VertexColoringAlgorithm<G::Vertex> for SmallestDegreeLastColoring<'_, G> {
    fn coloring(&self) -> Result<Coloring<G::Vertex>> {
        let g = self.graph;
        let (vs, degree0, max_degree) = degrees(g);
        let index: HashMap<&G::Vertex, usize> =
            vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        // buckets keep their members in graph order, standing in for JGraphT's `HashSet`s
        let mut buckets: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); max_degree + 1];
        for (i, &d) in degree0.iter().enumerate() {
            buckets[d].insert(i);
        }
        let mut degree: HashMap<usize, usize> = degree0.iter().copied().enumerate().collect();
        let mut order: std::collections::VecDeque<G::Vertex> = std::collections::VecDeque::new();
        let mut i = 0;
        while i <= max_degree {
            while let Some(&v) = buckets[i].iter().next() {
                buckets[i].remove(&v);
                order.push_front(vs[v].clone());
                degree.remove(&v);
                for e in g.edges_of(vs[v]) {
                    let u = g.opposite_vertex(e, vs[v]).unwrap();
                    if vs[v] == u {
                        return Err(GraphError::InvalidArgument(SELF_LOOPS_NOT_ALLOWED.into()));
                    }
                    let ui = index[u];
                    if let Some(&d) = degree.get(&ui) {
                        if d > 0 {
                            buckets[d].remove(&ui);
                            degree.insert(ui, d - 1);
                            buckets[d - 1].insert(ui);
                            if d - 1 < i {
                                i = d - 1;
                            }
                        }
                    }
                }
            }
            i += 1;
        }
        let order: Vec<G::Vertex> = order.into_iter().collect();
        color_in_order(g, &order)
    }
}

/// Greedy colouring in a random order (`RandomGreedyColoring`), shuffled as `Collections.shuffle`.
pub struct RandomGreedyColoring<'g, G: Graph, R> {
    graph: &'g G,
    rng: std::cell::RefCell<R>,
}

impl<'g, G: Graph, R: RandomSource> RandomGreedyColoring<'g, G, R> {
    pub fn new(graph: &'g G, rng: R) -> Self {
        RandomGreedyColoring {
            graph,
            rng: std::cell::RefCell::new(rng),
        }
    }
}

impl<G: Graph, R: RandomSource> VertexColoringAlgorithm<G::Vertex>
    for RandomGreedyColoring<'_, G, R>
{
    fn coloring(&self) -> Result<Coloring<G::Vertex>> {
        let mut order: Vec<G::Vertex> = self.graph.vertices().cloned().collect();
        java_shuffle(&mut order, &mut *self.rng.borrow_mut());
        color_in_order(self.graph, &order)
    }
}

/// DSatur colouring: next the vertex with the most distinct neighbour colours, then the highest
/// remaining degree (`SaturationDegreeColoring`, including its own binary heap and tie-breaking).
pub struct SaturationDegreeColoring<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> SaturationDegreeColoring<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        SaturationDegreeColoring { graph }
    }
}

/// The 1-based array heap of JGraphT's `SaturationDegreeColoring`, over vertex indices.
struct DSaturHeap {
    /// `array[1..=size]`; entries are vertex indices
    array: Vec<usize>,
    /// heap position of each vertex (0 = not in the heap)
    index: Vec<usize>,
    size: usize,
}

impl DSaturHeap {
    fn new(n: usize) -> Self {
        DSaturHeap {
            array: vec![usize::MAX; n + 1],
            index: vec![0; n],
            size: 0,
        }
    }
    /// Orders by larger saturation, then larger degree.
    fn compare(sat: &[usize], deg: &[usize], a: usize, b: usize) -> std::cmp::Ordering {
        sat[b].cmp(&sat[a]).then(deg[b].cmp(&deg[a]))
    }
    fn fixdown(&mut self, mut k: usize, sat: &[usize], deg: &[usize]) {
        let h = self.array[k];
        while 2 * k <= self.size {
            let mut j = 2 * k;
            if j < self.size && Self::compare(sat, deg, self.array[j], self.array[j + 1]).is_gt() {
                j += 1;
            }
            if Self::compare(sat, deg, h, self.array[j]).is_le() {
                break;
            }
            self.array[k] = self.array[j];
            self.index[self.array[k]] = k;
            k = j;
        }
        self.array[k] = h;
        self.index[h] = k;
    }
    fn fixup_at(&mut self, mut k: usize, sat: &[usize], deg: &[usize]) {
        let h = self.array[k];
        while k > 1 && Self::compare(sat, deg, self.array[k / 2], h).is_gt() {
            self.array[k] = self.array[k / 2];
            self.index[self.array[k]] = k;
            k /= 2;
        }
        self.array[k] = h;
        self.index[h] = k;
    }
    fn force_fixup(&mut self, mut k: usize) {
        let h = self.array[k];
        while k > 1 {
            self.array[k] = self.array[k / 2];
            self.index[self.array[k]] = k;
            k /= 2;
        }
        self.array[k] = h;
        self.index[h] = k;
    }
    fn delete_min(&mut self, sat: &[usize], deg: &[usize]) -> usize {
        let result = self.array[1];
        if self.size == 1 {
            self.array[1] = usize::MAX;
            self.size = 0;
        } else {
            self.array[1] = self.array[self.size];
            self.array[self.size] = usize::MAX;
            self.size -= 1;
            self.fixdown(1, sat, deg);
        }
        self.index[result] = 0;
        result
    }
    fn delete(&mut self, v: usize, sat: &[usize], deg: &[usize]) {
        self.force_fixup(self.index[v]);
        self.delete_min(sat, deg);
    }
    fn insert(&mut self, v: usize, sat: &[usize], deg: &[usize]) {
        self.size += 1;
        self.array[self.size] = v;
        self.index[v] = self.size;
        self.fixup_at(self.size, sat, deg);
    }
    fn bulk_insert(&mut self, vs: impl Iterator<Item = usize>, sat: &[usize], deg: &[usize]) {
        for v in vs {
            self.size += 1;
            self.array[self.size] = v;
            self.index[v] = self.size;
        }
        for i in (1..=self.size / 2).rev() {
            self.fixdown(i, sat, deg);
        }
    }
}

impl<G: Graph> VertexColoringAlgorithm<G::Vertex> for SaturationDegreeColoring<'_, G> {
    fn coloring(&self) -> Result<Coloring<G::Vertex>> {
        let g = self.graph;
        let (vs, mut degree, _) = degrees(g);
        let n = vs.len();
        let index: HashMap<&G::Vertex, usize> =
            vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let mut saturation = vec![0usize; n];
        let mut adj_colors: Vec<HashSet<usize>> = vec![HashSet::new(); n];
        let mut colors: HashMap<G::Vertex, usize> = HashMap::new();
        let mut colored = vec![false; n];
        let mut max_color: i64 = -1;
        let mut heap = DSaturHeap::new(n);
        heap.bulk_insert(0..n, &saturation, &degree);
        while heap.size > 0 {
            let v = heap.delete_min(&saturation, &degree);
            let used = &adj_colors[v];
            let c = (0..).find(|c| !used.contains(c)).unwrap();
            max_color = max_color.max(c as i64);
            colors.insert(vs[v].clone(), c);
            colored[v] = true;
            for e in g.edges_of(vs[v]) {
                let u = index[g.opposite_vertex(e, vs[v]).unwrap()];
                if !colored[u] {
                    if adj_colors[u].contains(&c) {
                        heap.delete(u, &saturation, &degree);
                        degree[u] -= 1;
                        heap.insert(u, &saturation, &degree);
                    } else {
                        adj_colors[u].insert(c);
                        saturation[u] += 1;
                        degree[u] -= 1;
                        heap.fixup_at(heap.index[u], &saturation, &degree);
                    }
                }
            }
        }
        Ok(Coloring {
            colors,
            count: (max_color + 1) as usize,
        })
    }
}

/// Exact colouring by Brown's backtracking (`BrownBacktrackColoring`); exponential time.
pub struct BrownBacktrackColoring<V: Clone + Eq + Hash> {
    vertex_list: Vec<V>,
    neighbors: Vec<Vec<usize>>,
    coloring: std::cell::OnceCell<Coloring<V>>,
}

struct BrownState<'a> {
    neighbors: &'a [Vec<usize>],
    partial: Vec<usize>,
    color_count: Vec<usize>,
    allowed: Vec<Vec<bool>>,
    chi: usize,
    complete: Vec<usize>,
}

impl BrownState<'_> {
    fn recursive_color(&mut self, pos: usize) {
        let last = self.neighbors.len() - 1;
        self.color_count[pos] = self.color_count[pos - 1];
        // colours 0..=colorCount may be used; colour 0 is never assigned
        let cc = self.color_count[pos];
        self.allowed[pos] = (0..=cc).map(|_| true).collect();
        for &nb in &self.neighbors[pos] {
            let c = self.partial[nb];
            if c > 0 && c < self.allowed[pos].len() {
                self.allowed[pos][c] = false;
            }
        }
        let mut i = 1;
        while i <= self.color_count[pos] && self.color_count[pos] < self.chi {
            if self.allowed[pos].get(i).copied().unwrap_or(false) {
                self.partial[pos] = i;
                if pos < last {
                    self.recursive_color(pos + 1);
                } else {
                    self.chi = self.color_count[pos];
                    self.complete.copy_from_slice(&self.partial);
                }
            }
            i += 1;
        }
        if self.color_count[pos] + 1 < self.chi {
            self.color_count[pos] += 1;
            self.partial[pos] = self.color_count[pos];
            if pos < last {
                self.recursive_color(pos + 1);
            } else {
                self.chi = self.color_count[pos];
                self.complete.copy_from_slice(&self.partial);
            }
        }
        self.partial[pos] = 0;
    }
}

impl<V: Clone + Eq + Hash> BrownBacktrackColoring<V> {
    pub fn new<G: Graph<Vertex = V>>(graph: &G) -> Self {
        let vertex_list: Vec<V> = graph.vertices().cloned().collect();
        let index: HashMap<&V, usize> = vertex_list
            .iter()
            .enumerate()
            .map(|(i, v)| (v, i))
            .collect();
        let neighbors = vertex_list
            .iter()
            .map(|v| {
                graph
                    .edges_of(v)
                    .map(|e| index[graph.opposite_vertex(e, v).unwrap()])
                    .collect()
            })
            .collect();
        BrownBacktrackColoring {
            vertex_list,
            neighbors,
            coloring: std::cell::OnceCell::new(),
        }
    }

    fn compute(&self) -> Coloring<V> {
        let n = self.neighbors.len();
        if n == 0 {
            return Coloring {
                colors: HashMap::new(),
                count: 0,
            };
        }
        let mut state = BrownState {
            neighbors: &self.neighbors,
            partial: vec![0; n],
            color_count: vec![0; n],
            allowed: vec![Vec::new(); n],
            chi: n + 1,
            complete: vec![0; n],
        };
        state.partial[0] = 1; // the first vertex always gets colour 1
        state.color_count[0] = 1;
        if n == 1 {
            // (JGraphT would index past the end here; a single vertex needs one colour)
            state.chi = 1;
            state.complete[0] = 1;
        } else {
            state.recursive_color(1);
        }
        Coloring {
            colors: self
                .vertex_list
                .iter()
                .cloned()
                .zip(state.complete.iter().copied())
                .collect(),
            count: state.chi,
        }
    }

    pub fn chromatic_number(&self) -> usize {
        self.coloring.get_or_init(|| self.compute()).count
    }
}

impl<V: Clone + Eq + Hash> VertexColoringAlgorithm<V> for BrownBacktrackColoring<V> {
    fn coloring(&self) -> Result<Coloring<V>> {
        Ok(self.coloring.get_or_init(|| self.compute()).clone())
    }
}

/// Optimal colouring of a chordal graph (`ChordalGraphColoring`); the colour count is the number of
/// distinct colours used.
pub struct ChordalGraphColoring<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> ChordalGraphColoring<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        ChordalGraphColoring { graph }
    }
    /// `None` when the graph is not chordal.
    pub fn coloring(&self) -> Option<Coloring<G::Vertex>> {
        chordal_coloring(self.graph)
    }
}

/// Colour refinement on successor lists: repeatedly splits colour classes by the sorted multiset
/// of successor colours until stable. Returns the colours and the number of classes.
pub(crate) fn refine_colors(succ: &[Vec<usize>], initial: Vec<usize>) -> (Vec<usize>, usize) {
    let mut color = initial;
    let n = color.len();
    let mut classes = color.iter().collect::<HashSet<_>>().len();
    loop {
        let sigs: Vec<(usize, Vec<usize>)> = (0..n)
            .map(|v| {
                let mut s: Vec<usize> = succ[v].iter().map(|&u| color[u]).collect();
                s.sort_unstable();
                (color[v], s)
            })
            .collect();
        let mut ids: HashMap<&(usize, Vec<usize>), usize> = HashMap::new();
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
    (color, classes)
}

/// Colour refinement (`ColorRefinementAlgorithm`): the coarsest equitable partition refining
/// `alpha`, where a vertex's signature is the multiset of its *successors'* colours (for directed
/// graphs only outgoing edges count, as in JGraphT; parallel edges count once).
pub struct ColorRefinementAlgorithm<'g, G: Graph> {
    graph: &'g G,
    alpha: Option<Coloring<G::Vertex>>,
}

impl<'g, G: Graph> ColorRefinementAlgorithm<'g, G> {
    /// Starts from the trivial colouring (all vertices alike).
    pub fn new(graph: &'g G) -> Self {
        ColorRefinementAlgorithm { graph, alpha: None }
    }

    /// Starts from `alpha`, which must colour every vertex with colours `0..alpha.count` (all used).
    pub fn with_alpha(graph: &'g G, alpha: Coloring<G::Vertex>) -> Result<Self> {
        let used: HashSet<usize> = alpha.colors.values().copied().collect();
        let consistent = alpha.colors.len() == graph.vertex_count()
            && used.len() == alpha.count
            && graph
                .vertices()
                .all(|v| alpha.colors.get(v).is_some_and(|&c| c < alpha.count));
        if !consistent {
            return Err(GraphError::InvalidArgument(
                "alpha is not a valid surjective l-coloring for the given graph.".into(),
            ));
        }
        Ok(ColorRefinementAlgorithm {
            graph,
            alpha: Some(alpha),
        })
    }
}

impl<G: Graph> VertexColoringAlgorithm<G::Vertex> for ColorRefinementAlgorithm<'_, G> {
    fn coloring(&self) -> Result<Coloring<G::Vertex>> {
        let g = self.graph;
        let vs: Vec<&G::Vertex> = g.vertices().collect();
        let index: HashMap<&G::Vertex, usize> =
            vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let succ: Vec<Vec<usize>> = vs
            .iter()
            .map(|v| {
                let mut seen = HashSet::new();
                g.out_edges_of(v)
                    .filter_map(|e| g.opposite_vertex(e, v))
                    .map(|w| index[w])
                    .filter(|w| seen.insert(*w))
                    .collect()
            })
            .collect();
        let initial: Vec<usize> = match &self.alpha {
            None => vec![0; vs.len()],
            Some(a) => vs.iter().map(|v| a.colors[*v]).collect(),
        };
        let (color, classes) = refine_colors(&succ, initial);
        Ok(Coloring {
            colors: vs
                .iter()
                .enumerate()
                .map(|(i, v)| ((*v).clone(), color[i]))
                .collect(),
            count: classes,
        })
    }
}
