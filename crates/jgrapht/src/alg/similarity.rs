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

//! Tree similarity: Zhang–Shasha tree edit distance.

use crate::graph::IncidenceGraph;
use crate::graph::OutgoingGraph;
use crate::graph::{Graph, GraphExt, props};
use crate::{GraphError, Result};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationType {
    Insert,
    Remove,
    Change,
}

/// One step of an edit script. `Insert` and `Remove` have no second operand.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditOperation<V> {
    pub ty: OperationType,
    pub first: V,
    pub second: Option<V>,
}

impl<V> EditOperation<V> {
    pub fn new(ty: OperationType, first: V, second: Option<V>) -> Self {
        EditOperation { ty, first, second }
    }
}

struct CacheEntry<V> {
    prev_i: usize,
    prev_j: usize,
    edit_operation: Option<EditOperation<V>>,
    tree_distance_i: usize,
    tree_distance_j: usize,
}

/// Postorder numbering (1-based) with leftmost-leaf values and keyroots.
struct TreeOrdering<V> {
    keyroots: Vec<usize>,
    index_to_vertex: Vec<Option<V>>,
    index_to_l: Vec<usize>,
}

impl<V: Clone + PartialEq> TreeOrdering<V> {
    fn new<G: Graph<Vertex = V> + OutgoingGraph>(tree: &G, root: &V) -> Self {
        let n = tree.vertex_count();
        let mut ordering = TreeOrdering {
            keyroots: Vec::new(),
            index_to_vertex: vec![None; n + 1],
            index_to_l: vec![0; n + 1],
        };
        ordering.compute(tree, root);
        ordering
    }

    /// The iterative post-order traversal of JGraphT, state machine included.
    fn compute<G: Graph<Vertex = V> + OutgoingGraph>(&mut self, tree: &G, root: &V) {
        struct Entry<V> {
            v: V,
            is_keyroot: bool,
            v_parent: Option<V>,
            is_keyroot_arg: bool,
            l_value: Option<usize>,
            children: std::vec::IntoIter<V>,
            l_v_child: usize,
            state: u8,
        }
        let successors = |v: &V| -> Vec<V> {
            tree.out_edges_of(v)
                .filter_map(|e| tree.opposite_vertex(e, v).cloned())
                .collect()
        };
        let mut current_index = 1;
        let mut stack = vec![Entry {
            v: root.clone(),
            is_keyroot: true,
            v_parent: None,
            is_keyroot_arg: false,
            l_value: None,
            children: Vec::new().into_iter(),
            l_v_child: 0,
            state: 0,
        }];
        while !stack.is_empty() {
            let top = stack.len() - 1;
            match stack[top].state {
                0 => {
                    if stack.len() > 1 {
                        stack[top].v_parent = Some(stack[top - 1].v.clone());
                    }
                    stack[top].children = successors(&stack[top].v).into_iter();
                    stack[top].state = 1;
                }
                1 => match stack[top].children.next() {
                    Some(child) => {
                        if stack[top].v_parent.as_ref().is_none_or(|p| *p != child) {
                            let is_keyroot = stack[top].is_keyroot_arg;
                            stack.push(Entry {
                                v: child,
                                is_keyroot,
                                v_parent: None,
                                is_keyroot_arg: false,
                                l_value: None,
                                children: Vec::new().into_iter(),
                                l_v_child: 0,
                                state: 0,
                            });
                            stack[top].state = 2;
                        }
                    }
                    None => stack[top].state = 3,
                },
                2 => {
                    let e = &mut stack[top];
                    e.is_keyroot_arg = true;
                    if e.l_value.is_none() {
                        e.l_value = Some(e.l_v_child);
                    }
                    e.state = 1;
                }
                _ => {
                    let e = stack.pop().unwrap();
                    let l_value = e.l_value.unwrap_or(current_index);
                    if e.is_keyroot {
                        self.keyroots.push(current_index);
                    }
                    self.index_to_vertex[current_index] = Some(e.v);
                    self.index_to_l[current_index] = l_value;
                    current_index += 1;
                    if let Some(parent) = stack.last_mut() {
                        parent.l_v_child = l_value;
                    }
                }
            }
        }
    }
}

