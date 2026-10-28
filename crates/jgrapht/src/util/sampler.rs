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

//! Walker's alias method for sampling from a discrete distribution in `O(1)`.

use super::ToleranceDoubleComparator;
use super::java::RandomSource;
use crate::{GraphError, Result};

/// Walker's alias method: draws from a discrete distribution in constant time after a linear setup.
pub struct AliasMethodSampler {
    comparator: ToleranceDoubleComparator,
    prob: Vec<f64>,
    alias: Vec<usize>,
}

impl AliasMethodSampler {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn new(p: &[f64]) -> Result<Self> {
        Self::with_epsilon(p, ToleranceDoubleComparator::DEFAULT_EPSILON)
    }

    /// Creates an instance with the given epsilon.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn with_epsilon(p: &[f64], epsilon: f64) -> Result<Self> {
        let comparator = ToleranceDoubleComparator::new(epsilon)?;
        let bad = || GraphError::InvalidParameter {
            name: "probabilities",
            requirement: crate::ParameterRequirement::ProbabilityDistribution,
        };
        if p.is_empty() {
            return Err(GraphError::InvalidParameter {
                name: "probabilities",
                requirement: crate::ParameterRequirement::NonEmpty,
            });
        }
        let mut sum = 0.0;
        for &x in p {
            if comparator.compare(x, 0.0) < 0 {
                return Err(bad());
            }
            sum += x;
        }
        if comparator.compare(sum, 1.0) != 0 {
            return Err(bad());
        }
        let mut p = p.to_vec();
        let n = p.len();
        let threshold = 1.0 / n as f64;
        let (mut large, mut small): (Vec<usize>, Vec<usize>) =
            (Vec::with_capacity(n), Vec::with_capacity(n));
        for (j, &x) in p.iter().enumerate() {
            if comparator.compare(x, threshold) > 0 {
                large.push(j);
            } else {
                small.push(j);
            }
        }
        let mut prob = vec![0.0; n];
        let mut alias = vec![0usize; n];
        while !small.is_empty() && !large.is_empty() {
            let j = small.pop().unwrap();
            let k = large.pop().unwrap();
            prob[j] = n as f64 * p[j];
            alias[j] = k;
            p[k] += p[j] - threshold;
            if comparator.compare(p[k], threshold) > 0 {
                large.push(k);
            } else {
                small.push(k);
            }
        }
        while let Some(j) = small.pop() {
            prob[j] = 1.0;
        }
        while let Some(j) = large.pop() {
            prob[j] = 1.0;
        }
        Ok(AliasMethodSampler {
            comparator,
            prob,
            alias,
        })
    }

    /// Draws an index according to the distribution using `rng`.
    pub fn next(&self, rng: &mut impl RandomSource) -> usize {
        let u = rng.next_double() * self.prob.len() as f64;
        let j = u.floor() as usize;
        if self.comparator.compare(u - j as f64, self.prob[j]) <= 0 {
            j
        } else {
            self.alias[j]
        }
    }
}
