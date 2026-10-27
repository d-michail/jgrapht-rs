use super::base::{Matching, MatchingAlgorithm};
use crate::error::{GraphError, Result};
use crate::graph::{Graph, props};
use indexmap::IndexSet;
use std::cmp::Ordering;
use std::collections::HashSet;

fn invalid<T>(message: &str) -> Result<T> {
    Err(GraphError::InvalidArgument(message.into()))
}

/// `Double.compare(x, 0.) == 0`: positive zero only.
fn is_positive_zero(x: f64) -> bool {
    x.total_cmp(&0.0) == Ordering::Equal
}

/// `KuhnMunkresMinimalWeightBipartitePerfectMatching`: a perfect matching of minimum weight in a
/// complete bipartite graph with equally sized partitions, by the Hungarian method in `O(n^3)`.
pub struct KuhnMunkresMinimalWeightBipartitePerfectMatching<'g, G: Graph> {
    graph: &'g G,
    partition1: IndexSet<G::Vertex>,
    partition2: IndexSet<G::Vertex>,
}

impl<'g, G: Graph> KuhnMunkresMinimalWeightBipartitePerfectMatching<'g, G> {
    pub fn new(
        graph: &'g G,
        partition1: IndexSet<G::Vertex>,
        partition2: IndexSet<G::Vertex>,
    ) -> Self {
        KuhnMunkresMinimalWeightBipartitePerfectMatching {
            graph,
            partition1,
            partition2,
        }
    }
}

impl<G: Graph> MatchingAlgorithm<G::Vertex, G::Edge>
    for KuhnMunkresMinimalWeightBipartitePerfectMatching<'_, G>
{
    fn get_matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        let graph = self.graph;
        if self.partition1.len() != self.partition2.len() {
            return invalid("Graph supplied isn't complete bipartite with equally sized partitions!");
        }
        let p1: HashSet<G::Vertex> = self.partition1.iter().cloned().collect();
        let p2: HashSet<G::Vertex> = self.partition2.iter().cloned().collect();
        if !props::is_bipartite_partition(graph, &p1, &p2) {
            return invalid("Invalid bipartite partition provided");
        }
        let partition = self.partition1.len();
        if graph.edge_count() != partition * partition {
            return invalid("Graph supplied isn't complete bipartite with equally sized partitions!");
        }
        if !props::is_simple(graph) {
            return invalid("Only simple graphs supported");
        }

        let first_partition: Vec<&G::Vertex> = self.partition1.iter().collect();
        let second_partition: Vec<&G::Vertex> = self.partition2.iter().collect();

        let matching: Vec<usize> = if graph.vertex_count() == 0 {
            Vec::new()
        } else {
            KuhnMunkresMatrixImplementation::new(graph, &first_partition, &second_partition)
                .build_matching()
        };

        let mut edge_set: IndexSet<G::Edge> = IndexSet::new();
        let mut weight = 0.0;
        for (i, &j) in matching.iter().enumerate() {
            let e = graph
                .get_edge(first_partition[i], second_partition[j])
                .expect("complete bipartite graph");
            weight += graph.edge_weight(e).unwrap_or(1.0);
            edge_set.insert(e.clone());
        }
        Ok(Matching::new(graph, edge_set, weight))
    }
}

/// `KuhnMunkresMatrixImplementation`.
struct KuhnMunkresMatrixImplementation {
    cost_matrix: Vec<Vec<f64>>,
    excess_matrix: Vec<Vec<f64>>,
    rows_covered: Vec<bool>,
    columns_covered: Vec<bool>,
    column_matched: Vec<i64>,
    row_matched: Vec<i64>,
}

