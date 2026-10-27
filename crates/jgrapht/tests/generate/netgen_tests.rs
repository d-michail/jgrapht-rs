//! Ports of `org.jgrapht.generate.netgen` tests: DistributorTest, NetworkGeneratorConfigBuilderTest
//! and NetworkGeneratorTest.
//!
//! Differences: the builder remembers a failing setter and reports it from `build()`, so the tests
//! whose Java setter throws on its own check `build().is_err()`; `Distributor` takes the random
//! source per call; problems do not hold the graph.

use jgrapht::alg::flow::{MaximumFlowAlgorithm, MinimumCostFlowProblem, PushRelabelMFImpl};
use jgrapht::alg::matching::{HopcroftKarpMaximumCardinalityBipartiteMatching, MatchingAlgorithm};
use jgrapht::generate::netgen::*;
use jgrapht::util::java::JavaRandom;
use jgrapht::*;
use std::collections::HashSet;

mod distributor_test {
    use super::*;
    use std::collections::HashMap;

    fn rng() -> JavaRandom {
        JavaRandom::new(1)
    }

    fn lower_bounds<K: std::hash::Hash + Eq + 'static>(m: &[(K, i32)]) -> impl Fn(&K) -> i32 + '_ {
        move |k| m.iter().find(|(x, _)| x == k).map_or(0, |(_, v)| *v)
    }

    #[test]
    fn no_upper_bounds_one_valid_distribution() {
        let m = [("a", 3), ("b", 4), ("c", 2)];
        let mut d = Distributor::new();
        d.add_lower_bound(lower_bounds(&m));
        assert_eq!(
            d.get_distribution(&mut rng(), &["a", "b", "c"], 9).unwrap(),
            [3, 4, 2]
        );
    }

    #[test]
    fn no_upper_bounds_no_valid_distribution() {
        let m = [(1, 5), (2, 6), (3, 2)];
        let mut d = Distributor::new();
        d.add_lower_bound(lower_bounds(&m));
        assert!(d.get_distribution(&mut rng(), &[1, 2, 3], 12).is_err());
    }

    #[test]
    fn no_lower_bounds_one_valid_distribution() {
        let m = [(1, 3), (2, 5), (3, 2)];
        let mut d = Distributor::new();
        d.add_lower_bound(lower_bounds(&m));
        assert_eq!(
            d.get_distribution(&mut rng(), &[1, 2, 3], 10).unwrap(),
            [3, 5, 2]
        );
    }

    #[test]
    fn no_lower_bounds_no_valid_distribution() {
        let m = [(1, 3), (2, 4), (3, 2)];
        let mut d = Distributor::new();
        d.add_lower_bound(lower_bounds(&m));
        assert!(d.get_distribution(&mut rng(), &[1, 2, 3], 8).is_err());
    }

    fn all_bounds(lb: i32, ub: i32, elements: usize, values: i32, exact: Option<i32>) {
        let mut d = Distributor::new();
        d.add_lower_bound(|_: &usize| lb);
        d.add_upper_bound(|_: &usize| ub);
        let keys: Vec<usize> = (0..elements).collect();
        let dist = d.get_distribution(&mut rng(), &keys, values).unwrap();
        assert_eq!(dist.iter().map(|&x| x as i64).sum::<i64>(), values as i64);
        for v in dist {
            match exact {
                Some(e) => assert_eq!(v, e),
                None => assert!((lb..=ub).contains(&v)),
            }
        }
    }

    #[test]
    fn all_bounds_1() {
        all_bounds(5, 10, 10, 50, Some(5));
    }
    #[test]
    fn all_bounds_2() {
        all_bounds(5, 10, 10, 100, Some(10));
    }
    #[test]
    fn all_bounds_3() {
        all_bounds(5, 10, 10, 80, None);
    }
    #[test]
    fn all_bounds_large_bounds() {
        let (lb, ub) = (1000 * 1000, 2 * 1000 * 1000);
        all_bounds(lb, ub, 1000, ((lb + ub) / 2) * 1000, None);
    }

