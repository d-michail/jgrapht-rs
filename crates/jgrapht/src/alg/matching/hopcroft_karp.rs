use super::base::{Matching, MatchingAlgorithm};
use crate::alg::util::FixedSizeIntegerQueue;
use crate::error::Result;
use crate::graph::{Graph, GraphExt, props};
use indexmap::IndexSet;
use std::collections::HashMap;

/// Dummy vertex. All vertices are initially matched against this dummy vertex.
const DUMMY: usize = 0;
const INF: usize = usize::MAX;

/// `HopcroftKarpMaximumCardinalityBipartiteMatching`: a maximum cardinality matching of a
/// bipartite graph in `O(E sqrt(V))`.
pub struct HopcroftKarpMaximumCardinalityBipartiteMatching<'g, G: Graph> {
    graph: &'g G,
    partition1: Vec<G::Vertex>,
    partition2: Vec<G::Vertex>,
}

struct State<'a, G: Graph> {
    graph: &'a G,
    /// Ordered list of vertices; position 0 is the dummy vertex.
    vertices: Vec<Option<&'a G::Vertex>>,
    /// Neighbours (as positions) of every real vertex, in the order of the graph.
    neighbors: Vec<Vec<usize>>,
    partition1_size: usize,
    matched_vertices: usize,
    matching: Vec<usize>,
    dist: Vec<usize>,
    queue: FixedSizeIntegerQueue,
}

impl<'g, G: Graph> HopcroftKarpMaximumCardinalityBipartiteMatching<'g, G> {
    /// `partition1` and `partition2` are the two sides of the bipartition. Fails if the graph is
    /// not undirected.
    pub fn new(
        graph: &'g G,
        partition1: impl IntoIterator<Item = G::Vertex>,
        partition2: impl IntoIterator<Item = G::Vertex>,
    ) -> Result<Self> {
        props::require_undirected(graph)?;
        let p1: Vec<G::Vertex> = partition1.into_iter().collect();
        let p2: Vec<G::Vertex> = partition2.into_iter().collect();
        // Ensure that partition1 is smaller or equal in size compared to partition 2
        let (partition1, partition2) = if p1.len() <= p2.len() {
            (p1, p2)
        } else {
            (p2, p1)
        };
        Ok(Self {
            graph,
            partition1,
            partition2,
        })
    }
}

impl<'a, G: Graph> State<'a, G> {
    fn bfs(&mut self) -> bool {
        self.queue.clear();
        for u in 1..=self.partition1_size {
            if self.matching[u] == DUMMY {
                // add all unmatched vertices to the queue and set their distance to 0
                self.dist[u] = 0;
                self.queue.enqueue(u);
            } else {
                // set distance of all matched vertices to INF
                self.dist[u] = INF;
            }
        }
        self.dist[DUMMY] = INF;
        while let Some(u) = self.queue.poll() {
            if self.dist[u] < self.dist[DUMMY] {
                for i in 0..self.neighbors[u].len() {
                    let v = self.neighbors[u][i];
                    let m = self.matching[v];
                    if self.dist[m] == INF {
                        self.dist[m] = self.dist[u] + 1;
                        self.queue.enqueue(m);
                    }
                }
            }
        }
        self.dist[DUMMY] != INF // true if an augmenting path is found
    }

    fn dfs(&mut self, u: usize) -> bool {
        if u != DUMMY {
            for i in 0..self.neighbors[u].len() {
                let v = self.neighbors[u][i];
                let m = self.matching[v];
                if self.dist[m] == self.dist[u].wrapping_add(1) && self.dfs(m) {
                    self.matching[v] = u;
                    self.matching[u] = v;
                    return true;
                }
            }
            // No augmenting path has been found. Set distance of u to INF to ensure that u isn't
            // visited again.
            self.dist[u] = INF;
            return false;
        }
        true
    }
}

impl<G: Graph> MatchingAlgorithm<G::Vertex, G::Edge>
    for HopcroftKarpMaximumCardinalityBipartiteMatching<'_, G>
{
    fn get_matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        let graph = self.graph;
        // init
        let mut vertices: Vec<Option<&G::Vertex>> = vec![None];
        vertices.extend(self.partition1.iter().map(Some));
        vertices.extend(self.partition2.iter().map(Some));
        let index: HashMap<&G::Vertex, usize> = vertices
            .iter()
            .enumerate()
            .filter_map(|(i, v)| v.map(|v| (v, i)))
            .collect();
        let neighbors: Vec<Vec<usize>> = vertices
            .iter()
            .map(|v| match v {
                None => Vec::new(),
                Some(v) => graph.neighbor_list_of(v).iter().map(|w| index[w]).collect(),
            })
            .collect();
        let n = vertices.len();
        let mut state = State {
            graph,
            vertices,
            neighbors,
            partition1_size: self.partition1.len(),
            matched_vertices: 0,
            matching: vec![DUMMY; n + 1],
            dist: vec![0; self.partition1.len() + 1],
            queue: FixedSizeIntegerQueue::new(n.max(1)),
        };

        // warm start
        for u in 1..=state.partition1_size {
            for i in 0..state.neighbors[u].len() {
                let v = state.neighbors[u][i];
                if state.matching[v] == DUMMY {
                    state.matching[v] = u;
                    state.matching[u] = v;
                    state.matched_vertices += 1;
                    break;
                }
            }
        }

        while state.matched_vertices < state.partition1_size && state.bfs() {
            // Greedily search for vertex disjoint augmenting paths
            let mut v = 1;
            while v <= state.partition1_size && state.matched_vertices < state.partition1_size {
                if state.matching[v] == DUMMY && state.dfs(v) {
                    state.matched_vertices += 1;
                }
                v += 1;
            }
        }

        let mut edges: IndexSet<G::Edge> = IndexSet::new();
        for i in 0..state.vertices.len() {
            if state.matching[i] != DUMMY {
                let (a, b) = (state.vertices[i], state.vertices[state.matching[i]]);
                if let (Some(a), Some(b)) = (a, b) {
                    edges.insert(
                        graph
                            .get_edge(a, b)
                            .expect("matched vertices are adjacent")
                            .clone(),
                    );
                }
            }
        }
        let weight = edges.len() as f64;
        let _ = state.graph;
        Ok(Matching::new(graph, edges, weight))
    }
}