impl KuhnMunkresMatrixImplementation {
    fn new<G: Graph>(g: &G, s: &[&G::Vertex], t: &[&G::Vertex]) -> Self {
        let partition = s.len();
        let mut cost_matrix = vec![vec![0.0; partition]; partition];
        for (i, source) in s.iter().enumerate() {
            for (j, target) in t.iter().enumerate() {
                if source == target {
                    continue;
                }
                cost_matrix[i][j] = g
                    .edge_weight(g.get_edge(source, target).expect("complete bipartite graph"))
                    .unwrap_or(1.0);
            }
        }
        KuhnMunkresMatrixImplementation {
            cost_matrix,
            excess_matrix: Vec::new(),
            rows_covered: Vec::new(),
            columns_covered: Vec::new(),
            column_matched: Vec::new(),
            row_matched: Vec::new(),
        }
    }

    fn build_matching(&mut self) -> Vec<usize> {
        let height = self.cost_matrix.len();
        let width = self.cost_matrix[0].len();
        self.excess_matrix = self.make_excess_matrix();
        self.rows_covered = vec![false; height];
        self.columns_covered = vec![false; width];
        self.column_matched = vec![-1; height];
        self.row_matched = vec![-1; width];

        while self.build_maximal_matching() < width {
            self.build_vertex_coverage();
            self.extend_equality_graph();
        }
        self.column_matched.iter().map(|&j| j as usize).collect()
    }

    fn make_excess_matrix(&self) -> Vec<Vec<f64>> {
        let mut excess_matrix = self.cost_matrix.clone();
        for row in excess_matrix.iter_mut() {
            let mut cheapest_task_cost = f64::MAX;
            for &x in row.iter() {
                if cheapest_task_cost > x {
                    cheapest_task_cost = x;
                }
            }
            for x in row.iter_mut() {
                *x -= cheapest_task_cost;
            }
        }
        for j in 0..excess_matrix[0].len() {
            let mut cheapest_worker_cost = f64::MAX;
            for row in &excess_matrix {
                if cheapest_worker_cost > row[j] {
                    cheapest_worker_cost = row[j];
                }
            }
            for row in excess_matrix.iter_mut() {
                row[j] -= cheapest_worker_cost;
            }
        }
        excess_matrix
    }

    fn build_maximal_matching(&mut self) -> usize {
        let mut matching_size_lower_bound = self.column_matched.iter().filter(|&&c| c != -1).count();

        for j in 0..self.excess_matrix[0].len() {
            if self.row_matched[j] == -1 {
                for i in 0..self.excess_matrix.len() {
                    if self.excess_matrix[i][j] == 0.0 && self.column_matched[i] == -1 {
                        matching_size_lower_bound += 1;
                        self.column_matched[i] = j as i64;
                        self.row_matched[j] = i as i64;
                        break;
                    }
                }
            }
        }
        if matching_size_lower_bound == self.excess_matrix[0].len() {
            return matching_size_lower_bound;
        }

        let mut rows_visited = vec![false; self.excess_matrix.len()];
        let mut cols_visited = vec![false; self.excess_matrix[0].len()];
        let mut matching_size = 0;
        let mut extending = true;
        while extending && matching_size < self.excess_matrix.len() {
            rows_visited.fill(false);
            cols_visited.fill(false);
            extending = false;
            for j in 0..self.excess_matrix.len() {
                if self.row_matched[j] == -1 && !cols_visited[j] {
                    // try to extend matching
                    extending |= self.extend_matching_el(j, &mut rows_visited, &mut cols_visited);
                }
            }
            matching_size = self.row_matched.iter().filter(|&&r| r != -1).count();
        }
        matching_size
    }

