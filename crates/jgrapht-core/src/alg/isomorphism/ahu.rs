//! Aho–Hopcroft–Ullman tree isomorphism, linear in the number of vertices.

use super::IsomorphismInspector;
use super::mapping::IsomorphicGraphMapping;
use crate::error::{GraphError, Result};
use crate::graph::props::{is_simple, require_undirected};
use crate::graph::{Graph, GraphExt};
use crate::util::radix_sort;
use indexmap::IndexMap;
use std::cell::OnceCell;
use std::collections::{HashMap, HashSet, VecDeque};

const NONE: usize = usize::MAX;

/// Vertices in graph order and, per vertex, the vertices at the far end of its outgoing edges.
fn adjacency<G: Graph>(g: &G) -> (Vec<&G::Vertex>, Vec<Vec<usize>>) {
    let vs: Vec<&G::Vertex> = g.vertices().collect();
    let index: HashMap<&G::Vertex, usize> = vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
    let adj = vs
        .iter()
        .map(|v| g.outgoing_of(v).map(|(_, w)| index[w]).collect())
        .collect();
    (vs, adj)
}

/// BFS levels from `root`: `levels[d]` lists the vertices at depth `d` in visiting order.
fn levels(adj: &[Vec<usize>], root: usize) -> Vec<Vec<usize>> {
    let mut depth = vec![NONE; adj.len()];
    let mut levels: Vec<Vec<usize>> = Vec::new();
    let mut queue = VecDeque::from([root]);
    depth[root] = 0;
    while let Some(u) = queue.pop_front() {
        if levels.len() <= depth[u] {
            levels.push(Vec::new());
        }
        levels[depth[u]].push(u);
        for &v in &adj[u] {
            if depth[v] == NONE {
                depth[v] = depth[u] + 1;
                queue.push_back(v);
            }
        }
    }
    levels
}

/// Pairs vertices with equal canonical names, breadth first from the roots, or `None` when a
/// child cannot be paired.
fn match_vertices_with_same_label(
    adj1: &[Vec<usize>],
    adj2: &[Vec<usize>],
    root1: usize,
    root2: usize,
    name1: &[Option<u32>],
    name2: &[Option<u32>],
) -> Option<Vec<usize>> {
    let mut forward = vec![NONE; adj1.len()];
    let mut backward = vec![NONE; adj2.len()];
    let mut queue = VecDeque::from([(root1, root2)]);
    while let Some((u, v)) = queue.pop_front() {
        forward[u] = v;
        backward[v] = u;
        let mut label_list: HashMap<Option<u32>, Vec<usize>> = HashMap::new();
        for &next in &adj1[u] {
            if forward[next] == NONE {
                label_list.entry(name1[next]).or_default().push(next);
            }
        }
        for &next in &adj2[v] {
            if backward[next] == NONE {
                let paired = label_list.get_mut(&name2[next]).and_then(Vec::pop)?;
                queue.push_back((paired, next));
            }
        }
    }
    Some(forward)
}

/// The AHU algorithm on index-based trees: a vertex-to-vertex mapping covering every vertex of
/// the first tree, if the rooted trees are isomorphic.
fn ahu(adj1: &[Vec<usize>], root1: usize, adj2: &[Vec<usize>], root2: usize) -> Option<Vec<usize>> {
    let lv1 = levels(adj1, root1);
    let lv2 = levels(adj2, root2);
    if lv1.len() != lv2.len() {
        return None;
    }
    let mut name1: Vec<Option<u32>> = vec![None; adj1.len()];
    let mut name2: Vec<Option<u32>> = vec![None; adj2.len()];
    let mut canonical: HashMap<Vec<u32>, u32> = HashMap::new();
    for lvl in (0..lv1.len()).rev() {
        if lv1[lvl].len() != lv2[lvl].len() {
            return None;
        }
        for (levels, adj, name) in [(&lv1, adj1, &mut name1), (&lv2, adj2, &mut name2)] {
            for &u in &levels[lvl] {
                let mut list: Vec<u32> = adj[u].iter().filter_map(|&v| name[v]).collect();
                radix_sort(&mut list);
                let fresh = canonical.len() as u32;
                name[u] = Some(*canonical.entry(list).or_insert(fresh));
            }
        }
    }
    let forward = match_vertices_with_same_label(adj1, adj2, root1, root2, &name1, &name2)?;
    (forward.iter().filter(|&&f| f != NONE).count() == adj1.len()).then_some(forward)
}

