// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

//! Java-style building blocks (shuffle, priority queue, hash order) and the [`RandomSource`]
//! abstraction. Randomised algorithms accept any `rand::RngCore`; seeded results are valid but
//! are not expected to match JGraphT's for the same seed.

/// Source of randomness used by the randomised algorithms; implemented for every `rand::RngCore`.
pub trait RandomSource {
    /// Uniform over all `i32` values (`Random.nextInt()`).
    fn next_int(&mut self) -> i32;
    /// Uniform in `[0, bound)`.
    fn next_int_bound(&mut self, bound: usize) -> usize;
    /// Uniform in `[0, 1)`.
    fn next_double(&mut self) -> f64;
    /// A random boolean.
    fn next_boolean(&mut self) -> bool;
    /// Uniform in `[0, 1)` with float precision.
    fn next_float(&mut self) -> f32;
}

impl<R: rand::RngCore> RandomSource for R {
    fn next_int(&mut self) -> i32 {
        rand::RngCore::next_u32(self) as i32
    }
    fn next_int_bound(&mut self, bound: usize) -> usize {
        rand::Rng::random_range(self, 0..bound)
    }
    fn next_double(&mut self) -> f64 {
        rand::Rng::random::<f64>(self)
    }
    fn next_boolean(&mut self) -> bool {
        rand::Rng::random_bool(self, 0.5)
    }
    fn next_float(&mut self) -> f32 {
        rand::Rng::random::<f32>(self)
    }
}

/// `Collections.shuffle(list, rng)`: Fisher–Yates from the back, `swap(i - 1, nextInt(i))`.
pub fn java_shuffle<T>(list: &mut [T], rng: &mut impl RandomSource) {
    for i in (2..=list.len()).rev() {
        let j = rng.next_int_bound(i);
        list.swap(i - 1, j);
    }
}

/// `java.util.PriorityQueue` with a comparator: same sift-up/sift-down, so ties and even
/// inconsistent comparators behave exactly as in Java.
pub struct JavaPriorityQueue<T, C: Fn(&T, &T) -> i32> {
    queue: Vec<T>,
    cmp: C,
}

impl<T, C: Fn(&T, &T) -> i32> JavaPriorityQueue<T, C> {
    /// An empty queue ordered by `cmp`, which returns a negative, zero or positive number like Java's
    /// `Comparator`.
    pub fn new(cmp: C) -> Self {
        JavaPriorityQueue {
            queue: Vec::new(),
            cmp,
        }
    }
    /// Number of elements.
    pub fn len(&self) -> usize {
        self.queue.len()
    }
    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
    /// The smallest element without removing it.
    pub fn peek(&self) -> Option<&T> {
        self.queue.first()
    }
    /// Inserts `x`.
    pub fn offer(&mut self, x: T) {
        let mut k = self.queue.len();
        self.queue.push(x);
        // sift up from position k without moving the element until its final slot is known
        while k > 0 {
            let parent = (k - 1) >> 1;
            if (self.cmp)(&self.queue[k], &self.queue[parent]) >= 0 {
                break;
            }
            self.queue.swap(k, parent);
            k = parent;
        }
    }
    /// Removes and returns the smallest element.
    pub fn poll(&mut self) -> Option<T> {
        let n = self.queue.len();
        if n == 0 {
            return None;
        }
        self.queue.swap(0, n - 1);
        let result = self.queue.pop();
        let n = n - 1;
        if n > 1 {
            // sift the element now at index 0 down, as Java's siftDown(0, x)
            let mut k = 0;
            let half = n >> 1;
            while k < half {
                let mut child = 2 * k + 1;
                let right = child + 1;
                if right < n && (self.cmp)(&self.queue[child], &self.queue[right]) > 0 {
                    child = right;
                }
                if (self.cmp)(&self.queue[k], &self.queue[child]) <= 0 {
                    break;
                }
                self.queue.swap(k, child);
                k = child;
            }
        }
        result
    }
}

/// `String.hashCode()` of a Java string, for UTF-16 code units.
pub fn java_string_hash(s: &str) -> i32 {
    s.encode_utf16()
        .fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(c as i32))
}

/// The iteration order of a `java.util.HashMap` / `HashSet` that received `items` (distinct, in
/// this insertion order) with default capacity: buckets by `(h ^ h >>> 16) & (capacity - 1)`,
/// insertion order within a bucket. Tree bins (very long collision chains) are not modelled.
pub fn java_hash_order<T>(items: Vec<T>, hash: impl Fn(&T) -> i32) -> Vec<T> {
    let mut capacity = 16usize;
    while items.len() * 4 > capacity * 3 {
        capacity *= 2;
    }
    let mut keyed: Vec<(usize, T)> = items
        .into_iter()
        .map(|t| {
            let h = hash(&t) as u32;
            (((h ^ (h >> 16)) as usize) & (capacity - 1), t)
        })
        .collect();
    keyed.sort_by_key(|(bucket, _)| *bucket);
    keyed.into_iter().map(|(_, t)| t).collect()
}

/// Iteration order of a `java.util.HashSet` built as `new HashSet<>(first)` followed by adding
/// `rest` (the pattern `new HashSet<>(a); addAll(b)`), for elements with the given `hashCode`.
pub fn java_hash_set_order<T>(first: Vec<T>, rest: Vec<T>, hash: impl Fn(&T) -> i32) -> Vec<T> {
    // new HashSet<>(Collection) sizes its table for max(n / 0.75 + 1, 16) elements
    let wanted = ((first.len() as f32 / 0.75) as usize + 1).max(16);
    let mut capacity = wanted.next_power_of_two();
    let mut size = first.len();
    for _ in 0..rest.len() {
        size += 1;
        if size > capacity * 3 / 4 {
            capacity *= 2;
        }
    }
    let mut keyed: Vec<(usize, T)> = first
        .into_iter()
        .chain(rest)
        .map(|t| {
            let h = hash(&t) as u32;
            (((h ^ (h >> 16)) as usize) & (capacity - 1), t)
        })
        .collect();
    keyed.sort_by_key(|(bucket, _)| *bucket);
    keyed.into_iter().map(|(_, t)| t).collect()
}
