use super::{BlossomVOptions, ObjectiveSense};
use super::{KolmogorovWeightedPerfectMatching, BLOSSOM_V_EPSILON};
use crate::alg::matching::{Matching, MatchingAlgorithm};
use crate::error::Result;
use crate::graph::{props, BaseGraph, Graph, GraphMut, UGraph, Undirected};
use crate::GraphType;
use indexmap::IndexSet;

/// Kolmogorov's reduction of general weighted matching to weighted perfect matching.
pub struct KolmogorovWeightedMatching<'g, G: Graph> {
    graph: &'g G,
    options: BlossomVOptions,
    objective_sense: ObjectiveSense,
    matching: Option<Matching<G::Vertex, G::Edge>>,
    error: Option<f64>,
}

impl<'g, G: Graph> KolmogorovWeightedMatching<'g, G> {
    /// Constructs a maximum-weight matching algorithm with default Blossom V options.
    pub fn new(graph: &'g G) -> Result<Self> {
        Self::with_options_and_objective(
            graph,
            BlossomVOptions::default(),
            ObjectiveSense::Maximize,
        )
    }

    /// Constructs an algorithm with default options and the requested objective.
    pub fn with_objective(graph: &'g G, objective_sense: ObjectiveSense) -> Result<Self> {
        Self::with_options_and_objective(graph, BlossomVOptions::default(), objective_sense)
    }

    /// Constructs a maximum-weight matching algorithm with custom Blossom V options.
    pub fn with_options(graph: &'g G, options: BlossomVOptions) -> Result<Self> {
        Self::with_options_and_objective(graph, options, ObjectiveSense::Maximize)
    }

    /// Constructs an algorithm with custom options and objective.
    pub fn with_options_and_objective(
        graph: &'g G,
        options: BlossomVOptions,
        objective_sense: ObjectiveSense,
    ) -> Result<Self> {
        props::require_undirected(graph)?;
        Ok(Self {
            graph,
            options,
            objective_sense,
            matching: None,
            error: None,
        })
    }

    /// Returns whether the reduced perfect matching satisfies the optimality conditions.
    pub fn test_optimality(&mut self) -> Result<bool> {
        Ok(self.get_error()? < BLOSSOM_V_EPSILON)
    }

    /// Returns the numerical optimality error of the reduced perfect matching.
    pub fn get_error(&mut self) -> Result<f64> {
        self.compute()?;
        Ok(self.error.expect("computed error"))
    }

    fn compute(&mut self) -> Result<()> {
        if self.matching.is_some() {
            return Ok(());
        }

        let vertices: Vec<_> = self.graph.vertices().cloned().collect();
        let original_edges: Vec<_> = self.graph.edges().cloned().collect();
        let n = vertices.len();
        let graph_type = GraphType::multigraph().as_weighted();
        let mut augmented: UGraph<usize, usize> =
            BaseGraph::<usize, usize, Undirected>::without_edge_factory(graph_type);
        for vertex in 0..2 * n {
            augmented.add_vertex(vertex);
        }

        let endpoint = |edge: &G::Edge| {
            let source = self.graph.edge_source(edge).expect("edge source");
            let target = self.graph.edge_target(edge).expect("edge target");
            let source = vertices
                .iter()
                .position(|vertex| vertex == source)
                .expect("source vertex");
            let target = vertices
                .iter()
                .position(|vertex| vertex == target)
                .expect("target vertex");
            (source, target)
        };

        let mut next_edge = 0;
        for edge in &original_edges {
            let (source, target) = endpoint(edge);
            if source == target {
                continue;
            }
            augmented.add_edge_with(&source, &target, next_edge)?;
            augmented.set_edge_weight(&next_edge, self.graph.edge_weight(edge).unwrap_or(1.0))?;
            next_edge += 1;
        }
        for vertex in 0..n {
            augmented.add_edge_with(&vertex, &(n + vertex), next_edge)?;
            augmented.set_edge_weight(&next_edge, 0.0)?;
            next_edge += 1;
        }
        let original_start = next_edge;
        let mut original_by_internal = Vec::new();
        for edge in &original_edges {
            let (source, target) = endpoint(edge);
            if source == target {
                continue;
            }
            augmented.add_edge_with(&(n + source), &(n + target), next_edge)?;
            augmented.set_edge_weight(&next_edge, self.graph.edge_weight(edge).unwrap_or(1.0))?;
            original_by_internal.push(edge.clone());
            next_edge += 1;
        }

        let mut perfect = KolmogorovWeightedPerfectMatching::with_options_and_objective(
            &augmented,
            self.options,
            self.objective_sense,
        )?;
        let reduced = perfect.get_matching()?;
        let error = perfect.get_error()?;
        let mut edges = IndexSet::new();
        for internal in reduced.edges() {
            if *internal >= original_start {
                edges.insert(original_by_internal[*internal - original_start].clone());
            }
        }
        let weight = reduced.weight() / 2.0;
        self.matching = Some(Matching::new(self.graph, edges, weight));
        self.error = Some(error);
        Ok(())
    }
}

