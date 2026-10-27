use super::{ShortestPathAlgorithm, SingleSourcePaths};
use crate::alg::util::Indexed;
use crate::graph::Graph;
use crate::{GraphError, Result};
use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Parallel Δ-stepping (Java `DeltaSteppingShortestPath`): the same bucket structure as the Java class
/// (`ceil(maxWeight / delta) + 1` circular buckets, bucket of `d` is `round(d / delta)`), with the
/// light and heavy relaxation phases of a bucket split across `parallelism` worker threads.
/// Distances and predecessors are updated under a per-vertex lock, buckets are ordered sets
/// behind their own locks (lock order: vertex, then bucket). Requires non-negative weights.
pub struct ParallelDeltaSteppingShortestPath<'g, G> {
    g: &'g G,
    /// `0.0` means "compute it": the largest edge weight divided by the largest out-degree.
    delta: f64,
    parallelism: usize,
}

type State = (f64, Option<(usize, usize)>); // (distance, (edge, tail))

impl<'g, G: Graph> ParallelDeltaSteppingShortestPath<'g, G> {
    /// Automatic delta, one worker per available CPU.
    pub fn new(g: &'g G) -> Self {
        Self::with_parallelism(g, default_parallelism())
    }

    /// Automatic delta with an explicit number of worker threads (at least one).
    pub fn with_parallelism(g: &'g G, parallelism: usize) -> Self {
        Self {
            g,
            delta: 0.0,
            parallelism: parallelism.max(1),
        }
    }

    /// Explicit delta (`0.0` selects the automatic one); fails if `delta` is negative or NaN.
    pub fn with_delta(g: &'g G, delta: f64, parallelism: usize) -> Result<Self> {
        if delta.is_nan() || delta < 0.0 {
            return Err(GraphError::InvalidArgument(
                "Delta must be non-negative".into(),
            ));
        }
        Ok(Self {
            g,
            delta,
            parallelism: parallelism.max(1),
        })
    }
}

fn default_parallelism() -> usize {
    std::thread::available_parallelism().map_or(1, usize::from)
}

struct Run<'a> {
    out: &'a [Vec<(usize, usize)>],
    w: &'a [f64],
    delta: f64,
    parallelism: usize,
    state: Vec<Mutex<State>>,
    buckets: Vec<Mutex<BTreeSet<usize>>>,
}

impl Run<'_> {
    fn bucket_index(&self, distance: f64) -> usize {
        (distance / self.delta).round() as usize % self.buckets.len()
    }

    fn relax(&self, v: usize, edge: usize, from: Option<usize>, distance: f64) {
        let target = self.bucket_index(distance);
        let mut s = self.state[v].lock().unwrap();
        if distance < s.0 {
            if s.0.is_finite() {
                self.buckets[self.bucket_index(s.0)]
                    .lock()
                    .unwrap()
                    .remove(&v);
            }
            self.buckets[target].lock().unwrap().insert(v);
            *s = (distance, from.map(|f| (edge, f)));
        }
    }

    /// Relaxes the light (`w <= delta`) or heavy (`w > delta`) edges of `vertices` on up to
    /// `parallelism` threads and waits for all of them.
    fn relax_requests(&self, vertices: &[usize], light: bool) {
        if vertices.is_empty() {
            return;
        }
        let next = AtomicUsize::new(0);
        let work = || {
            loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(&v) = vertices.get(i) else { break };
                let d = self.state[v].lock().unwrap().0;
                for &(u, k) in &self.out[v] {
                    if (self.w[k] <= self.delta) == light {
                        self.relax(u, k, Some(v), d + self.w[k]);
                    }
                }
            }
        };
        let threads = self.parallelism.min(vertices.len());
        std::thread::scope(|scope| {
            for _ in 1..threads {
                scope.spawn(work);
            }
            work();
        });
    }

    /// `getContentAndReplace`.
    fn take_bucket(&self, i: usize) -> BTreeSet<usize> {
        std::mem::take(&mut *self.buckets[i].lock().unwrap())
    }

    fn compute(&self, source: usize) {
        self.relax(source, usize::MAX, None, 0.0);
        let mut removed: Vec<usize> = Vec::new();
        loop {
            let Some(first) = self
                .buckets
                .iter()
                .position(|b| !b.lock().unwrap().is_empty())
            else {
                break;
            };
            let mut elements = self.take_bucket(first);
            while !elements.is_empty() {
                let batch: Vec<usize> = elements.iter().copied().collect();
                removed.extend_from_slice(&batch);
                self.relax_requests(&batch, true);
                elements = self.take_bucket(first);
            }
            self.relax_requests(&removed, false);
            removed.clear();
        }
    }
}

impl<G: Graph> ShortestPathAlgorithm<G> for ParallelDeltaSteppingShortestPath<'_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        let ix = Indexed::new(self.g);
        let s = *ix.idx.get(source).ok_or(GraphError::VertexNotFound)?;
        if ix.w.iter().any(|w| w.is_nan() || *w < 0.0) {
            return Err(GraphError::InvalidArgument(
                "Negative edge weight not allowed".into(),
            ));
        }
        let max_w = ix.w.iter().copied().fold(0.0, f64::max);
        let delta = if self.delta == 0.0 {
            if max_w == 0.0 {
                1.0
            } else {
                let max_out_degree = ix.out.iter().map(Vec::len).max().unwrap_or(1).max(1);
                max_w / max_out_degree as f64
            }
        } else {
            self.delta
        };
        let n = ix.n();
        let num_buckets = (max_w / delta).ceil() as usize + 1;
        let run = Run {
            out: &ix.out,
            w: &ix.w,
            delta,
            parallelism: self.parallelism,
            state: (0..n).map(|_| Mutex::new((f64::INFINITY, None))).collect(),
            buckets: (0..num_buckets)
                .map(|_| Mutex::new(BTreeSet::new()))
                .collect(),
        };
        run.compute(s);

        let mut result = SingleSourcePaths {
            source: source.clone(),
            dist: HashMap::new(),
            pred: HashMap::new(),
        };
        for (v, state) in run.state.into_iter().enumerate() {
            let (dist, pred) = state.into_inner().unwrap();
            if dist.is_finite() {
                result.dist.insert(ix.vertex(v), dist);
                if let Some((k, from)) = pred {
                    result
                        .pred
                        .insert(ix.vertex(v), (ix.es[k].clone(), ix.vertex(from)));
                }
            }
        }
        Ok(result)
    }
}