type CostFn<'a, V> = Box<dyn Fn(&V) -> f64 + 'a>;
type ChangeFn<'a, V> = Box<dyn Fn(&V, &V) -> f64 + 'a>;

/// Dynamic programming algorithm for computing edit distance between trees.
///
/// The algorithm is originally described in Zhang, Kaizhong & Shasha, Dennis. (1989). Simple
/// Fast Algorithms for the Editing Distance Between Trees and Related Problems. SIAM J. Comput.. 18.
/// 1245-1262. 10.1137/0218082.
///
/// The time complexity of the algorithm is $O(|T_1|\cdot|T_2|\cdot min(depth(T_1),leaves(T_1)) \cdot
/// min(depth(T_2),leaves(T_2)))$. Space complexity is $O(|T_1|\cdot |T_2|)$, where $|T_1|$ and
/// $|T_2|$ denote number of vertices in trees $T_1$ and $T_2$ correspondingly, $leaves()$ function
/// returns number of leaf vertices in a tree.
///
/// The tree edit distance problem is defined in a following way. Consider $2$ trees $T_1$ and $T_2$
/// with root vertices $r_1$ and $r_2$ correspondingly. For those trees there are 3 elementary
/// modification actions:
///
/// - Remove a vertex $v$ from $T_1$.
/// - Insert a vertex $v$ into $T_2$.
/// - Change vertex $v_1$ in $T_1$ to vertex $v_2$ in $T_2$.
///
/// The algorithm assigns a cost to each of those operations which also depends on the vertices. The
/// problem is then to compute a sequence of edit operations which transforms $T_1$ into $T_2$ and
/// has a minimum cost over all such sequences. Here the cost of a sequence of edit operations is
/// defined as sum of costs of individual operations.
///
/// The algorithm is based on a dynamic programming principle and assigns a label to each vertex in
/// the trees which is equal to its index in post-oder traversal. It also uses a notion of a keyroot
/// which is defined as a vertex in a tree which has a left sibling. Additionally a special $l()$
/// function is introduced with returns for every vertex the index of its leftmost child wrt the
/// post-order traversal in the tree.
///
/// Solving the tree edit problem distance is divided into computing edit distance for every pair of
/// subtrees rooted at vertices $v_1$ and $v_2$ where $v_1$ is a keyroot in the first tree and $v_2$
/// is a keyroot in the second tree.
///
/// # Rust notes
///
/// Zhang–Shasha tree edit distance of two rooted trees, with the edit script
/// (`ZhangShashaTreeEditDistance`).
pub struct ZhangShashaTreeEditDistance<'a, G: Graph> {
    tree1: &'a G,
    root1: G::Vertex,
    tree2: &'a G,
    root2: G::Vertex,
    insert_cost: CostFn<'a, G::Vertex>,
    remove_cost: CostFn<'a, G::Vertex>,
    change_cost: ChangeFn<'a, G::Vertex>,
    tree_distances: Vec<Vec<f64>>,
    edit_operation_lists: Vec<Vec<Option<Vec<EditOperation<G::Vertex>>>>>,
    executed: bool,
}