    #[test]
    fn unused_import_guard() {
        let _: HashMap<i32, i32> = HashMap::new();
    }
}

mod config_builder_test {
    use super::*;

    #[allow(clippy::too_many_arguments)]
    fn builder(
        n: i32,
        a: i32,
        s: i32,
        t: i32,
        ts: i32,
        tt: i32,
        supply: i32,
        min_cap: i32,
        max_cap: i32,
        min_cost: i32,
        max_cost: i32,
        pc: i32,
        pi: i32,
    ) -> NetworkGeneratorConfigBuilder {
        NetworkGeneratorConfigBuilder::new()
            .set_node_num(n)
            .set_arc_num(a)
            .set_source_num(s)
            .set_sink_num(t)
            .set_tsource_num(ts)
            .set_tsink_num(tt)
            .set_total_supply(supply)
            .set_min_cap(min_cap)
            .set_max_cap(max_cap)
            .set_min_cost(min_cost)
            .set_max_cost(max_cost)
            .set_percent_capacitated(pc)
            .set_percent_with_inf_cost(pi)
    }
    fn assignment() -> NetworkGeneratorConfigBuilder {
        builder(4, 4, 2, 2, 0, 0, 2, 1, 1, 0, 0, 100, 0)
    }
    fn min_cost_flow() -> NetworkGeneratorConfigBuilder {
        builder(10, 20, 2, 3, 1, 1, 50, 1, 10, 0, 10, 100, 0)
    }
    fn err(b: NetworkGeneratorConfigBuilder) {
        assert!(b.build().is_err());
    }
    fn ok(b: NetworkGeneratorConfigBuilder) {
        b.build().unwrap();
    }
    /// All parameters except the one a test leaves out.
    fn partial() -> NetworkGeneratorConfigBuilder {
        NetworkGeneratorConfigBuilder::new()
    }

    #[test]
    fn node_num() {
        err(partial()
            .set_arc_num(20)
            .set_source_num(2)
            .set_sink_num(3)
            .set_tsource_num(1)
            .set_tsink_num(1)
            .set_total_supply(50)
            .set_min_cap(1)
            .set_max_cap(10)
            .set_min_cost(0)
            .set_max_cost(10)
            .set_percent_capacitated(100)
            .set_percent_with_inf_cost(0));
        err(min_cost_flow().set_node_num(-1));
        err(min_cost_flow().set_node_num(MAX_NODE_NUM + 1));
    }

    #[test]
    fn arc_num() {
        err(partial()
            .set_node_num(10)
            .set_source_num(2)
            .set_sink_num(3)
            .set_tsource_num(1)
            .set_tsink_num(1)
            .set_total_supply(50)
            .set_min_cap(1)
            .set_max_cap(10)
            .set_min_cost(0)
            .set_max_cost(10)
            .set_percent_capacitated(100)
            .set_percent_with_inf_cost(0));
        err(min_cost_flow().set_arc_num(-1));
        err(min_cost_flow().set_arc_num(MAX_ARC_NUM + 1));
    }

    #[test]
    fn arc_num_in_assignment_problem() {
        err(assignment().set_arc_num(1));
        ok(assignment().set_arc_num(2));
        err(assignment().set_arc_num(5));
        ok(assignment().set_arc_num(4));
    }

    #[test]
    fn arc_num_in_min_cost_flow_problem() {
        err(min_cost_flow().set_arc_num(7));
        ok(min_cost_flow().set_arc_num(8));
        err(min_cost_flow().set_arc_num(9 + 8 + 40 + 8 + 1));
        ok(min_cost_flow().set_arc_num(9 + 8 + 40 + 8));
    }

