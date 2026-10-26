//! Path enumeration, disjoint paths and negative cycle detection.

use super::flow::min_cost_flow;
use super::util::Indexed;
use crate::graph::{DirectedGraph, Graph, GraphMut, GraphMutExt};
use crate::walk::GraphWalk;
use crate::{DefaultEdge, DiGraph, GraphError, Result};
use std::collections::{HashMap, HashSet};

/// All simple (vertex-repeating-free) paths from `source` to `sink`, optionally limited to
/// `max_edges` edges. Parallel edges give distinct paths.
pub fn all_simple_paths<G: Graph>(
    g: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    max_edges: Option<usize>,
) -> Vec<GraphWalk<G::Vertex, G::Edge>> {
    let ix = Indexed::new(g);
    let (Some(&s), Some(&t)) = (ix.idx.get(source), ix.idx.get(sink)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let (mut on_path, mut vs, mut es) = (vec![false; ix.n()], vec![s], Vec::new());
    on_path[s] = true;
    #[allow(clippy::too_many_arguments)]
    fn rec<G: Graph>(
        ix: &Indexed<'_, G>,
        v: usize,
        t: usize,
        max: Option<usize>,
        on_path: &mut Vec<bool>,
        vs: &mut Vec<usize>,
        es: &mut Vec<usize>,
        out: &mut Vec<GraphWalk<G::Vertex, G::Edge>>,
    ) {
        if v == t && !es.is_empty() {
            out.push(GraphWalk {
                start: ix.vertex(vs[0]),
                end: ix.vertex(t),
                vertices: ix.vertices_of(vs),
                edges: es.iter().map(|&k| ix.es[k].clone()).collect(),
                weight: es.iter().map(|&k| ix.w[k]).sum(),
            });
            return;
        }
        if max.is_some_and(|m| es.len() >= m) {
            return;
        }
        for &(u, k) in &ix.out[v] {
            if !on_path[u] || (u == t && !on_path[t]) {
                on_path[u] = true;
                vs.push(u);
                es.push(k);
                rec(ix, u, t, max, on_path, vs, es, out);
                es.pop();
                vs.pop();
                on_path[u] = false;
            }
        }
    }
    if s == t {
        // a "path" from a vertex to itself is the empty path
        return vec![GraphWalk::single(source.clone())];
    }
    rec(
        &ix,
        s,
        t,
        max_edges,
        &mut on_path,
        &mut vs,
        &mut es,
        &mut out,
    );
    out
}

/// A negative-weight cycle (as its vertex sequence, without repeating the first vertex), if any.
/// In undirected graphs any negative edge is a negative cycle of length two.
pub fn find_negative_cycle<G: Graph>(g: &G) -> Option<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut dist = vec![0.0f64; n];
    let mut pred = vec![usize::MAX; n];
    let mut last = usize::MAX;
    for _ in 0..=n {
        last = usize::MAX;
        for v in 0..n {
            for &(u, k) in &ix.out[v] {
                if dist[v] + ix.w[k] < dist[u] - 1e-12 {
                    dist[u] = dist[v] + ix.w[k];
                    pred[u] = v;
                    last = u;
                }
            }
        }
        if last == usize::MAX {
            return None;
        }
    }
    // walk back n steps to land inside the cycle, then collect it
    for _ in 0..n {
        last = pred[last];
    }
    let mut cycle = vec![last];
    let mut cur = pred[last];
    while cur != last {
        cycle.push(cur);
        cur = pred[cur];
    }
    cycle.reverse();
    Some(ix.vertices_of(&cycle))
}

struct Arc {
    capacity: f64,
    cost: f64,
    /// index of the original edge, `None` for the vertex-splitting arcs
    origin: Option<usize>,
}

fn disjoint<G: DirectedGraph>(
    g: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
    vertex_disjoint: bool,
) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
    if source == sink {
        return Err(GraphError::InvalidArgument(
            "source and sink must differ".into(),
        ));
    }
    let ix = Indexed::new(g);
    let s0 = *ix.idx.get(source).ok_or(GraphError::VertexNotFound)?;
    let t0 = *ix.idx.get(sink).ok_or(GraphError::VertexNotFound)?;
    let n = ix.n();
    // vertex-disjoint: split v into (2v = in, 2v+1 = out) joined by a unit arc
    let (s, t, size) = if vertex_disjoint {
        (2 * s0 + 1, 2 * t0, 2 * n)
    } else {
        (s0, t0, n)
    };
    let mut net = DiGraph::<usize>::multi().weighted();
    net.add_vertices(0..size);
    let mut arcs: HashMap<DefaultEdge, Arc> = HashMap::new();
    let mut add = |net: &mut DiGraph<usize>, a: usize, b: usize, arc: Arc| -> Result<()> {
        let e = net.new_edge()?;
        net.add_edge_with(&a, &b, e)?;
        arcs.insert(e, arc);
        Ok(())
    };
    if vertex_disjoint {
        for v in 0..n {
            let capacity = if v == s0 || v == t0 { k as f64 } else { 1.0 };
            add(
                &mut net,
                2 * v,
                2 * v + 1,
                Arc {
                    capacity,
                    cost: 0.0,
                    origin: None,
                },
            )?;
        }
    }
    for (e, &(a, b)) in ix.ends.iter().enumerate() {
        if a != b {
            let (x, y) = if vertex_disjoint {
                (2 * a + 1, 2 * b)
            } else {
                (a, b)
            };
            add(
                &mut net,
                x,
                y,
                Arc {
                    capacity: 1.0,
                    cost: ix.w[e],
                    origin: Some(e),
                },
            )?;
        }
    }
    let r = min_cost_flow(
        &net,
        &s,
        &t,
        Some(k as f64),
        |e| arcs[e].capacity,
        |e| arcs[e].cost,
    )?;
    // peel off one path per unit of flow
    let mut used: HashSet<DefaultEdge> = HashSet::new();
    let mut paths = Vec::new();
    for _ in 0..r.flow_value.round() as usize {
        let (mut cur, mut es) = (s, Vec::new());
        while cur != t {
            let next = net
                .out_edges_of(&cur)
                .find(|e| r.flow[*e] > 0.5 && !used.contains(*e))
                .copied()
                .expect("flow conservation guarantees a continuation");
            used.insert(next);
            es.extend(arcs[&next].origin);
            cur = *net.edge_target(&next).unwrap();
        }
        let mut vertices = vec![ix.vertex(s0)];
        vertices.extend(es.iter().map(|&k| ix.vertex(ix.ends[k].1)));
        paths.push(GraphWalk {
            start: ix.vertex(s0),
            end: ix.vertex(t0),
            vertices,
            weight: es.iter().map(|&k| ix.w[k]).sum(),
            edges: es.iter().map(|&k| ix.es[k].clone()).collect(),
        });
    }
    Ok(paths)
}

/// Up to `k` edge-disjoint paths from `source` to `sink` of minimum total weight
/// (non-negative weights; generalises Suurballe's algorithm). Fewer are returned if the
/// graph does not contain `k` edge-disjoint paths.
pub fn edge_disjoint_shortest_paths<G: DirectedGraph>(
    g: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
    disjoint(g, source, sink, k, false)
}

/// Up to `k` internally vertex-disjoint paths of minimum total weight.
pub fn vertex_disjoint_shortest_paths<G: DirectedGraph>(
    g: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
    disjoint(g, source, sink, k, true)
}
