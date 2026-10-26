//! Connected components, strong connectivity, biconnectivity and condensation.

use super::util::Indexed;
use crate::graph::{DirectedGraph, Graph};
use crate::traverse::DfsPostOrder;
use crate::view::EdgeReversed;
use crate::{DiGraph, GraphMut, GraphMutExt};
use std::collections::HashSet;

/// Connected components (weak components for directed graphs), in vertex order.
pub fn connected_components<G: Graph>(g: &G) -> Vec<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut comp = vec![usize::MAX; n];
    let mut out: Vec<Vec<usize>> = Vec::new();
    for s in 0..n {
        if comp[s] != usize::MAX {
            continue;
        }
        let c = out.len();
        comp[s] = c;
        let mut members = vec![s];
        let mut stack = vec![s];
        while let Some(v) = stack.pop() {
            for &(u, _) in ix.out[v].iter().chain(&ix.inn[v]) {
                if comp[u] == usize::MAX {
                    comp[u] = c;
                    members.push(u);
                    stack.push(u);
                }
            }
        }
        members.sort_unstable();
        out.push(members);
    }
    out.iter().map(|m| ix.vertices_of(m)).collect()
}

/// The (weak) component containing `v`.
pub fn connected_component_of<G: Graph>(g: &G, v: &G::Vertex) -> Option<Vec<G::Vertex>> {
    connected_components(g).into_iter().find(|c| c.contains(v))
}

/// Strongly connected components (Tarjan, iterative). Components come out in reverse
/// topological order of the condensation.
pub fn strongly_connected_components<G: DirectedGraph>(g: &G) -> Vec<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let (mut index, mut low) = (vec![usize::MAX; n], vec![0; n]);
    let (mut on, mut stack, mut next) = (vec![false; n], Vec::new(), 0);
    let mut comps = Vec::new();
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut call = vec![(root, 0usize)];
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on[root] = true;
        while let Some(&mut (v, ref mut i)) = call.last_mut() {
            if *i < ix.out[v].len() {
                let u = ix.out[v][*i].0;
                *i += 1;
                if index[u] == usize::MAX {
                    index[u] = next;
                    low[u] = next;
                    next += 1;
                    stack.push(u);
                    on[u] = true;
                    call.push((u, 0));
                } else if on[u] {
                    low[v] = low[v].min(index[u]);
                }
            } else {
                call.pop();
                if let Some(&(p, _)) = call.last() {
                    low[p] = low[p].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut c = Vec::new();
                    loop {
                        let u = stack.pop().unwrap();
                        on[u] = false;
                        c.push(u);
                        if u == v {
                            break;
                        }
                    }
                    c.sort_unstable();
                    comps.push(ix.vertices_of(&c));
                }
            }
        }
    }
    comps
}

/// Strongly connected components via Kosaraju–Sharir (two DFS passes).
pub fn strongly_connected_components_kosaraju<G: DirectedGraph>(g: &G) -> Vec<Vec<G::Vertex>> {
    let order: Vec<_> = DfsPostOrder::all(g).collect();
    let rev = EdgeReversed(g);
    let mut seen: HashSet<&G::Vertex> = HashSet::new();
    let mut comps = Vec::new();
    for &root in order.iter().rev() {
        if seen.contains(root) {
            continue;
        }
        let mut comp = Vec::new();
        let mut stack = vec![root];
        seen.insert(root);
        while let Some(v) = stack.pop() {
            comp.push(v.clone());
            for e in rev.edges_of(v) {
                let n = if rev.edge_source(e) == Some(v) {
                    rev.edge_target(e)
                } else {
                    None
                };
                if let Some(n) = n {
                    if seen.insert(n) {
                        stack.push(n);
                    }
                }
            }
        }
        comps.push(comp);
    }
    comps
}

pub fn is_strongly_connected<G: DirectedGraph>(g: &G) -> bool {
    g.vertex_count() > 0 && strongly_connected_components(g).len() == 1
}

/// Condensation: one vertex per strongly connected component (its index in `components`),
/// with an edge for every pair of components joined by at least one edge.
pub fn condensation<G: DirectedGraph>(g: &G) -> (DiGraph<usize>, Vec<Vec<G::Vertex>>) {
    let comps = strongly_connected_components(g);
    let comp_of: std::collections::HashMap<&G::Vertex, usize> = comps
        .iter()
        .enumerate()
        .flat_map(|(i, c)| c.iter().map(move |v| (v, i)))
        .collect();
    let mut dag = DiGraph::<usize>::simple();
    dag.add_vertices(0..comps.len());
    for e in g.edges() {
        let (a, b) = (
            comp_of[g.edge_source(e).unwrap()],
            comp_of[g.edge_target(e).unwrap()],
        );
        if a != b {
            let _ = dag.add_edge(&a, &b);
        }
    }
    (dag, comps)
}