    #[test]
    fn source_and_sink_num() {
        err(partial()
            .set_node_num(10)
            .set_arc_num(20)
            .set_sink_num(3)
            .set_tsource_num(1)
            .set_tsink_num(1)
            .set_total_supply(50)
            .set_min_cap(1)
            .set_max_cap(10)
            .set_min_cost(0)
            .set_max_cost(10)
            .set_percent_capacitated(100)
            .set_percent_with_inf_cost(0));
        err(min_cost_flow().set_source_num(-1));
        err(min_cost_flow().set_source_num(11));
        err(partial()
            .set_node_num(10)
            .set_arc_num(20)
            .set_source_num(2)
            .set_tsource_num(1)
            .set_tsink_num(1)
            .set_total_supply(50)
            .set_min_cap(1)
            .set_max_cap(10)
            .set_min_cost(0)
            .set_max_cost(10)
            .set_percent_capacitated(100)
            .set_percent_with_inf_cost(0));
        err(min_cost_flow().set_sink_num(-1));
        err(min_cost_flow().set_sink_num(11));
        err(min_cost_flow().set_source_num(5).set_sink_num(6));
    }

    #[test]
    fn transshipment_sources_and_sinks() {
        err(min_cost_flow().set_tsource_num(-1));
        err(min_cost_flow().set_tsource_num(3));
        err(min_cost_flow().set_tsink_num(-1));
        err(min_cost_flow().set_tsink_num(4));
    }

    #[test]
    fn supply() {
        err(partial()
            .set_node_num(10)
            .set_arc_num(20)
            .set_source_num(2)
            .set_sink_num(3)
            .set_tsource_num(1)
            .set_tsink_num(1)
            .set_min_cap(1)
            .set_max_cap(10)
            .set_min_cost(0)
            .set_max_cost(10)
            .set_percent_capacitated(100)
            .set_percent_with_inf_cost(0));
        err(min_cost_flow().set_total_supply(-1));
        err(min_cost_flow().set_total_supply(MAX_SUPPLY + 1));
        ok(min_cost_flow().set_total_supply(MAX_SUPPLY));
        err(min_cost_flow().set_total_supply(1));
    }

    #[test]
    fn capacities() {
        err(min_cost_flow().set_min_cap(-1));
        err(min_cost_flow().set_max_cap(-1));
        err(partial()
            .set_node_num(10)
            .set_arc_num(20)
            .set_source_num(2)
            .set_sink_num(3)
            .set_tsource_num(1)
            .set_tsink_num(1)
            .set_total_supply(50)
            .set_max_cap(10)
            .set_min_cost(0)
            .set_max_cost(10)
            .set_percent_capacitated(100)
            .set_percent_with_inf_cost(0));
        err(partial()
            .set_node_num(10)
            .set_arc_num(20)
            .set_source_num(2)
            .set_sink_num(3)
            .set_tsource_num(1)
            .set_tsink_num(1)
            .set_total_supply(50)
            .set_min_cap(1)
            .set_min_cost(0)
            .set_max_cost(10)
            .set_percent_capacitated(100)
            .set_percent_with_inf_cost(0));
        err(min_cost_flow().set_min_cap(CAPACITY_COST_BOUND + 1));
        ok(min_cost_flow()
            .set_min_cap(CAPACITY_COST_BOUND)
            .set_max_cap(CAPACITY_COST_BOUND));
        err(min_cost_flow().set_max_cap(CAPACITY_COST_BOUND + 1));
        ok(min_cost_flow().set_max_cap(CAPACITY_COST_BOUND));
        err(min_cost_flow().set_min_cap(10).set_max_cap(9));
    }

    #[test]
    fn costs() {
        err(min_cost_flow().set_min_cost(-CAPACITY_COST_BOUND - 1));
        err(min_cost_flow().set_max_cost(-CAPACITY_COST_BOUND - 1));
        err(min_cost_flow().set_min_cost(CAPACITY_COST_BOUND + 1));
        err(min_cost_flow().set_max_cost(CAPACITY_COST_BOUND + 1));
        err(min_cost_flow().set_min_cost(10).set_max_cost(9));
    }

