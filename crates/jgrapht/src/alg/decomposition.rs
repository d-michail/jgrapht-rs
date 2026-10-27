//! Tree decompositions via elimination-ordering heuristics.

use super::util::Indexed;
use crate::graph::{Graph, GraphExt};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EliminationHeuristic {
    MinDegree,
    MinFill,
}

#[derive(Debug, Clone)]
pub struct TreeDecomposition<V> {
    pub bags: Vec<Vec<V>>,
    /// Edges between bag indices; together they form a tree (or a path of trees' roots).
    pub tree_edges: Vec<(usize, usize)>,
}

impl<V> TreeDecomposition<V> {
    /// Largest bag size minus one: an upper bound on the treewidth.
    pub fn width(&self) -> usize {
        self.bags
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(1)
            .saturating_sub(1)
    }
}

/// Tree decomposition from a greedy elimination ordering (direction ignored).
pub fn tree_decomposition<G: Graph>(
    g: &G,
    heuristic: EliminationHeuristic,
) -> TreeDecomposition<G::Vertex> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut nb: Vec<HashSet<usize>> = (0..n)
        .map(|v| {
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v)
                .collect()
        })
        .collect();
    let mut alive = vec![true; n];
    let mut order_pos = vec![usize::MAX; n];
    let mut bags: Vec<Vec<usize>> = Vec::with_capacity(n);
    let mut bag_nb: Vec<Vec<usize>> = Vec::with_capacity(n);
    for step in 0..n {
        let cost = |v: usize| -> usize {
            match heuristic {
                EliminationHeuristic::MinDegree => nb[v].len(),
                EliminationHeuristic::MinFill => {
                    let l: Vec<_> = nb[v].iter().copied().collect();
                    (0..l.len())
                        .map(|i| {
                            (i + 1..l.len())
                                .filter(|&j| !nb[l[i]].contains(&l[j]))
                                .count()
                        })
                        .sum()
                }
            }
        };
        let v = (0..n)
            .filter(|&v| alive[v])
            .min_by_key(|&v| cost(v))
            .unwrap();
        let neighbors: Vec<usize> = nb[v].iter().copied().collect();
        for &a in &neighbors {
            for &b in &neighbors {
                if a != b {
                    nb[a].insert(b);
                }
            }
            nb[a].remove(&v);
        }
        alive[v] = false;
        order_pos[v] = step;
        let mut bag = vec![v];
        bag.extend(&neighbors);
        bags.push(bag);
        bag_nb.push(neighbors);
    }
    // bag `i` (eliminating vertex order[i]) hangs below the bag of its earliest-eliminated neighbour
    let mut tree_edges = Vec::new();
    let mut roots = Vec::new();
    for i in 0..n {
        match bag_nb[i].iter().min_by_key(|&&u| order_pos[u]) {
            Some(&u) => tree_edges.push((i, order_pos[u])),
            None => roots.push(i),
        }
    }
    for w in roots.windows(2) {
        tree_edges.push((w[0], w[1]));
    }
    TreeDecomposition {
        bags: bags.iter().map(|b| ix.vertices_of(b)).collect(),
        tree_edges,
    }
}

/// Upper bound on the treewidth (min-fill decomposition).
pub fn treewidth_upper_bound<G: Graph>(g: &G) -> usize {
    tree_decomposition(g, EliminationHeuristic::MinFill).width()
}

// ---------------------------------------------------------------------------------------------
// Heavy path decomposition

use super::interfaces::PathDecomposition;
use crate::util::VertexToIntegerMapping;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::hash::Hash;

/// Heavy-light decomposition of a tree or forest into paths (`HeavyPathDecomposition`).
///
/// An edge to a child is *heavy* when the child's subtree holds more than half of the parent's
/// subtree; every root-to-vertex walk crosses `O(log n)` light edges.
pub struct HeavyPathDecomposition<'g, G: Graph> {
    graph: &'g G,
    vertex_map: HashMap<G::Vertex, usize>,
    index_list: Vec<G::Vertex>,
    size_subtree: Vec<usize>,
    parent: Vec<Option<usize>>,
    depth: Vec<Option<usize>>,
    component: Vec<Option<usize>>,
    path: Vec<Option<usize>>,
    position_in_path: Vec<Option<usize>>,
    first_node_in_path: Vec<usize>,
    number_of_paths: usize,
    paths: Vec<Vec<G::Vertex>>,
    heavy_edges: HashSet<G::Edge>,
    light_edges: HashSet<G::Edge>,
}

impl<'g, G: Graph> HeavyPathDecomposition<'g, G> {
    pub fn new(tree: &'g G, root: G::Vertex) -> Result<Self> {
        Self::with_roots(tree, [root])
    }

