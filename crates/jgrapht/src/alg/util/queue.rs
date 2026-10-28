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

/// Primitive but efficient implementation of a fixed size queue for integers.
///
/// Note: this queue is not implemented as a ring, so at most $N$ enqueue operations are allowed,
/// where $N$ is the maximum capacity of the queue! After that, `clear()` must be invoked.
///
/// # Rust notes
///
/// Array-backed queue of `usize` with a fixed capacity (`FixedSizeIntegerQueue`): elements can be
/// enqueued only `capacity` times between calls to [`clear`](Self::clear).
#[derive(Debug, Clone)]
pub struct FixedSizeIntegerQueue {
    vs: Vec<usize>,
    /// index of the first element
    i: usize,
    /// one past the last element; the queue is empty when `i == n`
    n: usize,
}

impl FixedSizeIntegerQueue {
    /// Create a queue of fixed size.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        FixedSizeIntegerQueue {
            vs: vec![0; capacity],
            i: 0,
            n: 0,
        }
    }
    /// Add an element to the queue.
    ///
    /// # Panics
    ///
    /// Panics if the queue is already full.
    pub fn enqueue(&mut self, e: usize) {
        assert!(self.n < self.vs.len(), "queue capacity exceeded");
        self.vs[self.n] = e;
        self.n += 1;
    }
    /// Poll the first element from the queue.
    pub fn poll(&mut self) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        self.i += 1;
        Some(self.vs[self.i - 1])
    }
    /// Check if the queue has any items.
    pub fn is_empty(&self) -> bool {
        self.i == self.n
    }
    /// Returns the len.
    pub fn len(&self) -> usize {
        self.n - self.i
    }
    ///  Empty the queue.
    pub fn clear(&mut self) {
        self.i = 0;
        self.n = 0;
    }
}

impl std::fmt::Display for FixedSizeIntegerQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for j in self.i..self.n {
            write!(f, "{} ", self.vs[j])?;
        }
        Ok(())
    }
}