impl<'a, G: OutgoingGraph + IncidenceGraph> ZhangShashaTreeEditDistance<'a, G>
where
    G::Vertex: Eq + Hash,
{
    /// Unit costs for inserting, removing and changing (0 for equal vertices).
    pub fn new(tree1: &'a G, root1: G::Vertex, tree2: &'a G, root2: G::Vertex) -> Result<Self> {
        Self::with_costs(
            tree1,
            root1,
            tree2,
            root2,
            |_| 1.0,
            |_| 1.0,
            |a, b| if a == b { 0.0 } else { 1.0 },
        )
    }

    pub fn with_costs(
        tree1: &'a G,
        root1: G::Vertex,
        tree2: &'a G,
        root2: G::Vertex,
        insert_cost: impl Fn(&G::Vertex) -> f64 + 'a,
        remove_cost: impl Fn(&G::Vertex) -> f64 + 'a,
        change_cost: impl Fn(&G::Vertex, &G::Vertex) -> f64 + 'a,
    ) -> Result<Self> {
        if !props::is_tree(tree1) {
            return Err(GraphError::InvalidArgument("graph1 must be a tree!".into()));
        }
        if !props::is_tree(tree2) {
            return Err(GraphError::InvalidArgument("graph2 must be a tree!".into()));
        }
        let (m, n) = (tree1.vertex_count(), tree2.vertex_count());
        Ok(ZhangShashaTreeEditDistance {
            tree1,
            root1,
            tree2,
            root2,
            insert_cost: Box::new(insert_cost),
            remove_cost: Box::new(remove_cost),
            change_cost: Box::new(change_cost),
            tree_distances: vec![vec![0.0; n]; m],
            edit_operation_lists: (0..m).map(|_| vec![None; n]).collect(),
            executed: false,
        })
    }

    /// Computes edit distance for `tree1` and `tree2`.
    pub fn distance(&mut self) -> f64 {
        self.run();
        self.tree_distances[self.tree1.vertex_count() - 1][self.tree2.vertex_count() - 1]
    }

    pub fn edit_operations(&mut self) -> &[EditOperation<G::Vertex>] {
        self.run();
        self.edit_operation_lists[self.tree1.vertex_count() - 1][self.tree2.vertex_count() - 1]
            .as_deref()
            .unwrap_or(&[])
    }

    fn run(&mut self) {
        if self.executed {
            return;
        }
        let o1 = TreeOrdering::new(self.tree1, &self.root1);
        let o2 = TreeOrdering::new(self.tree2, &self.root2);
        for &k1 in &o1.keyroots {
            for &k2 in &o2.keyroots {
                self.tree_distance(k1, k2, &o1, &o2);
            }
        }
        self.executed = true;
    }

    /// Computes edit distance and list of edit operations for vertex $v1$ from `tree1` which
    /// has tree ordering index equal to $i$ and vertex $v2$ from `tree2` which has tree
    /// ordering index equal to $j$. Both $v1$ and $v2$ must be keyroots in the corresponding trees.
    fn tree_distance(
        &mut self,
        i: usize,
        j: usize,
        o1: &TreeOrdering<G::Vertex>,
        o2: &TreeOrdering<G::Vertex>,
    ) {
        let li = o1.index_to_l[i];
        let lj = o2.index_to_l[j];
        let (m, n) = (i - li + 2, j - lj + 2);
        let mut forestdist = vec![vec![0.0f64; n]; m];
        let mut cached: Vec<Vec<Option<CacheEntry<G::Vertex>>>> =
            (0..m).map(|_| (0..n).map(|_| None).collect()).collect();
        let (i_offset, j_offset) = (li - 1, lj - 1);
        let v1 = |k: usize| o1.index_to_vertex[k].clone().unwrap();
        let v2 = |k: usize| o2.index_to_vertex[k].clone().unwrap();

        for i1 in li..=i {
            let vertex = v1(i1);
            let idx = i1 - i_offset;
            forestdist[idx][0] = forestdist[idx - 1][0] + (self.remove_cost)(&vertex);
            cached[idx][0] = Some(CacheEntry {
                prev_i: idx - 1,
                prev_j: 0,
                edit_operation: Some(EditOperation::new(OperationType::Remove, vertex, None)),
                tree_distance_i: 0,
                tree_distance_j: 0,
            });
        }
        for j1 in lj..=j {
            let vertex = v2(j1);
            let idx = j1 - j_offset;
            // JGraphT charges the removal cost for the insertions along the border
            forestdist[0][idx] = forestdist[0][idx - 1] + (self.remove_cost)(&vertex);
            cached[0][idx] = Some(CacheEntry {
                prev_i: 0,
                prev_j: idx - 1,
                edit_operation: Some(EditOperation::new(OperationType::Insert, vertex, None)),
                tree_distance_i: 0,
                tree_distance_j: 0,
            });
        }

        for i1 in li..=i {
            let i1_vertex = v1(i1);
            let li1 = o1.index_to_l[i1];
            for j1 in lj..=j {
                let j1_vertex = v2(j1);
                let lj1 = o2.index_to_l[j1];
                let (ii, jj) = (i1 - i_offset, j1 - j_offset);
                if li1 == li && lj1 == lj {
                    let d1 = forestdist[ii - 1][jj] + (self.remove_cost)(&i1_vertex);
                    let d2 = forestdist[ii][jj - 1] + (self.insert_cost)(&j1_vertex);
                    let d3 =
                        forestdist[ii - 1][jj - 1] + (self.change_cost)(&i1_vertex, &j1_vertex);
                    let result = d1.min(d2.min(d3));
                    let entry = if result == d1 {
                        CacheEntry {
                            prev_i: ii - 1,
                            prev_j: jj,
                            edit_operation: Some(EditOperation::new(
                                OperationType::Remove,
                                i1_vertex.clone(),
                                None,
                            )),
                            tree_distance_i: 0,
                            tree_distance_j: 0,
                        }
                    } else if result == d2 {
                        CacheEntry {
                            prev_i: ii,
                            prev_j: jj - 1,
                            edit_operation: Some(EditOperation::new(
                                OperationType::Insert,
                                j1_vertex.clone(),
                                None,
                            )),
                            tree_distance_i: 0,
                            tree_distance_j: 0,
                        }
                    } else {
                        CacheEntry {
                            prev_i: ii - 1,
                            prev_j: jj - 1,
                            edit_operation: Some(EditOperation::new(
                                OperationType::Change,
                                i1_vertex.clone(),
                                Some(j1_vertex.clone()),
                            )),
                            tree_distance_i: 0,
                            tree_distance_j: 0,
                        }
                    };
                    cached[ii][jj] = Some(entry);
                    forestdist[ii][jj] = result;
                    self.tree_distances[i1 - 1][j1 - 1] = result;
                    self.edit_operation_lists[i1 - 1][j1 - 1] =
                        Some(self.restore_operations_list(&cached, ii, jj));
                } else {
                    let i2 = li1 - 1 - i_offset;
                    let j2 = lj1 - 1 - j_offset;
                    let d1 = forestdist[ii - 1][jj] + (self.remove_cost)(&i1_vertex);
                    let d2 = forestdist[ii][jj - 1] + (self.insert_cost)(&j1_vertex);
                    let d3 = forestdist[i2][j2] + self.tree_distances[i1 - 1][j1 - 1];
                    let result = d1.min(d2.min(d3));
                    forestdist[ii][jj] = result;
                    let entry = if result == d1 {
                        CacheEntry {
                            prev_i: ii - 1,
                            prev_j: jj,
                            edit_operation: Some(EditOperation::new(
                                OperationType::Remove,
                                i1_vertex.clone(),
                                None,
                            )),
                            tree_distance_i: 0,
                            tree_distance_j: 0,
                        }
                    } else if result == d2 {
                        CacheEntry {
                            prev_i: ii,
                            prev_j: jj - 1,
                            edit_operation: Some(EditOperation::new(
                                OperationType::Insert,
                                j1_vertex.clone(),
                                None,
                            )),
                            tree_distance_i: 0,
                            tree_distance_j: 0,
                        }
                    } else {
                        CacheEntry {
                            prev_i: i2,
                            prev_j: j2,
                            edit_operation: None,
                            tree_distance_i: i1 - 1,
                            tree_distance_j: j1 - 1,
                        }
                    };
                    cached[ii][jj] = Some(entry);
                }
            }
        }
    }

    /// Restores list of edit operations which have been cached in `cachedOperations` during
    /// the edit distance computation. Starting from a cache entry at index $(i,j)$.
    fn restore_operations_list(
        &self,
        cached: &[Vec<Option<CacheEntry<G::Vertex>>>],
        i: usize,
        j: usize,
    ) -> Vec<EditOperation<G::Vertex>> {
        let mut result = Vec::new();
        let mut it = cached[i][j].as_ref();
        while let Some(entry) = it {
            match &entry.edit_operation {
                None => {
                    if let Some(ops) =
                        &self.edit_operation_lists[entry.tree_distance_i][entry.tree_distance_j]
                    {
                        result.extend(ops.iter().cloned());
                    }
                }
                Some(op) => result.push(op.clone()),
            }
            it = cached[entry.prev_i][entry.prev_j].as_ref();
        }
        result
    }
}