    /// One root per tree of the forest; every vertex of a tree is reachable from its root.
    pub fn with_roots(forest: &'g G, roots: impl IntoIterator<Item = G::Vertex>) -> Result<Self> {
        let mapping = VertexToIntegerMapping::new(forest.vertices().cloned())?;
        let n = forest.vertex_count();
        let mut d = HeavyPathDecomposition {
            graph: forest,
            vertex_map: mapping.vertex_map().clone(),
            index_list: mapping.index_list().to_vec(),
            size_subtree: vec![0; n],
            parent: vec![None; n],
            depth: vec![None; n],
            component: vec![None; n],
            path: vec![None; n],
            position_in_path: vec![None; n],
            first_node_in_path: Vec::new(),
            number_of_paths: 0,
            paths: Vec::new(),
            heavy_edges: HashSet::new(),
            light_edges: HashSet::new(),
        };
        d.decompose(roots)?;
        Ok(d)
    }

    fn opposite(&self, e: &G::Edge, u: usize) -> usize {
        let v = self
            .graph
            .opposite_vertex(e, &self.index_list[u])
            .expect("incident edge");
        self.vertex_map[v]
    }

    fn dfs_iterative(&mut self, root: usize, c: usize, length_path: &mut Vec<usize>) {
        let mut explored = vec![false; self.index_list.len()];
        let mut stack = vec![root];
        while let Some(u) = stack.pop() {
            if !explored[u] {
                explored[u] = true;
                stack.push(u);
                self.component[u] = Some(c);
                self.size_subtree[u] = 1;
                let vertex_u = self.index_list[u].clone();
                let edges: Vec<G::Edge> = self.graph.edges_of(&vertex_u).cloned().collect();
                for edge in &edges {
                    let child = self.opposite(edge, u);
                    if !explored[child] {
                        self.parent[child] = Some(u);
                        self.depth[child] = Some(self.depth[u].map_or(0, |d| d + 1));
                        stack.push(child);
                    }
                }
            } else {
                let mut path_child: Option<usize> = None;
                let mut path_edge: Option<G::Edge> = None;
                let vertex_u = self.index_list[u].clone();
                let edges: Vec<G::Edge> = self.graph.edges_of(&vertex_u).cloned().collect();
                for edge in &edges {
                    let child = self.opposite(edge, u);
                    if Some(child) != self.parent[u] {
                        self.size_subtree[u] += self.size_subtree[child];
                        if path_child
                            .is_none_or(|pc| self.size_subtree[pc] < self.size_subtree[child])
                        {
                            path_child = Some(child);
                            path_edge = Some(edge.clone());
                        }
                        self.light_edges.insert(edge.clone());
                    }
                }
                match path_child {
                    None => {
                        self.path[u] = Some(self.number_of_paths);
                        self.number_of_paths += 1;
                        length_path.push(0);
                    }
                    Some(pc) => {
                        self.path[u] = self.path[pc];
                        if 2 * self.size_subtree[pc] > self.size_subtree[u] {
                            let pe = path_edge.unwrap();
                            self.light_edges.remove(&pe);
                            self.heavy_edges.insert(pe);
                        }
                    }
                }
                let p = self.path[u].unwrap();
                self.position_in_path[u] = Some(length_path[p]);
                length_path[p] += 1;
            }
        }
    }

    fn decompose(&mut self, roots: impl IntoIterator<Item = G::Vertex>) -> Result<()> {
        let mut length_path: Vec<usize> = Vec::new();
        let mut number_component = 0;
        for root in roots {
            let Some(&u) = self.vertex_map.get(&root) else {
                return Err(GraphError::InvalidArgument(
                    "root not contained in graph".into(),
                ));
            };
            if self.component[u].is_none() {
                self.dfs_iterative(u, number_component, &mut length_path);
                number_component += 1;
            } else {
                return Err(GraphError::InvalidArgument(
                    "multiple roots in the same tree".into(),
                ));
            }
        }
        self.first_node_in_path = vec![0; self.number_of_paths];
        for i in 0..self.index_list.len() {
            if let Some(p) = self.path[i] {
                let pos = length_path[p] - self.position_in_path[i].unwrap() - 1;
                self.position_in_path[i] = Some(pos);
                if pos == 0 {
                    self.first_node_in_path[p] = i;
                }
            }
        }
        let mut paths: Vec<Vec<Option<G::Vertex>>> =
            length_path.iter().map(|&l| vec![None; l]).collect();
        for i in 0..self.index_list.len() {
            if let Some(p) = self.path[i] {
                paths[p][self.position_in_path[i].unwrap()] = Some(self.index_list[i].clone());
            }
        }
        self.paths = paths
            .into_iter()
            .map(|p| p.into_iter().map(Option::unwrap).collect())
            .collect();
        Ok(())
    }

