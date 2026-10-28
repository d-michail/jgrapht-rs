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

//! A pairing heap that reproduces JHeaps' `PairingHeap` exactly, including which of several
//! equal-key entries comes out first. JGraphT's tests (traversal order, shortest paths with ties)
//! depend on that order, which differs from other pairing-heap layouts.

use rheaps::error::{DecreaseKeyError, InvalidHandle};
use rheaps::{AddressableHeap, DecreaseKeyHeap};

const NIL: usize = usize::MAX;

struct Node<K, V> {
    entry: Option<(K, V)>,
    /// "older child": head of the child list (the most recently linked child)
    o_c: usize,
    /// "younger sibling": next in the child list
    y_s: usize,
    /// "older sibling or parent"
    o_s: usize,
    generation: u32,
}

/// Opaque entry handle of a [`JavaPairingHeap`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JavaPairingHandle {
    index: usize,
    generation: u32,
}

/// A pairing heap that breaks ties like the one in JGraphT, so that algorithms visit equal keys in
/// the same order.
pub struct JavaPairingHeap<K, V> {
    nodes: Vec<Node<K, V>>,
    free: Vec<usize>,
    root: usize,
    len: usize,
}

impl<K, V> Default for JavaPairingHeap<K, V> {
    fn default() -> Self {
        JavaPairingHeap {
            nodes: Vec::new(),
            free: Vec::new(),
            root: NIL,
            len: 0,
        }
    }
}

impl<K: Ord, V> JavaPairingHeap<K, V> {
    /// An empty heap.
    pub fn new() -> Self {
        Self::default()
    }

    fn key_of(&self, n: usize) -> &K {
        &self.nodes[n].entry.as_ref().expect("live node").0
    }

    fn check(&self, h: JavaPairingHandle) -> Result<usize, InvalidHandle> {
        match self.nodes.get(h.index) {
            Some(n) if n.generation == h.generation && n.entry.is_some() => Ok(h.index),
            _ => Err(InvalidHandle::Stale),
        }
    }

    fn link(&mut self, f: usize, s: usize) -> usize {
        if s == NIL {
            return f;
        }
        if f == NIL {
            return s;
        }
        if self.key_of(f) <= self.key_of(s) {
            let head = self.nodes[f].o_c;
            self.nodes[s].y_s = head;
            self.nodes[s].o_s = f;
            if head != NIL {
                self.nodes[head].o_s = s;
            }
            self.nodes[f].o_c = s;
            f
        } else {
            self.link(s, f)
        }
    }

    fn cut_children(&mut self, n: usize) -> usize {
        let child = self.nodes[n].o_c;
        self.nodes[n].o_c = NIL;
        if child != NIL {
            self.nodes[child].o_s = NIL;
        }
        child
    }

    /// Two-pass pairing of a child list.
    fn combine(&mut self, l: usize) -> usize {
        if l == NIL {
            return NIL;
        }
        let (mut pairs, mut it) = (NIL, l);
        while it != NIL {
            let mut p_it = it;
            it = self.nodes[it].y_s;
            if it == NIL {
                self.nodes[p_it].y_s = pairs;
                self.nodes[p_it].o_s = NIL;
                pairs = p_it;
            } else {
                let n_it = self.nodes[it].y_s;
                self.nodes[p_it].y_s = NIL;
                self.nodes[p_it].o_s = NIL;
                self.nodes[it].y_s = NIL;
                self.nodes[it].o_s = NIL;
                p_it = self.link(p_it, it);
                self.nodes[p_it].y_s = pairs;
                pairs = p_it;
                it = n_it;
            }
        }
        let (mut it, mut f) = (pairs, NIL);
        while it != NIL {
            let next = self.nodes[it].y_s;
            self.nodes[it].y_s = NIL;
            f = self.link(f, it);
            it = next;
        }
        f
    }

    fn detach(&mut self, n: usize) {
        let (y, o) = (self.nodes[n].y_s, self.nodes[n].o_s);
        if y != NIL {
            self.nodes[y].o_s = o;
        }
        if self.nodes[o].o_c == n {
            self.nodes[o].o_c = y;
        } else {
            self.nodes[o].y_s = y;
        }
        self.nodes[n].y_s = NIL;
        self.nodes[n].o_s = NIL;
    }

