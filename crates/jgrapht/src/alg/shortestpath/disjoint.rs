use crate::alg::util::Indexed;
use crate::graph::{DirectedGraph, props};
use crate::util::OrderedF64;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone)]
struct ResidualArc {
    tail: usize,
    head: usize,
    cost: f64,
    residual: bool,
    original_edge: Option<usize>,
    forward: bool,
}

struct ResidualNetwork {
    arcs: Vec<ResidualArc>,
    outgoing: Vec<Vec<usize>>,
}

impl ResidualNetwork {
    fn new(vertices: usize) -> Self {
        Self {
            arcs: Vec::new(),
            outgoing: vec![Vec::new(); vertices],
        }
    }

    fn add_arc(&mut self, tail: usize, head: usize, cost: f64, original_edge: Option<usize>) {
        let f = self.arcs.len();
        let r = f + 1;
        self.arcs.push(ResidualArc {
            tail,
            head,
            cost,
            residual: true,
            original_edge,
            forward: true,
        });
        self.arcs.push(ResidualArc {
            tail: head,
            head: tail,
            cost: -cost,
            residual: false,
            original_edge,
            forward: false,
        });
        self.outgoing[tail].push(f);
        self.outgoing[head].push(r);
    }

    fn augment(&mut self, source: usize, target: usize, pred: &[usize]) {
        let mut vertex = target;
        while vertex != source {
            let arc = pred[vertex];
            self.arcs[arc].residual = false;
            self.arcs[arc ^ 1].residual = true;
            vertex = self.arcs[arc].tail;
        }
    }

    fn dijkstra(
        &self,
        source: usize,
        target: usize,
        potential: &mut [f64],
    ) -> Result<Option<Vec<usize>>> {
        let n = self.outgoing.len();
        let mut dist = vec![f64::INFINITY; n];
        let mut pred = vec![usize::MAX; n];
        let mut heap = BinaryHeap::new();
        dist[source] = 0.0;
        heap.push((Reverse(OrderedF64(0.0)), source));
        while let Some((Reverse(OrderedF64(d)), vertex)) = heap.pop() {
            if d > dist[vertex] {
                continue;
            }
            for &arc_index in &self.outgoing[vertex] {
                let arc = &self.arcs[arc_index];
                if !arc.residual {
                    continue;
                }
                let mut reduced = arc.cost + potential[vertex] - potential[arc.head];
                if reduced < 0.0 && reduced > -1e-10 {
                    reduced = 0.0;
                }
                if reduced.is_nan() || reduced < 0.0 {
                    return Err(GraphError::InvalidArgument(
                        "Suurballe requires non-negative edge weights".into(),
                    ));
                }
                let candidate = d + reduced;
                if candidate < dist[arc.head] {
                    dist[arc.head] = candidate;
                    pred[arc.head] = arc_index;
                    heap.push((Reverse(OrderedF64(candidate)), arc.head));
                }
            }
        }
        if !dist[target].is_finite() {
            return Ok(None);
        }
        for (vertex, &distance) in dist.iter().enumerate() {
            if distance.is_finite() {
                potential[vertex] += distance;
            }
        }
        Ok(Some(pred))
    }

    fn bellman_ford(&self, source: usize, target: usize) -> Result<Option<Vec<usize>>> {
        let n = self.outgoing.len();
        let mut dist = vec![f64::INFINITY; n];
        let mut pred = vec![usize::MAX; n];
        dist[source] = 0.0;
        for round in 0..n {
            let mut changed = false;
            for (arc_index, arc) in self.arcs.iter().enumerate() {
                if !arc.residual || !dist[arc.tail].is_finite() {
                    continue;
                }
                let candidate = dist[arc.tail] + arc.cost;
                if candidate < dist[arc.head] {
                    dist[arc.head] = candidate;
                    pred[arc.head] = arc_index;
                    changed = true;
                    if round + 1 == n {
                        return Err(GraphError::NegativeCycle);
                    }
                }
            }
            if !changed {
                break;
            }
        }
        Ok(dist[target].is_finite().then_some(pred))
    }
}

#[derive(Clone, Copy)]
enum SearchKind {
    Suurballe,
    Bhandari,
}

