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

use std::hash::{Hash, Hasher};

/// Generic pair.
///
/// # Rust notes
///
/// Ordered pair (`Pair`). Absent elements are `Option`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Pair<A, B> {
    /// The first.
    pub first: A,
    /// The second.
    pub second: B,
}

impl<A, B> Pair<A, B> {
    /// Create a new pair.
    pub fn of(first: A, second: B) -> Self {
        Pair { first, second }
    }
    /// Get the first element of the pair
    pub fn first(&self) -> &A {
        &self.first
    }
    /// Get the second element of the pair
    pub fn second(&self) -> &B {
        &self.second
    }
}

impl<A: PartialEq, B: PartialEq> Pair<A, B> {
    /// Returns whether there is first.
    pub fn has_first(&self, a: &A) -> bool {
        self.first == *a
    }
    /// Returns whether there is second.
    pub fn has_second(&self, b: &B) -> bool {
        self.second == *b
    }
}

impl<T: PartialEq> Pair<T, T> {
    /// Whether `t` is one of the two elements (Java's `hasElement`).
    pub fn has_element(&self, t: &T) -> bool {
        self.first == *t || self.second == *t
    }
}

/// Generic triple (3-tuple).
///
/// # Rust notes
///
/// Ordered triple (`Triple`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Triple<A, B, C> {
    /// The first.
    pub first: A,
    /// The second.
    pub second: B,
    /// The third.
    pub third: C,
}

impl<A, B, C> Triple<A, B, C> {
    /// Create a new triple.
    pub fn of(first: A, second: B, third: C) -> Self {
        Triple {
            first,
            second,
            third,
        }
    }
}

/// Generic unordered pair.
///
/// Although the instances of this class are immutable, it is impossible to ensure that the
/// references passed to the constructor will not be modified by the caller.
///
/// # Rust notes
///
/// Pair that is equal to its swap and hashes independently of the order (`UnorderedPair`).
///
/// Java lets the two elements have different static types; here they share one type (use an enum
/// for heterogeneous elements).
#[derive(Debug, Clone, Copy)]
pub struct UnorderedPair<T> {
    /// The first.
    pub first: T,
    /// The second.
    pub second: T,
}

impl<T> UnorderedPair<T> {
    /// Creates new unordered pair of elements pulling of the necessity to provide corresponding
    /// types of the elements supplied.
    pub fn of(first: T, second: T) -> Self {
        UnorderedPair { first, second }
    }
}

impl<T: PartialEq> UnorderedPair<T> {
    /// Assess if this pair contains an element.
    pub fn has_element(&self, t: &T) -> bool {
        self.first == *t || self.second == *t
    }
}

impl<T: PartialEq> PartialEq for UnorderedPair<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.first == other.first && self.second == other.second)
            || (self.first == other.second && self.second == other.first)
    }
}
impl<T: Eq> Eq for UnorderedPair<T> {}

impl<T: Hash> Hash for UnorderedPair<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let h = |t: &T| {
            let mut s = std::collections::hash_map::DefaultHasher::new();
            t.hash(&mut s);
            s.finish()
        };
        let (h1, h2) = (h(&self.first), h(&self.second));
        // same shape as JGraphT's `hash1 > hash2 ? hash1 * 31 + hash2 : hash2 * 31 + hash1`
        state.write_u64(if h1 > h2 {
            h1.wrapping_mul(31).wrapping_add(h2)
        } else {
            h2.wrapping_mul(31).wrapping_add(h1)
        });
    }
}
