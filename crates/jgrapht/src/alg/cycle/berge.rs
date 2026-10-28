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

use crate::alg::connectivity::ConnectivityInspector;
use crate::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
use crate::error::{GraphError, Result};
use crate::graph::OutgoingGraph;
use crate::graph::capability::IncidenceGraph;
use crate::graph::{DefaultEdge, Graph, GraphExt, GraphMut, GraphMutExt, UGraph};
use crate::view::AsSubgraph;
use crate::walk::GraphWalk;
use indexmap::IndexSet;
use std::collections::{BTreeSet, HashMap};

/// The graph under test as a graph on `0..n` (vertex order and edge order are kept, so that
/// Dijkstra breaks ties as it does on the original graph).
type H = UGraph<usize>;
type Walk = GraphWalk<usize, DefaultEdge>;
type VSet = BTreeSet<usize>;

fn ce<S: Graph<Vertex = usize>>(g: &S, a: usize, b: usize) -> bool {
    g.contains_edge_between(&a, &b)
}

fn weight<S: Graph<Vertex = usize, Edge = DefaultEdge>>(g: &S, edges: &[DefaultEdge]) -> f64 {
    edges.iter().map(|e| g.weight_of(e)).sum()
}

fn edge_between<S: Graph<Vertex = usize, Edge = DefaultEdge>>(
    g: &S,
    a: usize,
    b: usize,
) -> DefaultEdge {
    *g.get_edge(&a, &b).expect("edge between adjacent vertices")
}

/// The complement graph (`ComplementGraphGenerator`): an edge for every non-adjacent pair.
fn complement(g: &H) -> H {
    let mut c = H::simple().weighted();
    let n = g.vertex_count();
    c.add_vertices(0..n);
    for i in 0..n.saturating_sub(1) {
        for j in i + 1..n {
            if !ce(g, i, j) {
                c.add_edge(&i, &j).expect("fresh pair");
            }
        }
    }
    c
}

fn induced<'a, S: Graph<Vertex = usize, Edge = DefaultEdge>>(
    g: &'a S,
    set: &VSet,
) -> AsSubgraph<&'a S> {
    AsSubgraph::induced(g, set.iter().copied())
}

fn shortest_path<S: Graph<Vertex = usize, Edge = DefaultEdge> + OutgoingGraph>(
    g: &S,
    a: usize,
    b: usize,
) -> Option<Walk> {
    Dijkstra::new(g).path(&a, &b).ok().flatten()
}

fn shortest_path_in<S: Graph<Vertex = usize, Edge = DefaultEdge> + OutgoingGraph>(
    g: &S,
    set: &VSet,
    a: usize,
    b: usize,
) -> Option<Walk> {
    let sub = induced(g, set);
    shortest_path(&sub, a, b)
}

fn connected_sets<S: Graph<Vertex = usize> + IncidenceGraph>(g: &S) -> Vec<VSet> {
    ConnectivityInspector::new()
        .connected_sets(g)
        .into_iter()
        .map(|s| s.into_iter().collect())
        .collect()
}

/// The vertex list of a walk given by a start vertex and edges of `g`.
fn walk_vertices<S: Graph<Vertex = usize, Edge = DefaultEdge>>(
    g: &S,
    start: usize,
    edges: &[DefaultEdge],
) -> Vec<usize> {
    let mut vs = vec![start];
    let mut cur = start;
    for e in edges {
        cur = *g.opposite_vertex(e, &cur).expect("consecutive edges");
        vs.push(cur);
    }
    vs
}

/// A certificate: a closed vertex list of the graph or of its complement.
struct Core {
    certify: bool,
    certificate: Option<(Vec<usize>, bool)>,
    on_complement: bool,
}

impl Core {
    fn record<S: Graph<Vertex = usize, Edge = DefaultEdge>>(
        &mut self,
        g: &S,
        start: usize,
        edges: &[DefaultEdge],
    ) {
        self.certificate = Some((walk_vertices(g, start, edges), self.on_complement));
    }