    #[test]
    fn percentages() {
        err(min_cost_flow().set_percent_capacitated(-1));
        err(min_cost_flow().set_percent_capacitated(101));
        err(min_cost_flow().set_percent_with_inf_cost(-1));
        err(min_cost_flow().set_percent_with_inf_cost(101));
    }

    #[test]
    fn valid_configs() {
        ok(assignment());
        ok(min_cost_flow());
        assert!(assignment().build().unwrap().maximum_arc_num() > 0);
    }
}

mod network_generator_test {
    use super::*;

    const EPS: f64 = 1e-9;
    type G = DiGraph<i32>;
    type Gen = NetworkGenerator<i32, DefaultEdge>;

    fn new_graph() -> G {
        G::with_graph_type(GraphType {
            self_loops: true,
            ..GraphType::simple()
        })
        .weighted()
        .with_vertex_factory(|i| i as i32)
    }

    fn validate_network(
        g: &G,
        info: &NetworkInfo<i32, DefaultEdge>,
        config: &NetworkGeneratorConfig,
    ) {
        assert_eq!(info.pure_sources().len() as i32, config.pure_source_num());
        assert_eq!(
            info.transshipment_sources().len() as i32,
            config.transship_source_num()
        );
        assert_eq!(
            info.transshipment_nodes().len() as i32,
            config.transship_node_num()
        );
        assert_eq!(info.pure_sinks().len() as i32, config.pure_sink_num());
        assert_eq!(
            info.transshipment_sinks().len() as i32,
            config.transship_sink_num()
        );
        let classes = [
            info.pure_sources(),
            info.transshipment_sources(),
            info.transshipment_nodes(),
            info.pure_sinks(),
            info.transshipment_sinks(),
        ];
        for i in 1..classes.len() {
            for j in 0..i {
                let a: HashSet<_> = classes[i].iter().collect();
                assert!(classes[j].iter().all(|v| !a.contains(v)));
            }
        }
        for class in classes {
            assert!(class.iter().all(|v| g.contains_vertex(v)));
        }
        assert_eq!(g.edge_count(), config.arc_num() as usize);
        assert!(
            info.pure_sources()
                .iter()
                .all(|v| g.in_edges_of(v).unwrap().next().is_none())
        );
        assert!(
            info.pure_sinks()
                .iter()
                .all(|v| g.out_edges_of(v).unwrap().next().is_none())
        );
    }

    fn validate_capacities(
        g: &G,
        cap: impl Fn(&DefaultEdge) -> f64,
        info: &NetworkInfo<i32, DefaultEdge>,
        config: &NetworkGeneratorConfig,
    ) {
        let skeleton: HashSet<_> = info.skeleton_arcs().iter().collect();
        for e in g.edges() {
            if !skeleton.contains(e) {
                assert!(cap(e) + EPS >= config.min_cap() as f64);
                assert!(cap(e) - EPS <= config.max_cap() as f64);
            }
        }
    }

    fn validate_costs(g: &G, cost: impl Fn(&DefaultEdge) -> f64, config: &NetworkGeneratorConfig) {
        for e in g.edges() {
            assert!(cost(e) >= config.min_cost() as f64 - EPS);
            assert!(cost(e) <= config.max_cost() as f64 + EPS);
        }
    }

    fn compare<T: PartialEq + std::fmt::Debug>(
        g1: &G,
        g2: &G,
        f1: impl Fn(&DefaultEdge) -> T,
        f2: impl Fn(&DefaultEdge) -> T,
    ) {
        for e1 in g1.edges() {
            let (u, v) = (g1.edge_source(e1).unwrap(), g1.edge_target(e1).unwrap());
            let e2 = g2.get_edge(u, v).expect("arc missing in second graph");
            assert_eq!(f1(e1), f2(e2));
        }
    }

