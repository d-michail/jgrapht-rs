//! Dynamic connectivity in a forest (`TreeDynamicConnectivity`).
//!
//! Like the Java class this keeps an Euler tour of every tree, supporting `link`, `cut` and
//! `connected` in logarithmic time. The tours live in one arena of treap nodes (the Java version
//! uses its own `AVLTree`, which is not part of this port); only the observable behaviour is the
//! same.

use crate::{GraphError, Result};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

const NIL: usize = usize::MAX;

#[derive(Debug, Clone)]
struct TNode {
    left: usize,
    right: usize,
    parent: usize,
    size: usize,
    prio: u64,
}

/// A forest whose trees can be linked and cut.
///
/// Every element has one occurrence node in its tree's tour, and every edge adds two arc nodes.
#[derive(Debug, Clone)]
pub struct TreeDynamicConnectivity<T: Eq + Hash + Clone> {
    nodes: Vec<TNode>,
    free: Vec<usize>,
    rng: u64,
    /// Occurrence node of each element.
    occurrence: HashMap<T, usize>,
    /// Arc nodes `(from, to)` of every edge, keyed by element.
    arcs: HashMap<(T, T), usize>,
    neighbors: HashMap<T, HashSet<T>>,
}

impl<T: Eq + Hash + Clone> Default for TreeDynamicConnectivity<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Eq + Hash + Clone> TreeDynamicConnectivity<T> {
    pub fn new() -> Self {
        TreeDynamicConnectivity {
            nodes: Vec::new(),
            free: Vec::new(),
            rng: 0x9E37_79B9_7F4A_7C15,
            occurrence: HashMap::new(),
            arcs: HashMap::new(),
            neighbors: HashMap::new(),
        }
    }

    pub fn contains(&self, element: &T) -> bool {
        self.occurrence.contains_key(element)
    }

    /// Adds an isolated element; `false` if it was already present.
    pub fn add(&mut self, element: T) -> bool {
        if self.contains(&element) {
            return false;
        }
        let n = self.new_node();
        self.occurrence.insert(element.clone(), n);
        self.neighbors.insert(element, HashSet::new());
        true
    }

    /// Removes an element, cutting all its edges; `false` if it was absent.
    pub fn remove(&mut self, element: &T) -> bool {
        if !self.contains(element) {
            return false;
        }
        let nbrs: Vec<T> = self.neighbors[element].iter().cloned().collect();
        for t in nbrs {
            self.cut_edge(element, &t);
        }
        let n = self.occurrence.remove(element).unwrap();
        self.neighbors.remove(element);
        self.free.push(n);
        true
    }

    /// Whether the two elements are joined by a path. Following the Java class, an element with
    /// no edges is connected to nothing, not even itself.
    pub fn connected(&self, first: &T, second: &T) -> bool {
        let (Some(&a), Some(&b)) = (self.occurrence.get(first), self.occurrence.get(second)) else {
            return false;
        };
        if self.neighbors[first].is_empty() || self.neighbors[second].is_empty() {
            return false;
        }
        self.root(a) == self.root(b)
    }

    /// Adds the edge `first -- second`, adding missing elements; `false` if they are already
    /// connected (the edge would close a cycle) or equal.
    pub fn link(&mut self, first: T, second: T) -> bool {
        if first == second {
            return false;
        }
        self.add(first.clone());
        self.add(second.clone());
        if self.connected(&first, &second) {
            return false;
        }
        let (a, b) = (self.occurrence[&first], self.occurrence[&second]);
        let ta = self.reroot(a);
        let tb = self.reroot(b);
        // tour(first) = [first | rest]; the result is [first, arc, tour(second), arc', rest]
        let (head, rest) = self.split(ta, 1);
        let arc_ab = self.new_node();
        let arc_ba = self.new_node();
        let mut t = self.merge(head, arc_ab);
        t = self.merge(t, tb);
        t = self.merge(t, arc_ba);
        self.merge(t, rest);
        self.arcs.insert((first.clone(), second.clone()), arc_ab);
        self.arcs.insert((second.clone(), first.clone()), arc_ba);
        self.neighbors
            .get_mut(&first)
            .unwrap()
            .insert(second.clone());
        self.neighbors.get_mut(&second).unwrap().insert(first);
        true
    }