    fn build_vertex_coverage(&mut self) {
        self.columns_covered.fill(false);
        self.rows_covered.fill(false);
        let mut invertible = vec![false; self.rows_covered.len()];
        for i in 0..self.excess_matrix.len() {
            if self.column_matched[i] != -1 {
                invertible[i] = true;
                continue;
            }
            for j in 0..self.excess_matrix[i].len() {
                if is_positive_zero(self.excess_matrix[i][j]) {
                    self.rows_covered[i] = true;
                    invertible[i] = true;
                    break;
                }
            }
        }
        let mut cont = true;
        while cont {
            for i in 0..self.excess_matrix.len() {
                if self.rows_covered[i] {
                    for j in 0..self.excess_matrix[i].len() {
                        if is_positive_zero(self.excess_matrix[i][j]) && !self.columns_covered[j] {
                            self.columns_covered[j] = true;
                        }
                    }
                }
            }
            cont = false;
            for j in 0..self.columns_covered.len() {
                if self.columns_covered[j] && self.row_matched[j] != -1 {
                    let r = self.row_matched[j] as usize;
                    if !self.rows_covered[r] {
                        cont = true;
                        self.rows_covered[r] = true;
                    }
                }
            }
        }
        for i in 0..self.rows_covered.len() {
            if invertible[i] {
                self.rows_covered[i] ^= true;
            }
        }
        debug_assert_eq!(self.uncovered(), 0);
        debug_assert!(self.minimal());
    }

    fn extend_equality_graph(&mut self) {
        let mut min_excess = f64::MAX;
        for i in 0..self.excess_matrix.len() {
            if self.rows_covered[i] {
                continue;
            }
            for j in 0..self.excess_matrix[i].len() {
                if self.columns_covered[j] {
                    continue;
                }
                if min_excess > self.excess_matrix[i][j] {
                    min_excess = self.excess_matrix[i][j];
                }
            }
        }
        for i in 0..self.excess_matrix.len() {
            if !self.rows_covered[i] {
                continue;
            }
            for j in 0..self.excess_matrix[i].len() {
                self.excess_matrix[i][j] += min_excess;
            }
        }
        for j in 0..self.excess_matrix[0].len() {
            if self.columns_covered[j] {
                continue;
            }
            for i in 0..self.excess_matrix.len() {
                self.excess_matrix[i][j] -= min_excess;
            }
        }
    }

    fn minimal(&self) -> bool {
        let matched = self.row_matched.iter().filter(|&&m| m != -1).count();
        let covered = self.rows_covered.iter().filter(|&&c| c).count()
            + self.columns_covered.iter().filter(|&&c| c).count();
        matched == covered
    }

    fn uncovered(&self) -> usize {
        let mut uncovered_zero = 0;
        for i in 0..self.excess_matrix.len() {
            if self.rows_covered[i] {
                continue;
            }
            for j in 0..self.excess_matrix[i].len() {
                if self.columns_covered[j] {
                    continue;
                }
                if is_positive_zero(self.excess_matrix[i][j]) {
                    uncovered_zero += 1;
                }
            }
        }
        uncovered_zero
    }

    /// `MatchExtender.extendMatchingOL`.
    fn extend_matching_ol(
        &mut self,
        path_tail_row: usize,
        path_tail_col: usize,
        rows_visited: &mut [bool],
        cols_visited: &mut [bool],
    ) -> bool {
        if self.column_matched[path_tail_row] == -1 {
            self.column_matched[path_tail_row] = path_tail_col as i64;
            self.row_matched[path_tail_col] = path_tail_row as i64;
            true
        } else {
            rows_visited[path_tail_row] = true;
            let matched_col = self.column_matched[path_tail_row] as usize;
            if cols_visited[matched_col] {
                return false;
            }
            let extending = self.extend_matching_el(matched_col, rows_visited, cols_visited);
            if extending {
                self.column_matched[path_tail_row] = path_tail_col as i64;
                self.row_matched[path_tail_col] = path_tail_row as i64;
            }
            extending
        }
    }

    /// `MatchExtender.extendMatchingEL`.
    fn extend_matching_el(
        &mut self,
        path_tail_col: usize,
        rows_visited: &mut [bool],
        cols_visited: &mut [bool],
    ) -> bool {
        cols_visited[path_tail_col] = true;
        for i in 0..self.excess_matrix.len() {
            if self.excess_matrix[i][path_tail_col] == 0.0 && !rows_visited[i] {
                // new tail to continue
                if self.extend_matching_ol(i, path_tail_col, rows_visited, cols_visited) {
                    return true;
                }
            }
        }
        false
    }
}
