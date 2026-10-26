//! Walker's alias method for sampling from a discrete distribution in `O(1)`.

use super::java::RandomSource;
use super::ToleranceDoubleComparator;
use crate::{GraphError, Result};

pub struct AliasMethodSampler<R> {
    rng: R,
    comparator: ToleranceDoubleComparator,
    prob: Vec<f64>,
    alias: Vec<usize>,
}

impl<R: RandomSource> AliasMethodSampler<R> {
    pub fn new(p: &[f64], rng: R) -> Result<Self> {
        Self::with_epsilon(p, rng, ToleranceDoubleComparator::DEFAULT_EPSILON)
    }

    pub fn with_epsilon(p: &[f64], rng: R, epsilon: f64) -> Result<Self> {
        let comparator = ToleranceDoubleComparator::new(epsilon)?;
        let bad = || GraphError::InvalidArgument("Non valid probability distribution".into());
        if p.is_empty() {
            return Err(GraphError::InvalidArgument("Probabilities cannot be empty".into()));
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
        let (mut large, mut small): (Vec<usize>, Vec<usize>) = (Vec::with_capacity(n), Vec::with_capacity(n));
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
        Ok(AliasMethodSampler { rng, comparator, prob, alias })
    }

    /// Draws an index according to the distribution.
    pub fn next(&mut self) -> usize {
        let u = self.rng.next_double() * self.prob.len() as f64;
        let j = u.floor() as usize;
        if self.comparator.compare(u - j as f64, self.prob[j]) <= 0 { j } else { self.alias[j] }
    }
}