    /// Removes the edge `first -- second`. Returns `false` if the elements are not connected and
    /// an error if they are connected but not adjacent.
    pub fn cut(&mut self, first: &T, second: &T) -> Result<bool> {
        if !self.connected(first, second) {
            return Ok(false);
        }
        if !self.arcs.contains_key(&(first.clone(), second.clone())) {
            return Err(GraphError::InvalidArgument(
                "elements are not adjacent in the forest".into(),
            ));
        }
        self.cut_edge(first, second);
        Ok(true)
    }

    fn cut_edge(&mut self, first: &T, second: &T) {
        let a = self.arcs.remove(&(first.clone(), second.clone())).unwrap();
        let b = self.arcs.remove(&(second.clone(), first.clone())).unwrap();
        self.neighbors.get_mut(first).unwrap().remove(second);
        self.neighbors.get_mut(second).unwrap().remove(first);
        let (mut i, mut j) = (self.rank(a), self.rank(b));
        let (mut x, mut y) = (a, b);
        if i > j {
            std::mem::swap(&mut i, &mut j);
            std::mem::swap(&mut x, &mut y);
        }
        // P x M y S  ->  P S and M
        let root = self.root(x);
        let (p, rest) = self.split(root, i);
        let (_x, rest) = self.split(rest, 1);
        let (m, rest) = self.split(rest, j - i - 1);
        let (_y, s) = self.split(rest, 1);
        self.merge(p, s);
        let _ = m;
        self.free.push(x);
        self.free.push(y);
    }

    // ----- treap -------------------------------------------------------------------------------

    fn new_node(&mut self) -> usize {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let n = TNode {
            left: NIL,
            right: NIL,
            parent: NIL,
            size: 1,
            prio: self.rng,
        };
        if let Some(i) = self.free.pop() {
            self.nodes[i] = n;
            i
        } else {
            self.nodes.push(n);
            self.nodes.len() - 1
        }
    }

    fn size(&self, t: usize) -> usize {
        if t == NIL { 0 } else { self.nodes[t].size }
    }

    fn update(&mut self, t: usize) {
        let (l, r) = (self.nodes[t].left, self.nodes[t].right);
        self.nodes[t].size = 1 + self.size(l) + self.size(r);
        if l != NIL {
            self.nodes[l].parent = t;
        }
        if r != NIL {
            self.nodes[r].parent = t;
        }
    }

    fn root(&self, mut t: usize) -> usize {
        while self.nodes[t].parent != NIL {
            t = self.nodes[t].parent;
        }
        t
    }

    /// Zero-based position of `t` in its sequence.
    fn rank(&self, mut t: usize) -> usize {
        let mut r = self.size(self.nodes[t].left);
        while self.nodes[t].parent != NIL {
            let p = self.nodes[t].parent;
            if self.nodes[p].right == t {
                r += self.size(self.nodes[p].left) + 1;
            }
            t = p;
        }
        r
    }

    fn merge(&mut self, a: usize, b: usize) -> usize {
        if a == NIL || b == NIL {
            let t = if a == NIL { b } else { a };
            if t != NIL {
                self.nodes[t].parent = NIL;
            }
            return t;
        }
        if self.nodes[a].prio > self.nodes[b].prio {
            let r = self.nodes[a].right;
            let m = self.merge(r, b);
            self.nodes[a].right = m;
            self.update(a);
            self.nodes[a].parent = NIL;
            a
        } else {
            let l = self.nodes[b].left;
            let m = self.merge(a, l);
            self.nodes[b].left = m;
            self.update(b);
            self.nodes[b].parent = NIL;
            b
        }
    }

    /// Splits off the first `k` elements.
    fn split(&mut self, t: usize, k: usize) -> (usize, usize) {
        if t == NIL {
            return (NIL, NIL);
        }
        let ls = self.size(self.nodes[t].left);
        if k <= ls {
            let l = self.nodes[t].left;
            let (a, b) = self.split(l, k);
            self.nodes[t].left = b;
            self.update(t);
            self.nodes[t].parent = NIL;
            if a != NIL {
                self.nodes[a].parent = NIL;
            }
            (a, t)
        } else {
            let r = self.nodes[t].right;
            let (a, b) = self.split(r, k - ls - 1);
            self.nodes[t].right = a;
            self.update(t);
            self.nodes[t].parent = NIL;
            if b != NIL {
                self.nodes[b].parent = NIL;
            }
            (t, b)
        }
    }

    /// Rotates the tour of `n` so that `n` comes first; returns the new root.
    fn reroot(&mut self, n: usize) -> usize {
        let root = self.root(n);
        let k = self.rank(n);
        if k == 0 {
            return root;
        }
        let (a, b) = self.split(root, k);
        self.merge(b, a)
    }
}