fn mapping_from_indices<'g, G: Graph>(
    vs1: &[&G::Vertex],
    vs2: &[&G::Vertex],
    forward: &[usize],
    g1: &'g G,
    g2: &'g G,
) -> IsomorphicGraphMapping<'g, G> {
    let mut f = IndexMap::new();
    let mut b = IndexMap::new();
    for (i, &j) in forward.iter().enumerate().take(vs1.len()) {
        f.insert(vs1[i].clone(), vs2[j].clone());
        b.insert(vs2[j].clone(), vs1[i].clone());
    }
    IsomorphicGraphMapping::from_maps(f, b, g1, g2)
}

/// `AHURootedTreeIsomorphismInspector`.
pub struct AHURootedTreeIsomorphismInspector<'g, G: Graph> {
    tree1: &'g G,
    tree2: &'g G,
    root1: usize,
    root2: usize,
    vs1: Vec<&'g G::Vertex>,
    vs2: Vec<&'g G::Vertex>,
    adj1: Vec<Vec<usize>>,
    adj2: Vec<Vec<usize>>,
    result: OnceCell<Option<Vec<usize>>>,
}

fn validate_tree<G: Graph>(tree: &G, root: &G::Vertex) -> Result<()> {
    debug_assert!(is_simple(tree));
    if tree.vertex_count() == 0 {
        return Err(GraphError::InvalidArgument("tree cannot be empty".into()));
    }
    if !tree.contains_vertex(root) {
        return Err(GraphError::InvalidArgument(
            "root not contained in forest".into(),
        ));
    }
    Ok(())
}

impl<'g, G: Graph> AHURootedTreeIsomorphismInspector<'g, G> {
    pub fn new(tree1: &'g G, root1: &G::Vertex, tree2: &'g G, root2: &G::Vertex) -> Result<Self> {
        validate_tree(tree1, root1)?;
        validate_tree(tree2, root2)?;
        let (vs1, adj1) = adjacency(tree1);
        let (vs2, adj2) = adjacency(tree2);
        let index_of = |vs: &[&G::Vertex], r: &G::Vertex| vs.iter().position(|v| *v == r).unwrap();
        Ok(AHURootedTreeIsomorphismInspector {
            tree1,
            tree2,
            root1: index_of(&vs1, root1),
            root2: index_of(&vs2, root2),
            vs1,
            vs2,
            adj1,
            adj2,
            result: OnceCell::new(),
        })
    }

    fn forward(&self) -> &Option<Vec<usize>> {
        self.result
            .get_or_init(|| ahu(&self.adj1, self.root1, &self.adj2, self.root2))
    }

    /// The isomorphism, if the rooted trees are isomorphic.
    pub fn mapping(&self) -> Option<IsomorphicGraphMapping<'g, G>> {
        let f = self.forward().as_ref()?;
        Some(mapping_from_indices::<G>(
            &self.vs1, &self.vs2, f, self.tree1, self.tree2,
        ))
    }
}

impl<'g, G: Graph> IsomorphismInspector<'g, G> for AHURootedTreeIsomorphismInspector<'g, G> {
    fn mappings(&self) -> Box<dyn Iterator<Item = IsomorphicGraphMapping<'g, G>> + '_> {
        Box::new(self.mapping().into_iter())
    }
    fn isomorphism_exists(&self) -> bool {
        self.forward().is_some()
    }
}

