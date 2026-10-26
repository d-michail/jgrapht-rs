//! Lowest common ancestors in rooted trees/forests (binary lifting).

use crate::graph::{Graph, GraphExt};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// LCA structure over the tree reachable from `root` (edges followed in their direction;
/// undirected graphs are treated as trees rooted at `root`).
pub struct Lca<V> {
    index: HashMap<V, usize>,
    vertices: Vec<V>,
    depth: Vec<usize>,
    up: Vec<Vec<usize>>,
}

impl<V: Clone + Eq + Hash> Lca<V> {
    pub fn new<G: Graph<Vertex = V>>(g: &G, root: &V) -> Option<Self> {
        let root = g.vertices().find(|v| *v == root)?;
        let mut index = HashMap::from([(root.clone(), 0)]);
        let mut vertices = vec![root.clone()];
        let (mut depth, mut parent) = (vec![0], vec![0]);
        let mut q = VecDeque::from([root]);
        while let Some(v) = q.pop_front() {
            let i = index[v];
            for n in g.out_neighbors_of(v) {
                if !index.contains_key(n) {
                    index.insert(n.clone(), vertices.len());
                    vertices.push(n.clone());
                    depth.push(depth[i] + 1);
                    parent.push(i);
                    q.push_back(n);
                }
            }
        }
        let levels = (usize::BITS - vertices.len().leading_zeros()) as usize;
        let mut up = vec![parent];
        for k in 1..levels.max(1) {
            let prev = &up[k - 1];
            up.push((0..vertices.len()).map(|v| prev[prev[v]]).collect());
        }
        Some(Lca {
            index,
            vertices,
            depth,
            up,
        })
    }

    /// Lowest common ancestor; `None` if either vertex is not in the tree.
    pub fn lca(&self, a: &V, b: &V) -> Option<&V> {
        let (mut a, mut b) = (*self.index.get(a)?, *self.index.get(b)?);
        if self.depth[a] < self.depth[b] {
            std::mem::swap(&mut a, &mut b);
        }
        let diff = self.depth[a] - self.depth[b];
        for (k, level) in self.up.iter().enumerate() {
            if diff >> k & 1 == 1 {
                a = level[a];
            }
        }
        if a != b {
            for level in self.up.iter().rev() {
                if level[a] != level[b] {
                    a = level[a];
                    b = level[b];
                }
            }
            a = self.up[0][a];
        }
        Some(&self.vertices[a])
    }

    pub fn depth(&self, v: &V) -> Option<usize> {
        self.index.get(v).map(|&i| self.depth[i])
    }
}