    pub fn heavy_edges(&self) -> &HashSet<G::Edge> {
        &self.heavy_edges
    }
    pub fn light_edges(&self) -> &HashSet<G::Edge> {
        &self.light_edges
    }

    /// The paths as walks (with their vertex count as weight, like JGraphT).
    pub fn path_decomposition(&self) -> Result<PathDecomposition<G::Vertex, G::Edge>> {
        let walks = self
            .paths
            .iter()
            .map(|p| GraphWalk::from_vertex_list(self.graph, p.clone(), p.len() as f64))
            .collect::<Result<Vec<_>>>()?;
        Ok(PathDecomposition::new(self.heavy_edges.clone(), walks))
    }

    /// Parent of `v` in the rooted forest.
    pub fn parent_of(&self, v: &G::Vertex) -> Option<&G::Vertex> {
        let i = *self.vertex_map.get(v)?;
        self.parent[i].map(|p| &self.index_list[p])
    }
    pub fn depth_of(&self, v: &G::Vertex) -> Option<usize> {
        self.depth[*self.vertex_map.get(v)?]
    }
    pub fn subtree_size_of(&self, v: &G::Vertex) -> usize {
        self.vertex_map.get(v).map_or(0, |&i| self.size_subtree[i])
    }
    pub fn component_of(&self, v: &G::Vertex) -> Option<usize> {
        self.component[*self.vertex_map.get(v)?]
    }
    pub fn vertex_map(&self) -> &HashMap<G::Vertex, usize> {
        &self.vertex_map
    }
    pub fn index_list(&self) -> &[G::Vertex] {
        &self.index_list
    }
    /// Path index of the vertex with the given index (`None` for vertices outside the trees).
    pub fn path_array(&self) -> &[Option<usize>] {
        &self.path
    }
    pub fn position_in_path_array(&self) -> &[Option<usize>] {
        &self.position_in_path
    }
    pub fn first_node_in_path_array(&self) -> &[usize] {
        &self.first_node_in_path
    }
    pub fn parent_array(&self) -> &[Option<usize>] {
        &self.parent
    }
    pub fn depth_array(&self) -> &[Option<usize>] {
        &self.depth
    }
    pub fn size_subtree_array(&self) -> &[usize] {
        &self.size_subtree
    }
    pub fn component_array(&self) -> &[Option<usize>] {
        &self.component
    }
}

// ---------------------------------------------------------------------------------------------
// Dulmage–Mendelsohn decomposition

use super::connectivity::strongly_connected_components_kosaraju;
use super::matching::{
    HopcroftKarpMaximumCardinalityBipartiteMatching, Matching, MatchingAlgorithm,
};
use crate::graph::{GraphMut, GraphMutExt};
use crate::view::EdgeReversed;

/// Result of a Dulmage–Mendelsohn decomposition.
#[derive(Debug, Clone)]
pub struct DmDecomposition<V> {
    pub partition1_dominated: HashSet<V>,
    pub partition2_dominated: HashSet<V>,
    /// Sets of the perfectly matched core (several when `fine`).
    pub perfect_matched_sets: Vec<HashSet<V>>,
}

/// Dulmage–Mendelsohn decomposition of a bipartite graph (`DulmageMendelsohnDecomposition`).
pub struct DulmageMendelsohnDecomposition<'g, G: Graph> {
    graph: &'g G,
    partition1: HashSet<G::Vertex>,
    partition2: HashSet<G::Vertex>,
}