    fn assert_graphs_equal(g1: &G, g2: &G) {
        assert_eq!(
            g1.vertices().collect::<HashSet<_>>(),
            g2.vertices().collect::<HashSet<_>>()
        );
        assert_eq!(g1.edge_count(), g2.edge_count());
        for e1 in g1.edges() {
            let e2 = g2
                .get_edge(g1.edge_source(e1).unwrap(), g1.edge_target(e1).unwrap())
                .unwrap();
            assert!((g1.edge_weight(e1).unwrap() - g2.edge_weight(e2).unwrap()).abs() < EPS);
        }
    }

    fn test_min_cost(config: &NetworkGeneratorConfig, seed: i64) {
        let mut g1 = new_graph();
        let mut generator1 = Gen::new(config.clone(), seed);
        let p1 = generator1
            .generate_minimum_cost_flow_problem(&mut g1)
            .unwrap();
        let info1 = generator1.network_info().unwrap();
        validate_network(p1.graph(), info1, config);
        for source in info1.sources() {
            assert!(p1.node_supply(source) > 0);
        }
        for sink in info1.sinks() {
            assert!(p1.node_supply(sink) < 0);
        }
        validate_capacities(
            p1.graph(),
            |edge| p1.arc_capacity_upper_bound(edge) as f64,
            info1,
            config,
        );
        validate_costs(p1.graph(), |edge| p1.arc_cost(edge), config);

        let mut g2 = new_graph();
        let mut generator2 = Gen::new(config.clone(), seed);
        let p2 = generator2
            .generate_minimum_cost_flow_problem(&mut g2)
            .unwrap();
        assert_graphs_equal(p1.graph(), p2.graph());
        for v in p1.graph().vertices() {
            assert_eq!(p1.node_supply(v), p2.node_supply(v));
        }
        compare(
            p1.graph(),
            p2.graph(),
            |e| p1.arc_capacity_lower_bound(e),
            |e| p2.arc_capacity_lower_bound(e),
        );
        compare(
            p1.graph(),
            p2.graph(),
            |e| p1.arc_capacity_upper_bound(e),
            |e| p2.arc_capacity_upper_bound(e),
        );
        compare(
            p1.graph(),
            p2.graph(),
            |e| p1.arc_cost(e),
            |e| p2.arc_cost(e),
        );
    }

    fn gen_max_flow(
        config: &NetworkGeneratorConfig,
        seed: i64,
    ) -> (G, MaximumFlowProblem<i32, DefaultEdge>) {
        let mut g = new_graph();
        let mut generator = Gen::new(config.clone(), seed);
        let problem = generator.generate_max_flow_problem(&mut g).unwrap();
        validate_network(&g, generator.network_info().unwrap(), config);
        validate_capacities(
            &g,
            |e| problem.capacity(e),
            generator.network_info().unwrap(),
            config,
        );
        (g, problem)
    }

    fn test_max_flow(config: &NetworkGeneratorConfig, seed: i64) {
        let (g1, mut p1) = gen_max_flow(config, seed);
        let (g2, p2) = gen_max_flow(config, seed);
        assert_graphs_equal(&g1, &g2);
        assert_eq!(p1.sources, p2.sources);
        assert_eq!(p1.sinks, p2.sinks);
        compare(
            &g1,
            &g2,
            |e| p1.capacity(e).to_bits(),
            |e| p2.capacity(e).to_bits(),
        );
        // max flow value
        let mut g = g1;
        let converted = p1.to_single_source_single_sink_problem(&mut g).unwrap();
        converted.dump_capacities(&mut g).unwrap();
        let flow = PushRelabelMFImpl::new(&g)
            .unwrap()
            .maximum_flow_value(converted.source(), converted.sink())
            .unwrap();
        assert!(flow + EPS > config.total_supply() as f64);
        p1.sources.clear();
    }

