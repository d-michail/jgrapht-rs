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

use crate::util::java::RandomSource;
use crate::{GraphError, Result};

/// Randomly distributes a number of values among keys subject to per-key lower and upper bounds.
///
/// Unlike JGraphT's `Distributor` it does not own its random source; pass it to
/// [`distribution`](Self::distribution).
pub struct Distributor<'a, K> {
    lower_bounds: Vec<Box<dyn Fn(&K) -> i32 + 'a>>,
    upper_bounds: Vec<Box<dyn Fn(&K) -> i32 + 'a>>,
}

impl<K> Default for Distributor<'_, K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, K> Distributor<'a, K> {
    /// A distributor with no keys.
    pub fn new() -> Self {
        Distributor {
            lower_bounds: Vec::new(),
            upper_bounds: Vec::new(),
        }
    }

    /// Adds upper bound.
    pub fn add_upper_bound(&mut self, f: impl Fn(&K) -> i32 + 'a) {
        self.upper_bounds.push(Box::new(f));
    }

    /// Adds lower bound.
    pub fn add_lower_bound(&mut self, f: impl Fn(&K) -> i32 + 'a) {
        self.lower_bounds.push(Box::new(f));
    }

    fn compute_lower_bounds(&self, keys: &[K]) -> Vec<i32> {
        keys.iter()
            .map(|k| self.lower_bounds.iter().fold(0, |lb, f| lb.max(f(k))))
            .collect()
    }

    fn compute_upper_bounds(&self, keys: &[K]) -> Vec<i32> {
        keys.iter()
            .map(|k| {
                self.upper_bounds
                    .iter()
                    .fold(i32::MAX, |ub, f| ub.min(f(k)))
            })
            .collect()
    }

    /// `(suffix sums clamped to int, total)`.
    fn suffix_sum(bounds: &[i32]) -> (Vec<i32>, i64) {
        let mut suffix = vec![0; bounds.len()];
        let mut sum = 0i64;
        for i in (0..bounds.len()).rev() {
            suffix[i] = sum.min(i32::MAX as i64) as i32;
            sum += bounds[i] as i64;
        }
        (suffix, sum)
    }
}

impl<K: std::fmt::Debug> Distributor<'_, K> {
    /// One value per key, summing to `value_count`, each within its bounds.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::Infeasible`] if the problem has no feasible solution;
    /// [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn distribution(
        &self,
        rng: &mut impl RandomSource,
        keys: &[K],
        value_count: i32,
    ) -> Result<Vec<i32>> {
        let key_lb = self.compute_lower_bounds(keys);
        let key_ub = self.compute_upper_bounds(keys);
        let (lb_suffix, lb_sum) = Self::suffix_sum(&key_lb);
        let (ub_suffix, ub_sum) = Self::suffix_sum(&key_ub);
        if lb_sum > value_count as i64 || ub_sum < value_count as i64 {
            return Err(GraphError::Infeasible);
        }
        let mut remaining = value_count;
        let mut result = Vec::with_capacity(keys.len());
        for i in 0..key_lb.len() {
            // `int` arithmetic as in Java
            let value_ub = remaining.wrapping_sub(lb_suffix[i]);
            let value_lb = remaining.wrapping_sub(ub_suffix[i]);
            let lower = key_lb[i].max(value_lb);
            let upper = key_ub[i].min(value_ub);
            if lower > upper {
                return Err(GraphError::Infeasible);
            }
            let bound = upper.wrapping_sub(lower).wrapping_add(1);
            if bound <= 0 {
                return Err(GraphError::must_be_positive("bound"));
            }
            let allocated = rng.next_int_bound(bound as usize) as i32 + lower;
            result.push(allocated);
            remaining -= allocated;
        }
        Ok(result)
    }
}