/// `AHUUnrootedTreeIsomorphismInspector`: roots both trees at their centers.
pub struct AHUUnrootedTreeIsomorphismInspector<'g, G: Graph> {
    tree1: &'g G,
    tree2: &'g G,
    result: OnceCell<Option<AHURootedTreeIsomorphismInspector<'g, G>>>,
}

fn validate_unrooted<G: Graph>(tree: &G) -> Result<()> {
    require_undirected(tree)?;
    debug_assert!(is_simple(tree));
    if tree.vertex_count() == 0 {
        return Err(GraphError::InvalidArgument("tree cannot be empty".into()));
    }
    Ok(())
}

/// `TreeMeasurer.getGraphCenter`: the middle of a longest path found by a double BFS.
fn tree_center(adj: &[Vec<usize>]) -> Vec<usize> {
    let bfs = |root: usize| {
        let mut depth = vec![NONE; adj.len()];
        let mut parent = vec![NONE; adj.len()];
        let (mut farthest, mut dist) = (NONE, 0);
        let mut queue = VecDeque::from([root]);
        depth[root] = 0;
        while let Some(u) = queue.pop_front() {
            if farthest == NONE || dist < depth[u] {
                farthest = u;
                dist = depth[u];
            }
            for &v in &adj[u] {
                if depth[v] == NONE {
                    depth[v] = depth[u] + 1;
                    parent[v] = u;
                    queue.push_back(v);
                }
            }
        }
        (farthest, parent)
    };
    let (v1, _) = bfs(0);
    let (mut v2, parent) = bfs(v1);
    let mut path = Vec::new();
    while v2 != NONE {
        path.push(v2);
        v2 = parent[v2];
    }
    if path.len() % 2 == 1 {
        vec![path[path.len() / 2]]
    } else {
        vec![path[path.len() / 2], path[path.len() / 2 - 1]]
    }
}

impl<'g, G: Graph> AHUUnrootedTreeIsomorphismInspector<'g, G> {
    pub fn new(tree1: &'g G, tree2: &'g G) -> Result<Self> {
        validate_unrooted(tree1)?;
        validate_unrooted(tree2)?;
        Ok(AHUUnrootedTreeIsomorphismInspector {
            tree1,
            tree2,
            result: OnceCell::new(),
        })
    }

    fn rooted(&self) -> &Option<AHURootedTreeIsomorphismInspector<'g, G>> {
        self.result.get_or_init(|| {
            let (vs1, adj1) = adjacency(self.tree1);
            let (vs2, adj2) = adjacency(self.tree2);
            let (c1, c2) = (tree_center(&adj1), tree_center(&adj2));
            let make = |a: usize, b: usize| {
                AHURootedTreeIsomorphismInspector::new(self.tree1, vs1[a], self.tree2, vs2[b])
                    .expect("trees were validated")
            };
            let insp = match (c1.len(), c2.len()) {
                (1, 1) => make(c1[0], c2[0]),
                (2, 2) => {
                    let first = make(c1[0], c2[0]);
                    if first.isomorphism_exists() {
                        first
                    } else {
                        make(c1[1], c2[0])
                    }
                }
                _ => return None,
            };
            insp.isomorphism_exists().then_some(insp)
        })
    }

    /// The isomorphism, if the trees are isomorphic.
    pub fn mapping(&self) -> Option<IsomorphicGraphMapping<'g, G>> {
        self.rooted().as_ref()?.mapping()
    }
}

impl<'g, G: Graph> IsomorphismInspector<'g, G> for AHUUnrootedTreeIsomorphismInspector<'g, G> {
    fn mappings(&self) -> Box<dyn Iterator<Item = IsomorphicGraphMapping<'g, G>> + '_> {
        Box::new(self.mapping().into_iter())
    }
    fn isomorphism_exists(&self) -> bool {
        self.rooted().is_some()
    }
}