impl<'g, G: Graph> DulmageMendelsohnDecomposition<'g, G>
where
    G::Vertex: Eq + Hash,
{
    pub fn new(
        graph: &'g G,
        partition1: HashSet<G::Vertex>,
        partition2: HashSet<G::Vertex>,
    ) -> Self {
        DulmageMendelsohnDecomposition {
            graph,
            partition1,
            partition2,
        }
    }

    /// Decomposes along a maximum matching; `fine` splits the core into its strongly connected parts.
    pub fn decomposition(&self, fine: bool) -> Result<DmDecomposition<G::Vertex>> {
        let matching = HopcroftKarpMaximumCardinalityBipartiteMatching::new(
            self.graph,
            self.partition1.iter().cloned(),
            self.partition2.iter().cloned(),
        )?
        .get_matching()?;
        self.decompose(&matching, fine)
    }

    pub fn decompose(
        &self,
        matching: &Matching<G::Vertex, G::Edge>,
        fine: bool,
    ) -> Result<DmDecomposition<G::Vertex>> {
        let g = self.graph;
        let (mut unmatched1, mut unmatched2) = (self.partition1.clone(), self.partition2.clone());
        for e in matching.edges() {
            let (source, target) = (g.edge_source(e).unwrap(), g.edge_target(e).unwrap());
            if self.partition1.contains(source) {
                unmatched1.remove(source);
                unmatched2.remove(target);
            } else {
                unmatched2.remove(source);
                unmatched1.remove(target);
            }
        }
        let dg = self.as_directed_graph(matching)?;
        let reach = |graph: &dyn Fn(&G::Vertex) -> Vec<G::Vertex>, starts: &HashSet<G::Vertex>| {
            let mut seen: HashSet<G::Vertex> = HashSet::new();
            for s in starts {
                let mut stack = vec![s.clone()];
                while let Some(v) = stack.pop() {
                    if seen.insert(v.clone()) {
                        stack.extend(graph(&v));
                    }
                }
            }
            seen
        };
        let forward = |v: &G::Vertex| {
            dg.out_edges_of(v)
                .map(|e| dg.edge_target(e).unwrap().clone())
                .collect()
        };
        let subset1 = reach(&forward, &unmatched1);
        let rev = EdgeReversed(&dg);
        let backward = |v: &G::Vertex| {
            rev.out_edges_of(v)
                .map(|e| rev.edge_target(e).unwrap().clone())
                .collect()
        };
        let subset2 = reach(&backward, &unmatched2);
        let subset3: HashSet<G::Vertex> = self
            .partition1
            .iter()
            .chain(&self.partition2)
            .filter(|v| !subset1.contains(*v) && !subset2.contains(*v))
            .cloned()
            .collect();
        let sets = if fine {
            let graph_h = self.as_directed_edge_graph(matching, &subset3)?;
            strongly_connected_components_kosaraju(&graph_h)
                .into_iter()
                .map(|edge_set| {
                    edge_set
                        .iter()
                        .flat_map(|e| {
                            [
                                g.edge_source(e).unwrap().clone(),
                                g.edge_target(e).unwrap().clone(),
                            ]
                        })
                        .collect()
                })
                .collect()
        } else {
            vec![subset3]
        };
        Ok(DmDecomposition {
            partition1_dominated: subset1,
            partition2_dominated: subset2,
            perfect_matched_sets: sets,
        })
    }

    /// Edges oriented from partition 1 to partition 2, plus matching edges back.
    fn as_directed_graph(&self, matching: &Matching<G::Vertex, G::Edge>) -> Result<crate::DiGraph<G::Vertex>> {
        let g = self.graph;
        let matched: HashSet<&G::Edge> = matching.edges().iter().collect();
        let mut dg = crate::DiGraph::<G::Vertex>::with_graph_type(crate::GraphType {
            self_loops: true,
            ..crate::GraphType::simple()
        });
        dg.add_vertices(g.vertices().cloned());
        for e in g.edges() {
            let (v1, v2) = (g.edge_source(e).unwrap(), g.edge_target(e).unwrap());
            if self.partition1.contains(v1) {
                dg.add_edge(v1, v2)?;
                if matched.contains(e) {
                    dg.add_edge(v2, v1)?;
                }
            } else {
                dg.add_edge(v2, v1)?;
                if matched.contains(e) {
                    dg.add_edge(v1, v2)?;
                }
            }
        }
        Ok(dg)
    }

    /// Graph on the edges of the core: matching edges are vertices, non-matching edges link the
    /// matching edges of their endpoints.
    fn as_directed_edge_graph(
        &self,
        matching: &Matching<G::Vertex, G::Edge>,
        subset: &HashSet<G::Vertex>,
    ) -> Result<crate::DiGraph<G::Edge>> {
        let g = self.graph;
        let matched: HashSet<&G::Edge> = matching.edges().iter().collect();
        let mut h = crate::DiGraph::<G::Edge>::with_graph_type(crate::GraphType {
            self_loops: true,
            ..crate::GraphType::simple()
        });
        for e in g.edges() {
            let (v1, v2) = (g.edge_source(e).unwrap(), g.edge_target(e).unwrap());
            if subset.contains(v1) && subset.contains(v2) {
                if matched.contains(e) {
                    h.add_vertex(e.clone());
                } else {
                    let find =
                        |v: &G::Vertex| g.edges_of(v).find(|o| matched.contains(*o)).cloned();
                    let (e1, e2) = (find(v1), find(v2));
                    if let (Some(e1), Some(e2)) = (e1, e2) {
                        h.add_vertex(e1.clone());
                        h.add_vertex(e2.clone());
                        h.add_edge(&e1, &e2)?;
                    }
                }
            }
        }
        Ok(h)
    }
}
