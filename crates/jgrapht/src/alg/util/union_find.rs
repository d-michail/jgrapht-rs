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

use crate::{GraphError, Result};
use indexmap::IndexMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;

/// An implementation of [Union Find](http://en.wikipedia.org/wiki/Disjoint-set_data_structure) data structure. Union Find is a disjoint-set data structure. It supports two operations:
/// finding the set a specific element is in, and merging two sets. The implementation uses union by
/// rank and path compression to achieve an amortized cost of $O(\alpha(n))$ per operation where
/// $\alpha$ is the inverse Ackermann function. UnionFind uses the hashCode and equals method of the
/// elements it operates on.
///
/// # Rust notes
///
/// Union–find over arbitrary elements with union by rank and path compression (`UnionFind<T>`).
/// Elements keep insertion order.
#[derive(Debug, Clone)]
pub struct UnionFind<T: Eq + Hash + Clone> {
    parent: IndexMap<T, T>,
    rank: IndexMap<T, usize>,
    count: usize,
}

impl<T: Eq + Hash + Clone> UnionFind<T> {
    /// Creates a UnionFind instance with all the elements in separate sets.
    pub fn new(elements: impl IntoIterator<Item = T>) -> Self {
        let mut uf = UnionFind {
            parent: IndexMap::new(),
            rank: IndexMap::new(),
            count: 0,
        };
        for e in elements {
            if !uf.parent.contains_key(&e) {
                uf.parent.insert(e.clone(), e.clone());
                uf.rank.insert(e, 0);
                uf.count += 1;
            }
        }
        uf
    }

    /// Adds a new element to the data structure in its own set.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if an argument violates a precondition; errors of the underlying graph operations are propagated.
    pub fn add_element(&mut self, element: T) -> Result<()> {
        if self.contains(&element) {
            return Err(GraphError::DuplicateElement);
        }
        self.parent.insert(element.clone(), element.clone());
        self.rank.insert(element, 0);
        self.count += 1;
        Ok(())
    }

    /// Returns true if the given value is an element in this UnionFind data structure.
    pub fn contains(&self, element: &T) -> bool {
        self.parent.contains_key(element)
    }

    /// Representative of the set containing `element`.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if an argument violates a precondition; errors of the underlying graph operations are propagated.
    pub fn find(&mut self, element: &T) -> Result<T> {
        if !self.contains(element) {
            return Err(GraphError::ElementNotFound);
        }
        let mut current = element.clone();
        loop {
            let parent = &self.parent[&current];
            if *parent == current {
                break;
            }
            current = parent.clone();
        }
        let root = current;
        let mut current = element.clone();
        while current != root {
            let parent = self.parent[&current].clone();
            self.parent.insert(current, root.clone());
            current = parent;
        }
        Ok(root)
    }

    /// Merges the sets which contain element1 and element2. No guarantees are given as to which
    /// element becomes the representative of the resulting (merged) set: this can be either
    /// find(element1) or find(element2).
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if an argument violates a precondition; errors of the underlying graph operations are propagated.
    pub fn union(&mut self, element1: &T, element2: &T) -> Result<()> {
        if !self.contains(element1) || !self.contains(element2) {
            return Err(GraphError::ElementNotFound);
        }
        let parent1 = self.find(element1)?;
        let parent2 = self.find(element2)?;
        if parent1 == parent2 {
            return Ok(());
        }
        let (rank1, rank2) = (self.rank[&parent1], self.rank[&parent2]);
        if rank1 > rank2 {
            self.parent.insert(parent2, parent1);
        } else if rank1 < rank2 {
            self.parent.insert(parent1, parent2);
        } else {
            self.parent.insert(parent2, parent1.clone());
            self.rank.insert(parent1, rank1 + 1);
        }
        self.count -= 1;
        Ok(())
    }

    /// Tests whether two elements are contained in the same set.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn in_same_set(&mut self, element1: &T, element2: &T) -> Result<bool> {
        Ok(self.find(element1)? == self.find(element2)?)
    }

    /// Returns the number of sets. Initially, all items are in their own set. The smallest number of
    /// sets equals one.
    pub fn number_of_sets(&self) -> usize {
        self.count
    }

    /// Returns the len.
    pub fn len(&self) -> usize {
        self.parent.len()
    }

    /// Returns whether empty.
    pub fn is_empty(&self) -> bool {
        self.parent.is_empty()
    }

    /// Puts every element back into its own set.
    pub fn reset(&mut self) {
        for (k, v) in self.parent.iter_mut() {
            *v = k.clone();
        }
        for r in self.rank.values_mut() {
            *r = 0;
        }
        self.count = self.parent.len();
    }
}

impl<T: Eq + Hash + Clone + Display> Display for UnionFind<T> {
    /// `{{rep:a,b}, {rep2:c}}` like JGraphT's `toString`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut uf = self.clone();
        let mut sets: IndexMap<T, Vec<T>> = IndexMap::new();
        let keys: Vec<T> = uf.parent.keys().cloned().collect();
        for t in keys {
            let rep = uf.find(&t).expect("element present");
            sets.entry(rep).or_default().push(t);
        }
        let parts: Vec<String> = sets
            .iter()
            .map(|(rep, members)| {
                let m: Vec<String> = members.iter().map(|x| x.to_string()).collect();
                format!("{{{}:{}}}", rep, m.join(","))
            })
            .collect();
        write!(f, "{{{}}}", parts.join(", "))
    }
}

impl<T: Eq + Hash + Clone + Debug> UnionFind<T> {
    /// The current partition, sets in order of first appearance.
    ///
    /// # Panics
    ///
    /// Panics only if an internal invariant is violated, which indicates a bug in this library.
    pub fn sets(&mut self) -> Vec<Vec<T>> {
        let keys: Vec<T> = self.parent.keys().cloned().collect();
        let mut sets: IndexMap<T, Vec<T>> = IndexMap::new();
        for t in keys {
            let rep = self.find(&t).expect("element present");
            sets.entry(rep).or_default().push(t);
        }
        sets.into_values().collect()
    }
}
