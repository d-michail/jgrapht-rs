/// Capabilities of a graph, mirroring JGraphT's `GraphType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphType {
    pub directed: bool,
    pub self_loops: bool,
    pub multiple_edges: bool,
    pub weighted: bool,
    pub allows_cycles: bool,
    pub modifiable: bool,
}

impl GraphType {
    /// Simple graph: no loops, no parallel edges, unweighted. Direction is set by the graph.
    pub const fn simple() -> Self {
        Self {
            directed: false,
            self_loops: false,
            multiple_edges: false,
            weighted: false,
            allows_cycles: true,
            modifiable: true,
        }
    }
    /// Multigraph: parallel edges, no loops.
    pub const fn multigraph() -> Self {
        Self {
            multiple_edges: true,
            ..Self::simple()
        }
    }
    /// Pseudograph: parallel edges and loops.
    pub const fn pseudograph() -> Self {
        Self {
            multiple_edges: true,
            self_loops: true,
            ..Self::simple()
        }
    }
    pub const fn directed(mut self, d: bool) -> Self {
        self.directed = d;
        self
    }
    pub const fn weighted(mut self, w: bool) -> Self {
        self.weighted = w;
        self
    }
    pub const fn is_undirected(&self) -> bool {
        !self.directed
    }
    pub const fn is_simple(&self) -> bool {
        !self.multiple_edges && !self.self_loops
    }
    pub const fn is_pseudograph(&self) -> bool {
        self.multiple_edges && self.self_loops
    }
    pub const fn is_multigraph(&self) -> bool {
        self.multiple_edges && !self.self_loops
    }
    pub const fn is_acyclic(&self) -> bool {
        !self.allows_cycles
    }
    pub const fn as_unmodifiable(mut self) -> Self {
        self.modifiable = false;
        self
    }
    pub const fn as_unweighted(mut self) -> Self {
        self.weighted = false;
        self
    }
    pub const fn as_weighted(mut self) -> Self {
        self.weighted = true;
        self
    }
}