fn validate<G: DirectedGraph>(
    graph: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
) -> Result<()> {
    if k == 0 {
        return Err(GraphError::InvalidArgument(
            "number of paths must be positive".into(),
        ));
    }
    if source == sink {
        return Err(GraphError::InvalidArgument(
            "source and target must differ".into(),
        ));
    }
    if !graph.contains_vertex(source) || !graph.contains_vertex(sink) {
        return Err(GraphError::VertexNotFound);
    }
    if !props::is_simple(graph) {
        return Err(GraphError::InvalidArgument("graph must be simple".into()));
    }
    Ok(())
}

fn get_paths<G: DirectedGraph>(
    graph: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
    kind: SearchKind,
) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
    validate(graph, source, sink, k)?;
    let indexed = Indexed::new(graph);
    let s = indexed.idx[source];
    let t = indexed.idx[sink];
    let mut residual = ResidualNetwork::new(indexed.n());
    for (edge, &(tail, head)) in indexed.ends.iter().enumerate() {
        let cost = indexed.w[edge];
        if matches!(kind, SearchKind::Suurballe) && (cost.is_nan() || cost < 0.0) {
            return Err(GraphError::InvalidArgument(
                "Suurballe requires non-negative edge weights".into(),
            ));
        }
        residual.add_arc(tail, head, cost, Some(edge));
    }

    let mut potential = vec![0.0; indexed.n()];
    let mut flow = 0;
    while flow < k {
        let predecessor = match kind {
            SearchKind::Suurballe => residual.dijkstra(s, t, &mut potential)?,
            SearchKind::Bhandari => residual.bellman_ford(s, t)?,
        };
        let Some(predecessor) = predecessor else {
            break;
        };
        residual.augment(s, t, &predecessor);
        flow += 1;
    }

    // A saturated original forward arc carries one unit of final flow. Decompose the integral
    // flow into source-target paths; DFS avoids accidentally following a detached flow cycle.
    let mut selected = vec![false; indexed.es.len()];
    for arc in residual.arcs.iter().filter(|arc| arc.forward) {
        if !arc.residual {
            if let Some(edge) = arc.original_edge {
                selected[edge] = true;
            }
        }
    }
    let mut adjacency = vec![Vec::new(); indexed.n()];
    for (edge, &enabled) in selected.iter().enumerate() {
        if enabled {
            adjacency[indexed.ends[edge].0].push(edge);
        }
    }

    fn find_path(
        vertex: usize,
        target: usize,
        adjacency: &[Vec<usize>],
        ends: &[(usize, usize)],
        selected: &[bool],
        seen: &mut [bool],
        path: &mut Vec<usize>,
    ) -> bool {
        if vertex == target {
            return true;
        }
        seen[vertex] = true;
        for &edge in &adjacency[vertex] {
            if !selected[edge] {
                continue;
            }
            let next = ends[edge].1;
            if seen[next] {
                continue;
            }
            path.push(edge);
            if find_path(next, target, adjacency, ends, selected, seen, path) {
                return true;
            }
            path.pop();
        }
        false
    }

    let mut result = Vec::new();
    for _ in 0..flow {
        let mut edge_path = Vec::new();
        let mut seen = vec![false; indexed.n()];
        if !find_path(
            s,
            t,
            &adjacency,
            &indexed.ends,
            &selected,
            &mut seen,
            &mut edge_path,
        ) {
            break;
        }
        for &edge in &edge_path {
            selected[edge] = false;
        }
        let mut vertices = vec![indexed.vertex(s)];
        vertices.extend(
            edge_path
                .iter()
                .map(|&edge| indexed.vertex(indexed.ends[edge].1)),
        );
        let weight = edge_path.iter().map(|&edge| indexed.w[edge]).sum();
        result.push(GraphWalk::new(
            vertices,
            edge_path
                .iter()
                .map(|&edge| indexed.es[edge].clone())
                .collect(),
            weight,
        ));
    }
    result.sort_by(|left, right| left.weight.total_cmp(&right.weight));
    Ok(result)
}