impl<G: Graph> MatchingAlgorithm<G::Vertex, G::Edge> for KolmogorovWeightedMatching<'_, G> {
    fn get_matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        self.compute()?;
        Ok(self.matching.clone().expect("computed matching"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphMutExt, UGraph};

    fn exact_weight(weights: &[Vec<f64>], maximize: bool) -> f64 {
        fn solve(
            weights: &[Vec<f64>],
            mask: usize,
            maximize: bool,
            memo: &mut [Option<f64>],
        ) -> f64 {
            if mask == 0 {
                return 0.0;
            }
            if let Some(value) = memo[mask] {
                return value;
            }
            let first = mask.trailing_zeros() as usize;
            let rest = mask & !(1 << first);
            let mut best = solve(weights, rest, maximize, memo);
            let mut candidates = rest;
            while candidates != 0 {
                let second = candidates.trailing_zeros() as usize;
                candidates &= !(1 << second);
                let value =
                    weights[first][second] + solve(weights, rest & !(1 << second), maximize, memo);
                best = if maximize {
                    best.max(value)
                } else {
                    best.min(value)
                };
            }
            memo[mask] = Some(best);
            best
        }

        let mut memo = vec![None; 1 << weights.len()];
        solve(weights, (1 << weights.len()) - 1, maximize, &mut memo)
    }

    #[test]
    fn maximum_and_minimum_general_matching() {
        let mut graph = UGraph::<usize>::simple().weighted();
        for vertex in 0..5 {
            graph.add_vertex(vertex);
        }
        graph.add_weighted_edge(&0, &1, 9.0).unwrap();
        graph.add_weighted_edge(&1, &2, 8.0).unwrap();
        graph.add_weighted_edge(&2, &3, -4.0).unwrap();
        graph.add_weighted_edge(&3, &4, -6.0).unwrap();

        let mut maximum = KolmogorovWeightedMatching::new(&graph).unwrap();
        let mut minimum =
            KolmogorovWeightedMatching::with_objective(&graph, ObjectiveSense::Minimize).unwrap();
        assert_eq!(maximum.get_matching().unwrap().weight(), 9.0);
        assert_eq!(minimum.get_matching().unwrap().weight(), -6.0);
        assert!(maximum.test_optimality().unwrap());
        assert!(minimum.test_optimality().unwrap());
    }

    #[test]
    fn reduction_matches_exact_solver_for_all_options() {
        for seed in 0..4 {
            let n = 5;
            let mut graph = UGraph::<usize>::simple().weighted();
            let mut weights = vec![vec![0.0; n]; n];
            for vertex in 0..n {
                graph.add_vertex(vertex);
            }
            for source in 0..n {
                for target in source + 1..n {
                    let weight =
                        (((source + 2) * 13 + (target + 7) * 19 + seed * 23) % 31) as f64 - 15.0;
                    weights[source][target] = weight;
                    weights[target][source] = weight;
                    graph.add_weighted_edge(&source, &target, weight).unwrap();
                }
            }
            for options in BlossomVOptions::all_options() {
                for (objective, maximize) in [
                    (ObjectiveSense::Minimize, false),
                    (ObjectiveSense::Maximize, true),
                ] {
                    let mut algorithm = KolmogorovWeightedMatching::with_options_and_objective(
                        &graph, options, objective,
                    )
                    .unwrap();
                    let matching = algorithm.get_matching().unwrap();
                    assert_eq!(
                        matching.weight(),
                        exact_weight(&weights, maximize),
                        "seed={seed}, options={options}, objective={objective:?}"
                    );
                    let error = algorithm.get_error().unwrap();
                    assert!(
                        error < BLOSSOM_V_EPSILON,
                        "error={error}, seed={seed}, options={options}, objective={objective:?}"
                    );
                }
            }
        }
    }
}
