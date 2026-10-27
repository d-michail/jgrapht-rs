use super::chordality::{ChordalityInspector, IterationOrder};
use crate::graph::{GraphExt, UndirectedGraph};
use indexmap::IndexMap;
use std::collections::HashMap;

/// `ChordalGraphMinimalVertexSeparatorFinder`: the minimal vertex separators of a chordal graph
/// with their multiplicities, read off a perfect elimination order.
///
/// A separator is a set of vertices; it is represented by the vector of its members sorted by
/// their position in the perfect elimination order, so equal sets have equal vectors.
pub struct ChordalGraphMinimalVertexSeparatorFinder<'g, G: UndirectedGraph> {
    graph: &'g G,
    chordality_inspector: ChordalityInspector<'g, G>,
    computed: bool,
    minimal_separators_with_multiplicities: Option<IndexMap<Vec<G::Vertex>, usize>>,
}

impl<'g, G: UndirectedGraph> ChordalGraphMinimalVertexSeparatorFinder<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        ChordalGraphMinimalVertexSeparatorFinder {
            graph,
            chordality_inspector: ChordalityInspector::with_order(graph, IterationOrder::Mcs),
            computed: false,
            minimal_separators_with_multiplicities: None,
        }
    }

    /// The minimal separators, or `None` if the graph is not chordal.
    pub fn get_minimal_separators(&mut self) -> Option<Vec<&Vec<G::Vertex>>> {
        self.lazy_compute();
        self.minimal_separators_with_multiplicities
            .as_ref()
            .map(|m| m.keys().collect())
    }

    /// The minimal separators with the number of times each occurs, or `None` if the graph is not
    /// chordal.
    pub fn get_minimal_separators_with_multiplicities(
        &mut self,
    ) -> Option<&IndexMap<Vec<G::Vertex>, usize>> {
        self.lazy_compute();
        self.minimal_separators_with_multiplicities.as_ref()
    }

    fn lazy_compute(&mut self) {
        if self.computed {
            return;
        }
        self.computed = true;
        if !self.chordality_inspector.is_chordal() {
            return;
        }
        let order: Vec<G::Vertex> = self
            .chordality_inspector
            .get_perfect_elimination_order()
            .expect("chordal graph")
            .to_vec();
        let vertex_in_order: HashMap<&G::Vertex, usize> =
            order.iter().enumerate().map(|(i, v)| (v, i)).collect();
        let mut result: IndexMap<Vec<G::Vertex>, usize> = IndexMap::new();
        let mut current: Vec<G::Vertex> = Vec::new();
        for vertex in order.iter().skip(1) {
            let previous_len = current.len();
            current = self.predecessors(&vertex_in_order, vertex);
            if current.len() <= previous_len {
                *result.entry(current.clone()).or_insert(0) += 1;
            }
        }
        self.minimal_separators_with_multiplicities = Some(result);
    }

    /// The neighbours earlier in the order, sorted by their position.
    fn predecessors(
        &self,
        vertex_in_order: &HashMap<&G::Vertex, usize>,
        vertex: &G::Vertex,
    ) -> Vec<G::Vertex> {
        let position = vertex_in_order[vertex];
        let mut predecessors: Vec<(usize, G::Vertex)> = Vec::new();
        for edge in self.graph.edges_of(vertex) {
            let opposite = self
                .graph
                .opposite_vertex(edge, vertex)
                .expect("edge of the graph");
            let dest = vertex_in_order[opposite];
            if dest < position && !predecessors.iter().any(|(d, _)| *d == dest) {
                predecessors.push((dest, opposite.clone()));
            }
        }
        predecessors.sort_by_key(|(d, _)| *d);
        predecessors.into_iter().map(|(_, v)| v).collect()
    }
}