/// Compatibility implementation for the former vertex-disjoint helper. Splitting every internal
/// vertex gives it unit capacity; successive reduced-cost Dijkstra searches then use the same
/// Suurballe residual transformation as the edge-disjoint class.
pub(crate) fn vertex_disjoint_paths<G: DirectedGraph>(
    graph: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
    validate(graph, source, sink, k)?;
    let indexed = Indexed::new(graph);
    let s0 = indexed.idx[source];
    let t0 = indexed.idx[sink];
    let mut residual = ResidualNetwork::new(2 * indexed.n());
    for vertex in 0..indexed.n() {
        if vertex != s0 && vertex != t0 {
            residual.add_arc(2 * vertex, 2 * vertex + 1, 0.0, None);
        }
    }
    for (edge, &(tail, head)) in indexed.ends.iter().enumerate() {
        let weight = indexed.w[edge];
        if weight.is_nan() || weight < 0.0 {
            return Err(GraphError::InvalidArgument(
                "vertex-disjoint shortest paths require non-negative edge weights".into(),
            ));
        }
        residual.add_arc(2 * tail + 1, 2 * head, weight, Some(edge));
    }
    let source_node = 2 * s0 + 1;
    let target_node = 2 * t0;
    let mut potential = vec![0.0; 2 * indexed.n()];
    let mut flow = 0;
    while flow < k {
        let Some(predecessor) = residual.dijkstra(source_node, target_node, &mut potential)? else {
            break;
        };
        residual.augment(source_node, target_node, &predecessor);
        flow += 1;
    }
    let mut selected = vec![false; indexed.es.len()];
    for arc in residual
        .arcs
        .iter()
        .filter(|arc| arc.forward && !arc.residual)
    {
        if let Some(edge) = arc.original_edge {
            selected[edge] = true;
        }
    }
    let mut adjacency = vec![Vec::new(); indexed.n()];
    for (edge, &enabled) in selected.iter().enumerate() {
        if enabled {
            adjacency[indexed.ends[edge].0].push(edge);
        }
    }
    fn extract(
        vertex: usize,
        target: usize,
        adjacency: &[Vec<usize>],
        ends: &[(usize, usize)],
        enabled: &[bool],
        seen: &mut [bool],
        path: &mut Vec<usize>,
    ) -> bool {
        if vertex == target {
            return true;
        }
        seen[vertex] = true;
        for &edge in &adjacency[vertex] {
            let next = ends[edge].1;
            if enabled[edge] && !seen[next] {
                path.push(edge);
                if extract(next, target, adjacency, ends, enabled, seen, path) {
                    return true;
                }
                path.pop();
            }
        }
        false
    }
    let mut paths = Vec::new();
    for _ in 0..flow {
        let mut edges = Vec::new();
        if !extract(
            s0,
            t0,
            &adjacency,
            &indexed.ends,
            &selected,
            &mut vec![false; indexed.n()],
            &mut edges,
        ) {
            break;
        }
        for &edge in &edges {
            selected[edge] = false;
        }
        let mut vertices = vec![indexed.vertex(s0)];
        vertices.extend(
            edges
                .iter()
                .map(|&edge| indexed.vertex(indexed.ends[edge].1)),
        );
        let weight = edges.iter().map(|&edge| indexed.w[edge]).sum();
        paths.push(GraphWalk::new(
            vertices,
            edges.iter().map(|&edge| indexed.es[edge].clone()).collect(),
            weight,
        ));
    }
    paths.sort_by(|left, right| left.weight.total_cmp(&right.weight));
    Ok(paths)
}

/// Suurballe's repeated reweight-and-reverse algorithm for edge-disjoint paths.
pub struct SuurballeKDisjointShortestPaths<'g, G> {
    graph: &'g G,
}

impl<'g, G: DirectedGraph> SuurballeKDisjointShortestPaths<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self { graph }
    }

    pub fn get_paths(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
        k: usize,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        get_paths(self.graph, source, sink, k, SearchKind::Suurballe)
    }
}

/// Bhandari's repeated Bellman-Ford/reversed-edge algorithm for edge-disjoint paths.
pub struct BhandariKDisjointShortestPaths<'g, G> {
    graph: &'g G,
}

impl<'g, G: DirectedGraph> BhandariKDisjointShortestPaths<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self { graph }
    }

    pub fn get_paths(
        &self,
        source: &G::Vertex,
        sink: &G::Vertex,
        k: usize,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        get_paths(self.graph, source, sink, k, SearchKind::Bhandari)
    }
}