    fn gen_bipartite(
        config: &NetworkGeneratorConfig,
        seed: i64,
    ) -> (G, BipartiteMatchingProblem<i32, DefaultEdge>) {
        let mut g = new_graph();
        let mut generator = Gen::new(config.clone(), seed);
        let problem = generator
            .generate_bipartite_matching_problem(&mut g)
            .unwrap();
        validate_network(&g, generator.network_info().unwrap(), config);
        validate_costs(&g, |e| problem.cost(e), config);
        (g, problem)
    }

    fn test_bipartite(config: &NetworkGeneratorConfig, seed: i64) {
        let (g1, p1) = gen_bipartite(config, seed);
        let (g2, p2) = gen_bipartite(config, seed);
        // feasibility: a perfect matching exists
        let undirected = AsUndirected(&g1);
        let matching = HopcroftKarpMaximumCardinalityBipartiteMatching::new(
            &undirected,
            p1.partition1.iter().copied(),
            p1.partition2.iter().copied(),
        )
        .unwrap()
        .get_matching()
        .unwrap();
        assert!(matching.is_perfect());
        assert_graphs_equal(&g1, &g2);
        assert_eq!(p1.partition1, p2.partition1);
        assert_eq!(p1.partition2, p2.partition2);
        compare(&g1, &g2, |e| p1.cost(e).to_bits(), |e| p2.cost(e).to_bits());
    }

    fn rng() -> JavaRandom {
        JavaRandom::new(1)
    }

    fn mcf_config(
        s: i64,
        ts: i64,
        tn: i64,
        tt: i64,
        t: i64,
        arcs: i64,
        supply: i32,
    ) -> NetworkGeneratorConfig {
        NetworkGeneratorConfigBuilder::new()
            .set_params(
                (s + tn + t) as i32,
                arcs as i32,
                s as i32,
                t as i32,
                ts as i32,
                tt as i32,
                supply,
                1,
                100,
                1,
                100,
                100,
                0,
            )
            .build()
            .unwrap()
    }

