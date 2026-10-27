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
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        FixedSizeIntegerQueue {
            vs: vec![0; capacity],
            i: 0,
            n: 0,
        }
    }
    pub fn enqueue(&mut self, e: usize) {
        assert!(self.n < self.vs.len(), "queue capacity exceeded");
        self.vs[self.n] = e;
        self.n += 1;
    }
    pub fn poll(&mut self) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        self.i += 1;
        Some(self.vs[self.i - 1])
    }
    pub fn is_empty(&self) -> bool {
        self.i == self.n
    }
    pub fn len(&self) -> usize {
        self.n - self.i
    }
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
