# Public API renames

Old name -> new name. Generated from the merged rename pass (master `15af4b1`); applies to the Rust crates `jgrapht` and `jgrapht-io`.

## Types

| Old | New |
|---|---|
| `AHUForestIsomorphismInspector` | `AhuForestIsomorphismInspector` |
| `AHURootedTreeIsomorphismInspector` | `AhuRootedTreeIsomorphismInspector` |
| `AHUUnrootedTreeIsomorphismInspector` | `AhuUnrootedTreeIsomorphismInspector` |
| `ALTAdmissibleHeuristic` | `AltAdmissibleHeuristic` |
| `AhujaOrlinSharmaCapacitatedMinimumSpanningTree` | `AhujaOrlinSharma` |
| `AhujaOrlinSharmaCyclicExchangeLocalAugmentation` | `AhujaOrlinSharmaCyclicExchange` |
| `BFSShortestPath` | `BfsShortestPath` |
| `BoykovKolmogorovMFImpl` | `BoykovKolmogorov` |
| `CHManyToManyShortestPaths` | `ContractionHierarchyManyToMany` |
| `ChordalGraphMinimalVertexSeparatorFinder` | `MinimalVertexSeparatorFinder` |
| `ChristofidesThreeHalvesApproxMetricTSP` | `Christofides` |
| `DenseEdmondsMaximumCardinalityMatching` | `DenseEdmonds` |
| `DinicMFImpl` | `Dinic` |
| `EdgeBasedTwoApproxVcImpl` | `EdgeBasedTwoApproxVertexCover` |
| `EdmondsKarpMFImpl` | `EdmondsKarp` |
| `EsauWilliamsCapacitatedMinimumSpanningTree` | `EsauWilliams` |
| `FRLayoutAlgorithm2D` | `FrLayoutAlgorithm2D` |
| `FRQuadTree` | `FrQuadTree` |
| `FarthestInsertionHeuristicTSP` | `FarthestInsertionHeuristicTsp` |
| `FastutilMapGraph` | `MapGraph` |
| `FastutilMapIntVertexGraph` | `IntMapGraph` |
| `FastutilMapIntVertexSpecifics` | `IntMapSpecifics` |
| `FastutilMapSpecifics` | `MapSpecifics` |
| `GreedyHeuristicTSP` | `GreedyHeuristicTsp` |
| `HeldKarpTSP` | `HeldKarpTsp` |
| `HopcroftKarpMaximumCardinalityBipartiteMatching` | `HopcroftKarp` |
| `KuhnMunkresMinimalWeightBipartitePerfectMatching` | `KuhnMunkres` |
| `LinearizedChordDiagramGraphGenerator` | `LinearizedChordDiagramGenerator` |
| `MinimumCostFlowProblemImpl` | `MinimumCostFlowNetwork` |
| `MinimumSTCutAlgorithm` | `MinimumStCutAlgorithm` |
| `NearestInsertionHeuristicTSP` | `NearestInsertionHeuristicTsp` |
| `NearestNeighborHeuristicTSP` | `NearestNeighborHeuristicTsp` |
| `PushRelabelMFImpl` | `PushRelabel` |
| `RandomTourTSP` | `RandomTourTsp` |
| `SimpleWeightedBipartiteGraphMatrixGenerator` | `WeightedBipartiteMatrixGenerator` |
| `SimpleWeightedGraphMatrixGenerator` | `WeightedMatrixGenerator` |
| `SparseEdmondsMaximumCardinalityMatching` | `SparseEdmonds` |
| `TwoApproxMetricTSP` | `TwoApproxMetricTsp` |
| `TwoOptHeuristicTSP` | `TwoOptHeuristicTsp` |
| `VF2MappingIterator` | `Vf2MappingIterator` |

### Removed aliases (use the real type)

| Old | New |
|---|---|
| `BFSShortestPath` | `BfsShortestPath` |
| `AStarShortestPath` | `AStar` |
| `BellmanFordShortestPath` | `BellmanFord` |
| `DeltaSteppingShortestPath` | `DeltaStepping` |
| `DijkstraShortestPath` | `Dijkstra` |
| `TreeSingleSourcePaths` | `SingleSourcePaths` |

## Methods (inherent)