    fn release(&mut self, n: usize) -> (K, V) {
        let node = &mut self.nodes[n];
        node.generation = node.generation.wrapping_add(1);
        node.o_c = NIL;
        node.y_s = NIL;
        node.o_s = NIL;
        let e = node.entry.take().expect("live node");
        self.free.push(n);
        e
    }
}

impl<K: Ord, V> AddressableHeap<K, V> for JavaPairingHeap<K, V> {
    type Handle = JavaPairingHandle;

    fn insert(&mut self, key: K, value: V) -> JavaPairingHandle {
        let node = Node {
            entry: Some((key, value)),
            o_c: NIL,
            y_s: NIL,
            o_s: NIL,
            generation: 0,
        };
        let index = match self.free.pop() {
            Some(i) => {
                let generation = self.nodes[i].generation;
                self.nodes[i] = Node { generation, ..node };
                i
            }
            None => {
                self.nodes.push(node);
                self.nodes.len() - 1
            }
        };
        self.root = self.link(self.root, index);
        self.len += 1;
        JavaPairingHandle {
            index,
            generation: self.nodes[index].generation,
        }
    }

    fn peek(&self) -> Option<(JavaPairingHandle, &K, &V)> {
        if self.root == NIL {
            return None;
        }
        let n = &self.nodes[self.root];
        let (k, v) = n.entry.as_ref()?;
        Some((
            JavaPairingHandle {
                index: self.root,
                generation: n.generation,
            },
            k,
            v,
        ))
    }

    fn pop(&mut self) -> Option<(K, V)> {
        if self.root == NIL {
            return None;
        }
        let old = self.root;
        let children = self.cut_children(old);
        self.root = self.combine(children);
        self.len -= 1;
        Some(self.release(old))
    }

    fn key(&self, handle: JavaPairingHandle) -> Result<&K, InvalidHandle> {
        self.check(handle).map(|i| self.key_of(i))
    }

    fn value(&self, handle: JavaPairingHandle) -> Result<&V, InvalidHandle> {
        let i = self.check(handle)?;
        Ok(&self.nodes[i].entry.as_ref().expect("live").1)
    }

    fn value_mut(&mut self, handle: JavaPairingHandle) -> Result<&mut V, InvalidHandle> {
        let i = self.check(handle)?;
        Ok(&mut self.nodes[i].entry.as_mut().expect("live").1)
    }

    fn delete(&mut self, handle: JavaPairingHandle) -> Result<(K, V), InvalidHandle> {
        let n = self.check(handle)?;
        if self.root == n {
            return Ok(self.pop().expect("root exists"));
        }
        self.detach(n);
        let children = self.cut_children(n);
        let t = self.combine(children);
        self.root = self.link(self.root, t);
        self.len -= 1;
        Ok(self.release(n))
    }

    fn len(&self) -> usize {
        self.len
    }

    fn clear(&mut self) {
        for n in &mut self.nodes {
            if n.entry.take().is_some() {
                n.generation = n.generation.wrapping_add(1);
            }
        }
        self.free = (0..self.nodes.len()).collect();
        self.root = NIL;
        self.len = 0;
    }
}

impl<K: Ord, V> DecreaseKeyHeap<K, V> for JavaPairingHeap<K, V> {
    fn decrease_key(&mut self, handle: JavaPairingHandle, key: K) -> Result<(), DecreaseKeyError> {
        let n = self
            .check(handle)
            .map_err(DecreaseKeyError::InvalidHandle)?;
        let ord = key.cmp(self.key_of(n));
        if ord == std::cmp::Ordering::Greater {
            return Err(DecreaseKeyError::NotDecreased);
        }
        self.nodes[n].entry.as_mut().expect("live").0 = key;
        if ord == std::cmp::Ordering::Equal || self.root == n {
            return Ok(());
        }
        self.detach(n);
        self.root = self.link(self.root, n);
        Ok(())
    }
}