    fn intersect_graph_paths(p1: &Walk, p2: &Walk) -> Vec<usize> {
        p1.vertex_list()
            .iter()
            .copied()
            .filter(|v| p2.vertex_list().contains(v))
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn p<S: Graph<Vertex = usize, Edge = DefaultEdge>>(
        g: &S,
        path_s: Option<&Walk>,
        path_t: Option<&Walk>,
        m: usize,
        b1: usize,
        b2: usize,
        b3: usize,
        s1: usize,
        s2: usize,
        s3: usize,
    ) -> Option<Walk> {
        if s1 == b1 {
            if b1 == m {
                Some(Walk::new_unchecked(vec![s1], Vec::new(), 0.0))
            } else {
                None
            }
        } else {
            if b1 == m {
                return None;
            }
            let bad = |t: usize| ce(g, t, b2) || ce(g, t, b3) || ce(g, t, s2) || ce(g, t, s3);
            if ce(g, m, b2) || ce(g, m, b3) || ce(g, m, s2) || ce(g, m, s3) {
                return None;
            }
            let (path_s, path_t) = (path_s?, path_t?);
            if path_s.vertex_list().iter().any(|&t| bad(t))
                || path_t.vertex_list().iter().any(|&t| t != b1 && bad(t))
            {
                return None;
            }
            let intersection = Self::intersect_graph_paths(path_s, path_t);
            if intersection.len() != 1 || !intersection.contains(&m) {
                return None;
            }
            if path_s
                .vertex_list()
                .iter()
                .any(|&s| s != m && path_t.vertex_list().iter().any(|&t| t != m && ce(g, s, t)))
            {
                return None;
            }
            let mut edges: Vec<DefaultEdge> = path_t.edge_list().to_vec();
            edges.extend_from_slice(path_s.edge_list());
            let mut vertices = path_t.vertex_list().to_vec();
            vertices.extend_from_slice(&path_s.vertex_list()[1..]);
            let w = weight(g, &edges);
            Some(Walk::new_unchecked(vertices, edges, w))
        }
    }

    fn bf_odd_hole_certificate<S: Graph<Vertex = usize, Edge = DefaultEdge> + IncidenceGraph>(
        &mut self,
        g: &S,
    ) {
        let vertices: Vec<usize> = g.vertices_cloned().collect();
        for &start in &vertices {
            if g.degree_of(&start) < 2 {
                continue;
            }
            let mut set: VSet = vertices.iter().copied().collect();
            for &neighbor_of_start in &vertices {
                if neighbor_of_start == start
                    || !ce(g, start, neighbor_of_start)
                    || g.degree_of(&neighbor_of_start) != 2
                {
                    continue;
                }
                set.remove(&neighbor_of_start);
                let subg = induced(g, &set);
                for &neighbors_neighbor in &vertices {
                    if neighbors_neighbor == start
                        || neighbors_neighbor == neighbor_of_start
                        || !ce(g, neighbors_neighbor, neighbor_of_start)
                        || ce(g, neighbors_neighbor, start)
                        || g.degree_of(&neighbors_neighbor) < 2
                    {
                        continue;
                    }
                    let Some(path) = shortest_path(&subg, start, neighbors_neighbor) else {
                        continue;
                    };
                    if path.length() < 3 || path.length() % 2 == 0 {
                        continue;
                    }
                    let mut edge_list: Vec<DefaultEdge> = path.edge_list().to_vec();
                    edge_list.push(edge_between(g, neighbors_neighbor, neighbor_of_start));
                    edge_list.push(edge_between(g, neighbor_of_start, start));
                    self.record(g, start, &edge_list);
                    break;
                }
                if self.certificate.is_some() {
                    break;
                }
            }
            if self.certificate.is_some() {
                break;
            }
        }
    }

    /// Whether `g` contains a pyramid.
    fn contains_pyramid(&mut self, g: &H) -> bool {
        let vertices: Vec<usize> = g.vertices_cloned().collect();
        let mut visited_triangles: IndexSet<VSet> = IndexSet::new();
        for &b1 in &vertices {
            for &b2 in &vertices {
                if b1 == b2 || !ce(g, b1, b2) {
                    continue;
                }
                for &b3 in &vertices {
                    if b3 == b1 || b3 == b2 || !ce(g, b2, b3) || !ce(g, b1, b3) {
                        continue;
                    }
                    let triangle: VSet = [b1, b2, b3].into_iter().collect();
                    if !visited_triangles.insert(triangle) {
                        continue;
                    }
                    for &a_candidate in &vertices {
                        if a_candidate == b1
                            || a_candidate == b2
                            || a_candidate == b3
                            || (ce(g, a_candidate, b1) && ce(g, a_candidate, b2))
                            || (ce(g, a_candidate, b2) && ce(g, a_candidate, b3))
                            || (ce(g, a_candidate, b1) && ce(g, a_candidate, b3))
                        {
                            continue;
                        }
                        for &s1 in &vertices {
                            if s1 == a_candidate
                                || !ce(g, s1, a_candidate)
                                || s1 == b2
                                || s1 == b3
                                || (s1 != b1 && (ce(g, s1, b2) || ce(g, s1, b3)))
                            {
                                continue;
                            }
                            for &s2 in &vertices {
                                if s2 == a_candidate
                                    || !ce(g, s2, a_candidate)
                                    || ce(g, s1, s2)
                                    || s1 == s2
                                    || s2 == b1
                                    || s2 == b3
                                    || (s2 != b2 && (ce(g, s2, b1) || ce(g, s2, b3)))
                                {
                                    continue;
                                }
                                for &s3 in &vertices {
                                    if s3 == a_candidate
                                        || !ce(g, s3, a_candidate)
                                        || ce(g, s3, s2)
                                        || s1 == s3
                                        || s3 == s2
                                        || ce(g, s1, s3)
                                        || s3 == b1
                                        || s3 == b2
                                        || (s3 != b3 && (ce(g, s3, b1) || ce(g, s3, b2)))
                                    {
                                        continue;
                                    }
                                    if self.pyramid_with(
                                        g,
                                        &vertices,
                                        b1,
                                        b2,
                                        b3,
                                        s1,
                                        s2,
                                        s3,
                                        a_candidate,
                                    ) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn pyramid_with(
        &mut self,
        g: &H,
        vertices: &[usize],
        b1: usize,
        b2: usize,
        b3: usize,
        s1: usize,
        s2: usize,
        s3: usize,
        a_candidate: usize,
    ) -> bool {
        let mut set_m: VSet = vertices.iter().copied().collect();
        for v in [b1, b2, b3, s1, s2, s3] {
            set_m.remove(&v);
        }
        // for the apex-side/base-side paths of each of the three legs
        let leg = |blocked: [usize; 4], s: usize, b: usize| {
            let mut map_s: HashMap<usize, Option<Walk>> = HashMap::new();
            let mut map_t: HashMap<usize, Option<Walk>> = HashMap::new();
            for &m in &set_m {
                let mut valid_interior: VSet = set_m.clone();
                valid_interior.retain(|&i| !blocked.iter().any(|&x| ce(g, i, x)));
                valid_interior.insert(m);
                valid_interior.insert(s);
                map_s.insert(m, shortest_path_in(g, &valid_interior, m, s));
                valid_interior.remove(&s);
                valid_interior.insert(b);
                map_t.insert(m, shortest_path_in(g, &valid_interior, b, m));
            }
            (map_s, map_t)
        };
        let (map_s1, map_t1) = leg([b2, s2, b3, s3], s1, b1);
        let (map_s2, map_t2) = leg([b1, s1, b3, s3], s2, b2);
        let (map_s3, map_t3) = leg([b1, s1, b2, s2], s3, b3);
        let mut set_m1 = set_m.clone();
        set_m1.insert(b1);
        for &m1 in &set_m1 {
            fn get(m: &HashMap<usize, Option<Walk>>, k: usize) -> Option<&Walk> {
                m.get(&k).and_then(|o| o.as_ref())
            }
            let Some(path_p1) = Self::p(
                g,
                get(&map_s1, m1),
                get(&map_t1, m1),
                m1,
                b1,
                b2,
                b3,
                s1,
                s2,
                s3,
            ) else {
                continue;
            };
            for &m2 in &set_m {
                let Some(path_p2) = Self::p(
                    g,
                    get(&map_s2, m2),
                    get(&map_t2, m2),
                    m2,
                    b2,
                    b1,
                    b3,
                    s2,
                    s1,
                    s3,
                ) else {
                    continue;
                };
                let mut set_m3 = set_m.clone();
                set_m3.insert(b3);
                for &m3 in &set_m3 {
                    let Some(path_p3) = Self::p(
                        g,
                        get(&map_s3, m3),
                        get(&map_t3, m3),
                        m3,
                        b3,
                        b1,
                        b2,
                        s3,
                        s1,
                        s2,
                    ) else {
                        continue;
                    };
                    if self.certify {
                        let (first, second) = if (path_p1.length() + path_p2.length()) % 2 == 0 {
                            (&path_p1, &path_p2)
                        } else if (path_p1.length() + path_p3.length()) % 2 == 0 {
                            (&path_p1, &path_p3)
                        } else {
                            (&path_p3, &path_p2)
                        };
                        let mut set: VSet = first.vertex_list().iter().copied().collect();
                        set.extend(second.vertex_list().iter().copied());
                        set.insert(a_candidate);
                        let sub = induced(g, &set);
                        self.bf_odd_hole_certificate(&sub);
                    }
                    return true;
                }
            }
        }
        false
    }

    fn find_all_components<S: Graph<Vertex = usize> + IncidenceGraph>(
        g: &S,
        f: &VSet,
    ) -> Vec<VSet> {
        connected_sets(&AsSubgraph::induced(g, f.iter().copied()))
    }

    fn has_a_neighbour(g: &H, set: &VSet, v: usize) -> bool {
        set.iter().any(|&s| ce(g, s, v))
    }

    fn contains_jewel(&mut self, g: &H) -> bool {
        let vertices: Vec<usize> = g.vertices_cloned().collect();
        for &v2 in &vertices {
            for &v3 in &vertices {
                if v2 == v3 || !ce(g, v2, v3) {
                    continue;
                }
                for &v5 in &vertices {
                    if v2 == v5 || v3 == v5 {
                        continue;
                    }
                    let mut set_f = VSet::new();
                    for &f in &vertices {
                        if f == v2
                            || f == v3
                            || f == v5
                            || ce(g, f, v2)
                            || ce(g, f, v3)
                            || ce(g, f, v5)
                        {
                            continue;
                        }
                        set_f.insert(f);
                    }
                    let components_of_f = Self::find_all_components(g, &set_f);
                    let mut set_x1 = VSet::new();
                    for &x1 in &vertices {
                        if x1 == v2
                            || x1 == v3
                            || x1 == v5
                            || !ce(g, x1, v2)
                            || !ce(g, x1, v5)
                            || ce(g, x1, v3)
                        {
                            continue;
                        }
                        set_x1.insert(x1);
                    }
                    let mut set_x2 = VSet::new();
                    for &x2 in &vertices {
                        if x2 == v2
                            || x2 == v3
                            || x2 == v5
                            || ce(g, x2, v2)
                            || !ce(g, x2, v5)
                            || !ce(g, x2, v3)
                        {
                            continue;
                        }
                        set_x2.insert(x2);
                    }
                    for &v1 in &set_x1 {
                        if ce(g, v1, v3) {
                            continue;
                        }
                        for &v4 in &set_x2 {
                            if v1 == v4 || ce(g, v1, v4) || ce(g, v2, v4) {
                                continue;
                            }
                            for f_prime in &components_of_f {
                                if Self::has_a_neighbour(g, f_prime, v1)
                                    && Self::has_a_neighbour(g, f_prime, v4)
                                {
                                    if self.certify {
                                        let mut valid_set = f_prime.clone();
                                        valid_set.insert(v1);
                                        valid_set.insert(v4);
                                        let p = shortest_path_in(g, &valid_set, v1, v4)
                                            .expect("v1 and v4 are linked through f'");
                                        let mut edge_list: Vec<DefaultEdge> =
                                            p.edge_list().to_vec();
                                        if p.length() % 2 == 1 {
                                            edge_list.push(edge_between(g, v4, v5));
                                            edge_list.push(edge_between(g, v5, v1));
                                        } else {
                                            edge_list.push(edge_between(g, v4, v3));
                                            edge_list.push(edge_between(g, v3, v2));
                                            edge_list.push(edge_between(g, v2, v1));
                                        }
                                        self.record(g, v1, &edge_list);
                                    }
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn contains_clean_shortest_odd_hole(&mut self, g: &H) -> bool {
        let vertices: Vec<usize> = g.vertices_cloned().collect();
        for &u in &vertices {
            for &v in &vertices {
                if u == v || ce(g, u, v) {
                    continue;
                }
                let Some(puv) = shortest_path(g, u, v) else {
                    continue;
                };
                for &w in &vertices {
                    if w == u || w == v || ce(g, w, u) || ce(g, w, v) {
                        continue;
                    }
                    let Some(pvw) = shortest_path(g, v, w) else {
                        continue;
                    };
                    let Some(pwu) = shortest_path(g, w, u) else {
                        continue;
                    };
                    let mut set: VSet = puv.vertex_list().iter().copied().collect();
                    set.extend(pvw.vertex_list().iter().copied());
                    set.extend(pwu.vertex_list().iter().copied());
                    let subg = induced(g, &set);
                    if set.len() < 7
                        || subg.vertex_count() != set.len()
                        || subg.edge_count() != subg.vertex_count()
                        || subg.vertex_count() % 2 == 0
                        || subg.vertices().any(|t| subg.degree_of(t) != 2)
                    {
                        continue;
                    }
                    if self.certify {
                        let mut edge_list: Vec<DefaultEdge> = puv.edge_list().to_vec();
                        edge_list.extend_from_slice(pvw.edge_list());
                        edge_list.extend_from_slice(pwu.edge_list());
                        self.record(g, u, &edge_list);
                    }
                    return true;
                }
            }
        }
        false
    }

    fn get_path_avoiding_x(g: &H, start: usize, end: usize, x: &VSet) -> Option<Walk> {
        let mut vertex_set: VSet = g.vertices_cloned().collect();
        for v in x {
            vertex_set.remove(v);
        }
        vertex_set.insert(start);
        vertex_set.insert(end);
        shortest_path_in(g, &vertex_set, start, end)
    }

    fn contains_shortest_odd_hole(&mut self, g: &H, x: &VSet) -> bool {
        let vertices: Vec<usize> = g.vertices_cloned().collect();
        for &y1 in &vertices {
            if x.contains(&y1) {
                continue;
            }
            for &x1 in &vertices {
                if x1 == y1 {
                    continue;
                }
                let rx1y1 = Self::get_path_avoiding_x(g, x1, y1, x);
                for &x3 in &vertices {
                    if x3 == x1 || x3 == y1 || !ce(g, x1, x3) {
                        continue;
                    }
                    for &x2 in &vertices {
                        if x2 == x3 || x2 == x1 || x2 == y1 || ce(g, x2, x1) || !ce(g, x3, x2) {
                            continue;
                        }
                        let rx2y1 = Self::get_path_avoiding_x(g, x2, y1, x);
                        let (Some(rx1y1), Some(rx2y1)) = (rx1y1.as_ref(), rx2y1.as_ref()) else {
                            continue;
                        };
                        let y2 =
                            rx2y1.vertex_list().iter().copied().find(|&c| {
                                ce(g, y1, c) && c != x1 && c != x2 && c != x3 && c != y1
                            });
                        let Some(y2) = y2 else {
                            continue;
                        };
                        let rx3y1 = Self::get_path_avoiding_x(g, x3, y1, x);
                        let rx3y2 = Self::get_path_avoiding_x(g, x3, y2, x);
                        let rx1y2 = Self::get_path_avoiding_x(g, x1, y2, x);
                        let n = (rx1y1.length() + 1) as f64;
                        if let (Some(rx3y1), Some(rx3y2), Some(rx1y2)) = (&rx3y1, &rx3y2, &rx1y2) {
                            if rx2y1.length() as f64 == n
                                && n == rx1y2.length() as f64
                                && rx3y1.length() as f64 >= n
                                && rx3y2.length() as f64 >= n
                            {
                                if self.certify {
                                    let mut edge_list: Vec<DefaultEdge> =
                                        rx1y1.edge_list().to_vec();
                                    for i in (0..rx2y1.length()).rev() {
                                        edge_list.push(rx2y1.edge_list()[i]);
                                    }
                                    edge_list.push(edge_between(g, x2, x3));
                                    edge_list.push(edge_between(g, x3, x1));
                                    self.record(g, x1, &edge_list);
                                }
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn routine1(&mut self, g: &H, x: &VSet) -> bool {
        self.contains_clean_shortest_odd_hole(g) || self.contains_shortest_odd_hole(g, x)
    }

    fn has_configuration_type1(&mut self, g: &H) -> bool {
        let vertices: Vec<usize> = g.vertices_cloned().collect();
        for &v1 in &vertices {
            let temp = ConnectivityInspector::new()
                .connected_set_of(g, &v1)
                .expect("vertex of the graph");
            let temp: VSet = temp.into_iter().collect();
            for &v2 in &temp {
                if v1 == v2 || !ce(g, v1, v2) {
                    continue;
                }
                for &v3 in &temp {
                    if v3 == v1 || v3 == v2 || !ce(g, v2, v3) || ce(g, v1, v3) {
                        continue;
                    }
                    for &v4 in &temp {
                        if v4 == v1
                            || v4 == v2
                            || v4 == v3
                            || ce(g, v1, v4)
                            || ce(g, v2, v4)
                            || !ce(g, v3, v4)
                        {
                            continue;
                        }
                        for &v5 in &temp {
                            if v5 == v1
                                || v5 == v2
                                || v5 == v3
                                || v5 == v4
                                || ce(g, v2, v5)
                                || ce(g, v3, v5)
                                || !ce(g, v1, v5)
                                || !ce(g, v4, v5)
                            {
                                continue;
                            }
                            if self.certify {
                                let edge_list = vec![
                                    edge_between(g, v1, v2),
                                    edge_between(g, v2, v3),
                                    edge_between(g, v3, v4),
                                    edge_between(g, v4, v5),
                                    edge_between(g, v5, v1),
                                ];
                                self.record(g, v1, &edge_list);
                            }
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn is_yx_complete(g: &H, y: usize, x: &VSet) -> bool {
        x.iter().all(|&t| ce(g, t, y))
    }

    fn find_all_anticomponents_of_y(g: &H, y: &VSet) -> Vec<VSet> {
        let target = complement(g);
        Self::find_all_components(&target, y)
    }

    fn has_configuration_type2(&mut self, g: &H) -> bool {
        let vertices: Vec<usize> = g.vertices_cloned().collect();
        for &v1 in &vertices {
            for &v2 in &vertices {
                if v1 == v2 || !ce(g, v1, v2) {
                    continue;
                }
                for &v3 in &vertices {
                    if v3 == v2 || v1 == v3 || ce(g, v1, v3) || !ce(g, v2, v3) {
                        continue;
                    }
                    for &v4 in &vertices {
                        if v4 == v1
                            || v4 == v2
                            || v4 == v3
                            || ce(g, v4, v2)
                            || ce(g, v4, v1)
                            || !ce(g, v3, v4)
                        {
                            continue;
                        }
                        let temp: VSet = [v1, v2, v4].into_iter().collect();
                        let mut set_y = VSet::new();
                        for &y in &vertices {
                            if Self::is_yx_complete(g, y, &temp) {
                                set_y.insert(y);
                            }
                        }
                        let anticomponents_of_y = Self::find_all_anticomponents_of_y(g, &set_y);
                        for set_x in &anticomponents_of_y {
                            let mut v2v3: VSet = vertices.iter().copied().collect();
                            v2v3.remove(&v2);
                            v2v3.remove(&v3);
                            for v in set_x {
                                v2v3.remove(v);
                            }
                            if !v2v3.contains(&v1) || !v2v3.contains(&v4) {
                                continue;
                            }
                            let Some(path) = shortest_path_in(g, &v2v3, v1, v4) else {
                                continue;
                            };
                            let list_p = path.vertex_list();
                            if !list_p.contains(&v1) || !list_p.contains(&v4) {
                                continue;
                            }
                            let mut cont = true;
                            for &p in list_p {
                                if p != v1
                                    && p != v4
                                    && (ce(g, p, v2)
                                        || ce(g, p, v3)
                                        || Self::is_yx_complete(g, p, set_x))
                                {
                                    cont = false;
                                    break;
                                }
                            }
                            if cont {
                                if self.certify {
                                    let mut edge_list: Vec<DefaultEdge> = Vec::new();
                                    if path.length() % 2 == 0 {
                                        edge_list.push(edge_between(g, v1, v2));
                                        edge_list.push(edge_between(g, v2, v3));
                                        edge_list.push(edge_between(g, v3, v4));
                                        edge_list.extend_from_slice(path.edge_list());
                                    } else {
                                        edge_list.extend_from_slice(path.edge_list());
                                        let x =
                                            *set_x.iter().next().expect("non-empty anticomponent");
                                        edge_list.push(edge_between(g, v4, x));
                                        edge_list.push(edge_between(g, x, v1));
                                    }
                                    self.record(g, v1, &edge_list);
                                }
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn find_maximal_connected_subset(g: &H, set_x: &VSet, v1: usize, v2: usize, v5: usize) -> VSet {
        let mut f_prime: VSet = ConnectivityInspector::new()
            .connected_set_of(g, &v5)
            .expect("vertex of the graph")
            .into_iter()
            .collect();
        f_prime.retain(|&t| {
            !((t != v5 && Self::is_yx_complete(g, t, set_x))
                || v1 == t
                || v2 == t
                || ce(g, v1, t)
                || ce(g, v2, t))
        });
        f_prime
    }

    fn has_a_nonneighbour_in_x(g: &H, v: usize, set_x: &VSet) -> bool {
        set_x.iter().any(|&x| !ce(g, v, x))
    }

    fn has_configuration_type3(&mut self, g: &H) -> bool {
        let vertices: Vec<usize> = g.vertices_cloned().collect();
        for &v1 in &vertices {
            for &v2 in &vertices {
                if v1 == v2 || !ce(g, v1, v2) {
                    continue;
                }
                for &v5 in &vertices {
                    if v1 == v5 || v2 == v5 || ce(g, v1, v5) || ce(g, v2, v5) {
                        continue;
                    }
                    let triple: VSet = [v1, v2, v5].into_iter().collect();
                    let mut set_y = VSet::new();
                    for &y in &vertices {
                        if Self::is_yx_complete(g, y, &triple) {
                            set_y.insert(y);
                        }
                    }
                    let anticomponents = Self::find_all_anticomponents_of_y(g, &set_y);
                    for set_x in &anticomponents {
                        let f_prime = Self::find_maximal_connected_subset(g, set_x, v1, v2, v5);
                        let mut set_f = f_prime.clone();
                        for &x in set_x {
                            if !ce(g, x, v1)
                                && !ce(g, x, v2)
                                && !ce(g, x, v5)
                                && Self::has_a_neighbour(g, &f_prime, x)
                            {
                                set_f.insert(x);
                            }
                        }
                        for &v4 in &vertices {
                            if v4 == v1
                                || v4 == v2
                                || v4 == v5
                                || ce(g, v2, v4)
                                || ce(g, v5, v4)
                                || !ce(g, v1, v4)
                                || !Self::has_a_neighbour(g, &set_f, v4)
                                || !Self::has_a_nonneighbour_in_x(g, v4, set_x)
                                || Self::is_yx_complete(g, v4, set_x)
                            {
                                continue;
                            }
                            for &v3 in &vertices {
                                if v3 == v1
                                    || v3 == v2
                                    || v3 == v4
                                    || v3 == v5
                                    || !ce(g, v2, v3)
                                    || !ce(g, v3, v4)
                                    || !ce(g, v5, v3)
                                    || ce(g, v1, v3)
                                    || !Self::has_a_nonneighbour_in_x(g, v3, set_x)
                                    || Self::is_yx_complete(g, v3, set_x)
                                {
                                    continue;
                                }
                                for &v6 in &set_f {
                                    if v6 == v1
                                        || v6 == v2
                                        || v6 == v3
                                        || v6 == v4
                                        || v6 == v5
                                        || !ce(g, v4, v6)
                                        || ce(g, v1, v6)
                                        || ce(g, v2, v6)
                                        || (ce(g, v5, v6) && !Self::is_yx_complete(g, v6, set_x))
                                    {
                                        continue;
                                    }
                                    let mut vertices_for_pv5v6 = f_prime.clone();
                                    vertices_for_pv5v6.insert(v5);
                                    vertices_for_pv5v6.insert(v6);
                                    for v in [v1, v2, v3, v4] {
                                        vertices_for_pv5v6.remove(&v);
                                    }
                                    let sub = induced(g, &vertices_for_pv5v6);
                                    if ConnectivityInspector::new()
                                        .path_exists(&sub, &v6, &v5)
                                        .unwrap_or(false)
                                    {
                                        if self.certify {
                                            let mut edge_list: Vec<DefaultEdge> = vec![
                                                edge_between(g, v1, v4),
                                                edge_between(g, v4, v6),
                                            ];
                                            let path = shortest_path(g, v6, v5)
                                                .expect("v6 and v5 are linked");
                                            edge_list.extend_from_slice(path.edge_list());
                                            if path.length() % 2 == 1 {
                                                let x = *set_x
                                                    .iter()
                                                    .next()
                                                    .expect("non-empty anticomponent");
                                                edge_list.push(edge_between(g, v5, x));
                                                edge_list.push(edge_between(g, x, v1));
                                            } else {
                                                edge_list.push(edge_between(g, v5, v3));
                                                edge_list.push(edge_between(g, v3, v4));
                                                edge_list.push(edge_between(g, v4, v1));
                                            }
                                            self.record(g, v1, &edge_list);
                                        }
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn routine2(&mut self, g: &H) -> bool {
        self.contains_pyramid(g)
            || self.contains_jewel(g)
            || self.has_configuration_type1(g)
            || self.has_configuration_type2(g)
            || self.has_configuration_type3(g)
    }

    /// The common neighbours of `a` and `b`.
    fn n(g: &H, a: usize, b: usize) -> VSet {
        g.vertices_cloned()
            .filter(|&t| ce(g, t, a) && ce(g, t, b))
            .collect()
    }

    fn r(g: &H, n_ab: &VSet, c: usize) -> usize {
        if Self::is_yx_complete(g, c, n_ab) {
            return 0;
        }
        Self::find_all_anticomponents_of_y(g, n_ab)
            .iter()
            .map(BTreeSet::len)
            .max()
            .expect("at least one anticomponent")
    }

    fn y(g: &H, n_ab: &VSet, c: usize) -> VSet {
        let cutoff = Self::r(g, n_ab, c);
        let mut res = VSet::new();
        for anticomponent in Self::find_all_anticomponents_of_y(g, n_ab) {
            if anticomponent.len() > cutoff {
                res.extend(anticomponent);
            }
        }
        res
    }

    fn w(g: &H, n_ab: &VSet, c: usize) -> VSet {
        let mut temp = n_ab.clone();
        temp.insert(c);
        Self::find_all_anticomponents_of_y(g, &temp)
            .into_iter()
            .find(|a| a.contains(&c))
            .unwrap_or_default()
    }

    fn z(g: &H, n_ab: &VSet, c: usize) -> VSet {
        let mut temp = Self::y(g, n_ab, c);
        temp.extend(Self::w(g, n_ab, c));
        g.vertices_cloned()
            .filter(|&it| Self::is_yx_complete(g, it, &temp))
            .collect()
    }

    fn x(g: &H, n_ab: &VSet, c: usize) -> VSet {
        let mut res = Self::y(g, n_ab, c);
        res.extend(Self::z(g, n_ab, c));
        res
    }

    fn is_triple_relevant(g: &H, a: usize, b: usize, c: usize) -> bool {
        a != b && !ce(g, a, b) && !Self::n(g, a, b).contains(&c)
    }

    fn routine3(g: &H) -> IndexSet<VSet> {
        let vertices: Vec<usize> = g.vertices_cloned().collect();
        let mut n_uv_list: IndexSet<VSet> = IndexSet::new();
        for &u in &vertices {
            for &v in &vertices {
                if u == v || !ce(g, u, v) {
                    continue;
                }
                n_uv_list.insert(Self::n(g, u, v));
            }
        }
        let mut triple_list: IndexSet<VSet> = IndexSet::new();
        for &a in &vertices {
            for &b in &vertices {
                if a == b || ce(g, a, b) {
                    continue;
                }
                let n_ab = Self::n(g, a, b);
                for &c in &vertices {
                    if Self::is_triple_relevant(g, a, b, c) {
                        triple_list.insert(Self::x(g, &n_ab, c));
                    }
                }
            }
        }
        let mut res: IndexSet<VSet> = IndexSet::new();
        for n_uv in &n_uv_list {
            for triple in &triple_list {
                let mut temp = n_uv.clone();
                temp.extend(triple.iter().copied());
                res.insert(temp);
            }
        }
        res
    }
}

/// Tests whether a graph is [perfect](http://mathworld.wolfram.com/PerfectGraph.html). A
/// perfect graph, also known as a Berge graph, is a graph $G$ such that for every induced subgraph
/// of $G$, the clique number $\chi(G)$ equals the chromatic number $\omega(G)$, i.e.,
/// $\omega(G)=\chi(G)$. Another characterization of perfect graphs is given by the Strong Perfect
/// Graph Theorem [M. Chudnovsky, N. Robertson, P. Seymour, R. Thomas. The strong perfect graph
/// theorem Annals of Mathematics, vol 164(1): pp. 51–230, 2006]: A graph $G$ is perfect if neither
/// $G$ nor its complement $\overline{G}$ have an odd hole. A hole in $G$ is an induced subgraph of
/// $G$ that is a cycle of length at least four, and it is odd or even if it has odd (or even,
/// respectively) length.
///
/// Some special [classes](http://graphclasses.org/classes/gc_56.html) of graphs are are
/// known to be perfect, e.g. Bipartite graphs and Chordal graphs. Testing whether a graph is resp.
/// Bipartite or Chordal can be done efficiently using `GraphTests#isBipartite` or
/// `org.jgrapht.alg.cycle.ChordalityInspector`.
///
/// The implementation of this class is based on the paper: M. Chudnovsky, G. Cornuejols, X. Liu, P.
/// Seymour, and K. Vuskovic. Recognizing Berge Graphs. Combinatorica 25(2): 143--186, 2003.
///
/// Special Thanks to Maria Chudnovsky for her kind help.
///
/// The runtime complexity of this implementation is $O(|V|^9|)$. This implementation is far more
/// efficient than simplistically testing whether graph $G$ or its complement $\overline{G}$ have an
/// odd cycle, because testing whether one graph can be found as an induced subgraph of another is
/// [known](https://en.wikipedia.org/wiki/Induced_subgraph_isomorphism_problem) to be
/// NP-hard.
///
/// # Rust notes
///
/// `BergeGraphInspector`: tests whether a graph is a Berge graph (it has neither an odd hole nor
/// an odd antihole) with the Chudnovsky–Cornuéjols–Liu–Seymour–Vušković recognition algorithm,
/// as implemented by JGraphT: pyramids, jewels and three configurations are searched in the graph
/// and its complement, then clean shortest odd holes.
///
/// The methods take the graph as an argument, like the Java class; undirected graphs only. The
/// certificate (an odd hole of the graph or of its complement, i.e. an odd antihole) is available
/// after [`is_berge`](Self::is_berge) with `compute_certificate`.
pub struct BergeGraphInspector<V> {
    core: Core,
    certificate: Option<BergeCertificate<V>>,
}

/// Why a graph is not a Berge graph: an odd hole of the graph or an odd antihole (an odd hole of
/// its complement), as a closed vertex list (the first vertex is repeated at the end).
///
/// Like [`WeakChordalityCertificate`](super::WeakChordalityCertificate) it is a vertex list rather
/// than a [`GraphWalk`](crate::GraphWalk), because an antihole has no edges between consecutive
/// vertices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BergeCertificate<V> {
    /// An odd induced cycle of the graph.
    OddHole(Vec<V>),
    /// An odd induced cycle of the complement graph.
    OddAntihole(Vec<V>),
}

impl<V> BergeCertificate<V> {
    /// The closed vertex list of the cycle.
    pub fn vertex_list(&self) -> &[V] {
        match self {
            Self::OddHole(vertices) | Self::OddAntihole(vertices) => vertices,
        }
    }

    /// Whether the cycle is an odd hole of the graph (rather than an odd antihole).
    pub fn is_hole(&self) -> bool {
        matches!(self, Self::OddHole(_))
    }
}

impl<V> Default for BergeGraphInspector<V> {
    fn default() -> Self {
        BergeGraphInspector {
            core: Core {
                certify: false,
                certificate: None,
                on_complement: false,
            },
            certificate: None,
        }
    }
}

impl<V: Clone + Eq + std::hash::Hash> BergeGraphInspector<V> {
    /// Constructs a new Berge graph inspector.
    pub fn new() -> Self {
        Self::default()
    }

    /// The graph on `0..n` mirroring `g`, and the vertex list of `g`.
    fn convert<G: Graph<Vertex = V>>(g: &G) -> Result<(H, Vec<V>)> {
        if g.graph_type().directed || g.graph_type().mixed {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::Undirected));
        }
        let vs: Vec<V> = g.vertices_cloned().collect();
        let index: HashMap<&V, usize> = vs.iter().enumerate().map(|(i, v)| (v, i)).collect();
        let mut h = H::pseudo().weighted();
        h.add_vertices(0..vs.len());
        for e in g.edges() {
            let (s, t) = g.edge_endpoints(e).expect("edge of the graph");
            let id = h
                .add_edge(&index[s], &index[t])?
                .expect("pseudograph accepts every edge");
            h.set_edge_weight(&id, g.weight_of(e))?;
        }
        Ok((h, vs))
    }

    /// Maps the core's certificate back to the vertices of `g`.
    fn publish<G: Graph<Vertex = V>>(&mut self, _g: &G, vs: &[V]) {
        if let Some((cycle, in_complement)) = &self.core.certificate {
            let vertices: Vec<V> = cycle.iter().map(|&i| vs[i].clone()).collect();
            self.certificate = Some(if *in_complement {
                BergeCertificate::OddAntihole(vertices)
            } else {
                BergeCertificate::OddHole(vertices)
            });
        }
    }

    /// `containsPyramid`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn contains_pyramid<G: Graph<Vertex = V>>(&mut self, g: &G) -> Result<bool> {
        let (h, vs) = Self::convert(g)?;
        self.core.on_complement = false;
        let r = self.core.contains_pyramid(&h);
        self.publish(g, &vs);
        Ok(r)
    }

    /// `containsJewel`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn contains_jewel<G: Graph<Vertex = V>>(&mut self, g: &G) -> Result<bool> {
        let (h, vs) = Self::convert(g)?;
        self.core.on_complement = false;
        let r = self.core.contains_jewel(&h);
        self.publish(g, &vs);
        Ok(r)
    }

    /// `containsCleanShortestOddHole`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn contains_clean_shortest_odd_hole<G: Graph<Vertex = V>>(
        &mut self,
        g: &G,
    ) -> Result<bool> {
        let (h, vs) = Self::convert(g)?;
        self.core.on_complement = false;
        let r = self.core.contains_clean_shortest_odd_hole(&h);
        self.publish(g, &vs);
        Ok(r)
    }

    /// `hasConfigurationType1`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn has_configuration_type1<G: Graph<Vertex = V>>(&mut self, g: &G) -> Result<bool> {
        let (h, vs) = Self::convert(g)?;
        self.core.on_complement = false;
        let r = self.core.has_configuration_type1(&h);
        self.publish(g, &vs);
        Ok(r)
    }

    /// `hasConfigurationType2`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn has_configuration_type2<G: Graph<Vertex = V>>(&mut self, g: &G) -> Result<bool> {
        let (h, vs) = Self::convert(g)?;
        self.core.on_complement = false;
        let r = self.core.has_configuration_type2(&h);
        self.publish(g, &vs);
        Ok(r)
    }

    /// `hasConfigurationType3`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn has_configuration_type3<G: Graph<Vertex = V>>(&mut self, g: &G) -> Result<bool> {
        let (h, vs) = Self::convert(g)?;
        self.core.on_complement = false;
        let r = self.core.has_configuration_type3(&h);
        self.publish(g, &vs);
        Ok(r)
    }

    /// Whether `y` is adjacent to every vertex of `x`.
    pub fn is_yx_complete<G: Graph<Vertex = V>>(
        &self,
        g: &G,
        y: &V,
        x: &std::collections::HashSet<V>,
    ) -> bool {
        x.iter().all(|t| g.contains_edge_between(t, y))
    }

    /// `routine3` of the algorithm: the candidate sets `X` for the shortest odd hole search.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn routine3<G: Graph<Vertex = V>>(&self, g: &G) -> Result<Vec<Vec<V>>> {
        let (h, vs) = Self::convert(g)?;
        Ok(Core::routine3(&h)
            .into_iter()
            .map(|s| s.into_iter().map(|i| vs[i].clone()).collect())
            .collect())
    }

    /// Performs the Berge Recognition Algorithm.
    ///
    /// First this algorithm is used to test whether $G$ or its complement contain a jewel, a pyramid
    /// or a configuration of type 1, 2 or 3. If so, it is output that $G$ is not Berge. If not, then
    /// every shortest odd hole in $G$ is amenable. This asserted, the near-cleaner subsets of $V(G)$
    /// are determined. For each of them in turn it is checked, if this subset is a near-cleaner and,
    /// thus, if there is an odd hole. If an odd hole is found, this checker will output that $G$ is
    /// not Berge. If no odd hole is found, all near-cleaners for the complement graph are determined
    /// and it will be proceeded as before. If again no odd hole is detected, $G$ is Berge.
    ///
    /// A certificate can be obtained through the `BergeGraphInspector#getCertificate` method,
    /// if `computeCertificate` is `true`.
    ///
    /// Running this method takes $O(|V|^9)$, and computing the certificate takes $O(|V|^5)$.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn is_berge<G: Graph<Vertex = V>>(&mut self, g: &G) -> Result<bool> {
        self.is_berge_with_certificate(g, false)
    }

    /// Whether `g` is a Berge graph; with `compute_certificate` an odd hole or odd antihole is
    /// kept for [`get_certificate`](Self::get_certificate) when it is not.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn is_berge_with_certificate<G: Graph<Vertex = V>>(
        &mut self,
        g: &G,
        compute_certificate: bool,
    ) -> Result<bool> {
        let (h, vs) = Self::convert(g)?;
        let comp = complement(&h);
        self.core.certify = compute_certificate;
        let result = self.run_berge(&h, &comp);
        self.core.certify = false;
        self.core.on_complement = false;
        self.publish(g, &vs);
        Ok(result)
    }

    fn run_berge(&mut self, h: &H, comp: &H) -> bool {
        self.core.on_complement = false;
        if self.core.routine2(h) {
            return false;
        }
        self.core.on_complement = true;
        if self.core.routine2(comp) {
            return false;
        }
        self.core.on_complement = false;
        for it in Core::routine3(h) {
            if self.core.routine1(h, &it) {
                return false;
            }
        }
        self.core.on_complement = true;
        for it in Core::routine3(comp) {
            if self.core.routine1(comp, &it) {
                return false;
            }
        }
        true
    }

    /// The odd hole or odd antihole found by the last run with `compute_certificate`, if any.
    pub fn get_certificate(&self) -> Option<&BergeCertificate<V>> {
        self.certificate.as_ref()
    }
}