| Type | Old | New |
|---|---|---|
| TsplibImporter | `compute_2d_geographical_distance` | `geographical_distance_2d` |
| capacitated Solution | `calculate_resulting_spanning_tree` | `resulting_spanning_tree` |
| BergeGraphInspector, WeakChordalityInspector | `get_certificate` | `certificate` |
| ChinesePostman | `get_cpp_solution` | `cpp_solution` |
| ChordalityInspector | `get_perfect_elimination_order` | `perfect_elimination_order` |
| ChordalityInspector | `get_hole` | `hole` |
| MinimalVertexSeparatorFinder (was ChordalGraphMinimalVertexSeparatorFinder) | `get_minimal_separators` | `minimal_separators` |
| MinimalVertexSeparatorFinder | `get_minimal_separators_with_multiplicities` | `minimal_separators_with_multiplicities` |
| GoldbergMaximumDensitySubgraph | `calculate_densest` | `densest_subgraph` |
| EdmondsKarp, PushRelabel (inherent method) | `calculate_maximum_flow` | `maximum_flow_value` |
| CapacityScalingMinimumCostFlow (inherent method) | `get_minimum_cost_flow` | `solve_flow` |
| KolmogorovWeightedMatching, KolmogorovWeightedPerfectMatching | `get_error` | `error` |
| KolmogorovWeightedPerfectMatching | `get_dual_solution` | `dual_solution` |
| MaximumWeightBipartiteMatching | `get_potentials` | `potentials` |
| MaximumWeightBipartiteMatching | `get_matching_weight` | `matching_weight` |
| DynamicProgrammingPathSolver | `get_maximum_weight_matching` | `maximum_weight_matching` |
| SparseEdmonds | `get_odd_set_cover` | `odd_set_cover` |
| AllDirectedPaths | `get_all_paths` | `all_paths` |
| AllDirectedPaths | `get_all_paths_between` | `all_paths_between` |
| BellmanFord | `get_paths` | `paths_from` |
| BellmanFord | `get_path` | `path_between` |
| ContractionHierarchyPrecomputation | `compute_contraction_hierarchy` | `contraction_hierarchy` |
| TransitNodeRoutingPrecomputation | `compute_transit_node_routing` | `transit_node_routing` |
| SuurballeKDisjointShortestPaths, BhandariKDisjointShortestPaths, EppsteinKShortestPath, YenKShortestPath, BoundedPrunedYenKShortestPath | `get_paths` | `paths` |
| DefaultManyToManyShortestPaths, DijkstraManyToManyShortestPaths | `get_many_to_many_paths` | `many_to_many_paths` |
| MartinShortestPath, LabelCorrectingMultiObjectiveShortestPath | `get_paths` | `paths_from` |
| MartinShortestPath, LabelCorrectingMultiObjectiveShortestPath | `get_paths_between` | `paths_between` |
| ArcRingArena | `get_node` | `node` |
| Distributor (netgen) | `get_distribution` | `distribution` |

## Trait methods

| Trait | Old | New |
|---|---|---|
| `MatchingAlgorithm` | `get_matching` | `matching` |
| `HamiltonianCycleAlgorithm` | `get_tour` | `tour` |
| `HamiltonianPathAlgorithm` | `get_path` | `path` |
| `CycleBasisAlgorithm` | `get_cycle_basis` | `cycle_basis` |
| `EulerianCycleAlgorithm` | `get_eulerian_cycle` | `eulerian_cycle` |
| `MinimumCycleMeanAlgorithm` | `get_cycle_mean` | `cycle_mean` |
| `MinimumCycleMeanAlgorithm` | `get_cycle` | `cycle` |
| `Graph` | `get_edge` | `edge_between` |

## Network generator (`jgrapht::generate::netgen`)

Every identifier ending in or containing `_num` is now `_count` (for example `node_num` -> `node_count`, `max_source_to_tsource_arc_num` -> `max_source_to_tsource_arc_count`). Current names:

`arc_count`, `max_sink_to_all_arc_count`, `max_source_to_all_arc_count`, `max_source_to_sink_arc_count`, `max_source_to_tnode_arc_count`, `max_source_to_tsource_arc_count`, `max_tnode_to_sink_arc_count`, `max_tnode_to_tnode_arc_count`, `max_tnode_to_tsource_arc_count`, `max_transship_node_to_all_arc_count`, `max_tsink_to_sink_arc_count`, `max_tsink_to_tnode_arc_count`, `max_tsink_to_tsource_arc_count`, `maximum_arc_count`, `maximum_arc_count_for`, `maximum_arc_count_full`, `minimum_arc_count`, `minimum_arc_count_for`, `node_count`, `pure_sink_count`, `pure_source_count`, `sink_count`, `source_count`, `transship_node_count`, `transship_sink_count`, `transship_source_count`, `with_arc_count`, `with_node_count`, `with_sink_count`, `with_source_count`, `with_tsink_count`, `with_tsource_count`


The consuming setters of `NetworkGeneratorConfigBuilder` changed from `set_*` to `with_*` (for example `set_node_num` -> `with_node_count`). Current names:

`with_arc_count`, `with_bipartite_matching_problem_params`, `with_bipartite_matching_problem_params_cost`, `with_bipartite_matching_problem_params_full`, `with_max_cap`, `with_max_cost`, `with_maximum_flow_problem_params`, `with_maximum_flow_problem_params_cap`, `with_maximum_flow_problem_params_full`, `with_maximum_flow_problem_params_sources`, `with_min_cap`, `with_min_cost`, `with_node_count`, `with_params`, `with_percent_capacitated`, `with_percent_with_inf_cost`, `with_sink_count`, `with_source_count`, `with_total_supply`, `with_tsink_count`, `with_tsource_count`


## Unchanged on purpose

`MartinShortestPath`, `LabelCorrectingMultiObjectiveShortestPath`, `ContractionHierarchyBidirectionalDijkstra`, the random-generator names, the function/struct pairs (`greedy_coloring` and `GreedyColoring`, ...), and `&mut self` `set_*` methods.