/// Result of a biconnectivity analysis of an undirected graph.
#[derive(Debug, Clone)]
pub struct Biconnectivity<V, E> {
    pub articulation_points: Vec<V>,
    pub bridges: Vec<E>,
    /// Each block (biconnected component) as its set of edges; isolated vertices have none.
    pub blocks: Vec<Vec<E>>,
}

/// Articulation points, bridges and blocks of an undirected graph (edge directions ignored).
pub fn biconnectivity<G: Graph>(g: &G) -> Biconnectivity<G::Vertex, G::Edge> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for (k, &(s, t)) in ix.ends.iter().enumerate() {
        if s != t {
            adj[s].push((t, k));
            adj[t].push((s, k));
        }
    }
    let (mut disc, mut low) = (vec![usize::MAX; n], vec![0usize; n]);
    let (mut art, mut bridges, mut blocks) = (vec![false; n], Vec::new(), Vec::new());
    let (mut time, mut estack) = (0, Vec::<usize>::new());
    for root in 0..n {
        if disc[root] != usize::MAX {
            continue;
        }
        disc[root] = time;
        low[root] = time;
        time += 1;
        let mut root_children = 0;
        let mut call = vec![(root, usize::MAX, 0usize)]; // vertex, parent edge, next adj index
        while let Some(&mut (v, pe, ref mut i)) = call.last_mut() {
            if *i < adj[v].len() {
                let (u, k) = adj[v][*i];
                *i += 1;
                if k == pe {
                    continue;
                }
                if disc[u] == usize::MAX {
                    disc[u] = time;
                    low[u] = time;
                    time += 1;
                    estack.push(k);
                    call.push((u, k, 0));
                } else if disc[u] < disc[v] {
                    low[v] = low[v].min(disc[u]);
                    estack.push(k);
                }
            } else {
                call.pop();
                if let Some(&(p, _, _)) = call.last() {
                    low[p] = low[p].min(low[v]);
                    if low[v] >= disc[p] {
                        if p == root {
                            root_children += 1;
                        } else {
                            art[p] = true;
                        }
                        let mut block = Vec::new();
                        while let Some(k) = estack.pop() {
                            block.push(k);
                            if k == pe {
                                break;
                            }
                        }
                        if low[v] > disc[p] {
                            bridges.push(pe);
                        }
                        blocks.push(block);
                    }
                }
            }
        }
        if root_children > 1 {
            art[root] = true;
        }
    }
    Biconnectivity {
        articulation_points: (0..n).filter(|&i| art[i]).map(|i| ix.vertex(i)).collect(),
        bridges: bridges.into_iter().map(|k| ix.es[k].clone()).collect(),
        blocks: blocks
            .into_iter()
            .map(|b| b.into_iter().map(|k| ix.es[k].clone()).collect())
            .collect(),
    }
}

/// No articulation points and connected (and at least 3 vertices, or a single edge).
pub fn is_biconnected<G: Graph>(g: &G) -> bool {
    crate::graph::props::is_connected(g) && biconnectivity(g).articulation_points.is_empty()
}

/// Block–cut tree of an undirected graph: one node per block and per articulation point, with
/// an edge between a cut vertex and every block containing it.
#[derive(Debug, Clone)]
pub struct BlockCutTree<V> {
    /// Vertices of each block.
    pub blocks: Vec<Vec<V>>,
    pub cut_vertices: Vec<V>,
    /// `(block index, cut vertex index)` pairs.
    pub edges: Vec<(usize, usize)>,
}

pub fn block_cut_tree<G: Graph>(g: &G) -> BlockCutTree<G::Vertex> {
    let bc = biconnectivity(g);
    let mut blocks: Vec<Vec<G::Vertex>> = Vec::new();
    for b in &bc.blocks {
        let mut vs: Vec<G::Vertex> = Vec::new();
        for e in b {
            for v in [g.edge_source(e).unwrap(), g.edge_target(e).unwrap()] {
                if !vs.contains(v) {
                    vs.push(v.clone());
                }
            }
        }
        blocks.push(vs);
    }
    // isolated vertices form singleton blocks
    for v in g.vertices() {
        if g.edges_of(v).next().is_none() {
            blocks.push(vec![v.clone()]);
        }
    }
    let mut edges = Vec::new();
    for (ci, c) in bc.articulation_points.iter().enumerate() {
        for (bi, b) in blocks.iter().enumerate() {
            if b.contains(c) {
                edges.push((bi, ci));
            }
        }
    }
    BlockCutTree {
        blocks,
        cut_vertices: bc.articulation_points,
        edges,
    }
}