/// `AHUForestIsomorphismInspector`: joins each forest's roots under a fresh root vertex.
pub struct AHUForestIsomorphismInspector<'g, G: Graph> {
    forest1: &'g G,
    forest2: &'g G,
    roots1: Vec<G::Vertex>,
    roots2: Vec<G::Vertex>,
    result: OnceCell<Option<IsomorphicGraphMapping<'g, G>>>,
}

fn validate_forest<G: Graph>(forest: &G, roots: &[G::Vertex]) -> Result<()> {
    debug_assert!(is_simple(forest));
    if forest.vertex_count() == 0 {
        return Err(GraphError::InvalidArgument(
            "input forest cannot be empty".into(),
        ));
    }
    if roots.is_empty() {
        return Err(GraphError::InvalidArgument(
            "set of roots cannot be empty".into(),
        ));
    }
    if !roots.iter().all(|r| forest.contains_vertex(r)) {
        return Err(GraphError::InvalidArgument(
            "root not contained in forest".into(),
        ));
    }
    Ok(())
}

impl<'g, G: Graph> AHUForestIsomorphismInspector<'g, G> {
    pub fn new(
        forest1: &'g G,
        roots1: impl IntoIterator<Item = G::Vertex>,
        forest2: &'g G,
        roots2: impl IntoIterator<Item = G::Vertex>,
    ) -> Result<Self> {
        let dedup = |roots: Vec<G::Vertex>| {
            let mut seen = HashSet::new();
            roots
                .into_iter()
                .filter(|r| seen.insert(r.clone()))
                .collect::<Vec<_>>()
        };
        let roots1 = dedup(roots1.into_iter().collect());
        let roots2 = dedup(roots2.into_iter().collect());
        validate_forest(forest1, &roots1)?;
        validate_forest(forest2, &roots2)?;
        Ok(AHUForestIsomorphismInspector {
            forest1,
            forest2,
            roots1,
            roots2,
            result: OnceCell::new(),
        })
    }

    /// Adds a vertex (index `n`) adjacent to every root, and an edge back from each root.
    fn with_fresh_root<'a>(
        forest: &'a G,
        roots: &[G::Vertex],
    ) -> (Vec<&'a G::Vertex>, Vec<Vec<usize>>, usize) {
        let (vs, mut adj) = adjacency(forest);
        let fresh = vs.len();
        let root_ix: Vec<usize> = roots
            .iter()
            .map(|r| vs.iter().position(|v| *v == r).unwrap())
            .collect();
        for &r in &root_ix {
            adj[r].push(fresh);
        }
        adj.push(root_ix);
        (vs, adj, fresh)
    }

    /// The isomorphism, if the forests are isomorphic (as rooted forests).
    pub fn mapping(&self) -> Option<IsomorphicGraphMapping<'g, G>> {
        self.result
            .get_or_init(|| {
                if self.roots1.len() == 1 && self.roots2.len() == 1 {
                    return AHURootedTreeIsomorphismInspector::new(
                        self.forest1,
                        &self.roots1[0],
                        self.forest2,
                        &self.roots2[0],
                    )
                    .ok()?
                    .mapping();
                }
                let (vs1, adj1, fresh1) = Self::with_fresh_root(self.forest1, &self.roots1);
                let (vs2, adj2, fresh2) = Self::with_fresh_root(self.forest2, &self.roots2);
                let forward = ahu(&adj1, fresh1, &adj2, fresh2)?;
                Some(mapping_from_indices::<G>(
                    &vs1,
                    &vs2,
                    &forward,
                    self.forest1,
                    self.forest2,
                ))
            })
            .clone()
    }
}

impl<'g, G: Graph> IsomorphismInspector<'g, G> for AHUForestIsomorphismInspector<'g, G> {
    fn mappings(&self) -> Box<dyn Iterator<Item = IsomorphicGraphMapping<'g, G>> + '_> {
        Box::new(self.mapping().into_iter())
    }
    fn isomorphism_exists(&self) -> bool {
        self.mapping().is_some()
    }
}