    #[test]
    fn min_cost_flow_minimum_arc_num() {
        let mut rng = rng();
        for s in 1..4i64 {
            for ts in 0..=s {
                for t in 1..4i64 {
                    for tt in 0..=t {
                        for tn in [0i64, 1, 2, 5, 10, 20, 30] {
                            let arcs = NetworkGeneratorConfig::minimum_arc_num_for(s, tn, t);
                            let config = mcf_config(s, ts, tn, tt, t, arcs, s.max(t) as i32);
                            test_min_cost(&config, rng.next_long());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn min_cost_flow_maximum_arc_num() {
        let mut rng = rng();
        for s in 1..4i64 {
            for ts in 0..=s {
                for t in 1..4i64 {
                    for tt in 0..=t {
                        for tn in [0i64, 1, 2, 5, 10, 20, 30] {
                            let arcs =
                                NetworkGeneratorConfig::maximum_arc_num_full(s, ts, tn, tt, t);
                            let config = mcf_config(s, ts, tn, tt, t, arcs, 10 * s as i32);
                            test_min_cost(&config, rng.next_long());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn min_cost_flow_minimum_arc_num_a() {
        let (s, tn, t) = (2i64, 1i64, 3i64);
        let arcs = NetworkGeneratorConfig::minimum_arc_num_for(s, tn, t);
        test_min_cost(
            &mcf_config(s, 0, tn, 0, t, arcs, 10 * s as i32),
            rng().next_long(),
        );
    }

    fn max_flow_config(s: i64, tn: i64, t: i64, arcs: i64) -> NetworkGeneratorConfig {
        NetworkGeneratorConfigBuilder::new()
            .set_maximum_flow_problem_params_sources(
                (s + tn + t) as i32,
                arcs as i32,
                10 * s as i32,
                1,
                100,
                s as i32,
                t as i32,
            )
            .build()
            .unwrap()
    }

    #[test]
    fn max_flow_minimum_number_of_arcs() {
        let mut rng = rng();
        for s in 1..5i64 {
            for t in 1..5i64 {
                for tn in 0..30i64 {
                    let arcs = NetworkGeneratorConfig::minimum_arc_num_for(s, tn, t);
                    test_max_flow(&max_flow_config(s, tn, t, arcs), rng.next_long());
                }
            }
        }
    }

    #[test]
    fn max_flow_maximum_number_of_arcs() {
        let mut rng = rng();
        for s in 1..5i64 {
            for t in 1..5i64 {
                for tn in 0..30i64 {
                    let arcs = NetworkGeneratorConfig::maximum_arc_num_for(s, tn, t);
                    test_max_flow(&max_flow_config(s, tn, t, arcs), rng.next_long());
                }
            }
        }
    }

    #[test]
    fn max_flow_random_number_of_arcs() {
        let mut rng = rng();
        for s in 1..5i64 {
            for t in 1..5i64 {
                for tn in 0..30i64 {
                    let lb = NetworkGeneratorConfig::minimum_arc_num_for(s, tn, t);
                    let ub = NetworkGeneratorConfig::maximum_arc_num_for(s, tn, t);
                    let arcs = rng.next_int_bound((ub - lb + 1) as i32) as i64 + lb;
                    test_max_flow(&max_flow_config(s, tn, t, arcs), rng.next_long());
                }
            }
        }
    }

    fn bipartite_config(i: i32, arcs: i32) -> NetworkGeneratorConfig {
        NetworkGeneratorConfigBuilder::new()
            .set_bipartite_matching_problem_params_cost(2 * i, arcs, 1, 100)
            .build()
            .unwrap()
    }

    #[test]
    fn bipartite_matching_minimum_number_of_arcs() {
        let mut rng = rng();
        for i in 1..50 {
            test_bipartite(&bipartite_config(i, i), rng.next_long());
        }
    }

    #[test]
    fn bipartite_matching_maximum_number_of_arcs() {
        let mut rng = rng();
        for i in 1..50 {
            test_bipartite(&bipartite_config(i, i * i), rng.next_long());
        }
    }

    #[test]
    fn bipartite_matching_random_number_of_arcs() {
        let mut rng = rng();
        for i in 1..50 {
            let arcs = rng.next_int_bound(i * i - i + 1) + i;
            test_bipartite(&bipartite_config(i, arcs), rng.next_long());
        }
    }

    #[test]
    fn bipartite_matching_large_problem() {
        let config = NetworkGeneratorConfigBuilder::new()
            .set_bipartite_matching_problem_params_cost(1000, 250000, 1, 100)
            .build()
            .unwrap();
        test_bipartite(&config, rng().next_long());
    }
}

/// Output of JGraphT's `NetworkGenerator` (seed 7, see the config below), sorted; produced by
/// running the Java implementation.
#[test]
fn matches_jgrapht_for_a_fixed_seed() {
    let c = NetworkGeneratorConfigBuilder::new()
        .set_params(30, 120, 3, 4, 1, 2, 500, 1, 100, -5, 50, 80, 10)
        .build()
        .unwrap();
    let mut g = DiGraph::<i32>::with_graph_type(GraphType {
        self_loops: true,
        ..GraphType::simple()
    })
    .weighted()
    .with_vertex_factory(|i| i as i32);
    let mut generator = NetworkGenerator::<i32, DefaultEdge>::new(c, 7);
    let p = generator
        .generate_minimum_cost_flow_problem(&mut g)
        .unwrap();
    let graph = p.graph();
    let mut lines: Vec<String> = graph
        .edges()
        .map(|e| {
            format!(
                "{}->{} cap={} cost={}",
                graph.edge_source(e).unwrap(),
                graph.edge_target(e).unwrap(),
                p.arc_capacity_upper_bound(e),
                p.arc_cost(e)
            )
        })
        .collect();
    lines.extend(
        graph
            .vertices()
            .map(|v| format!("s{v}={}", p.node_supply(v))),
    );
    lines.sort();
    let expected = include_str!("fixtures/netgen_seed7.txt");
    assert_eq!(lines.join("\n"), expected.trim_end());
}
