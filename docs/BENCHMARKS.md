# Benchmarks

Benchmarks (criterion, in `crates/jgrapht/benches/`) never run automatically: not on push, not on
pull requests, not on a schedule. Start them when you want them.

```sh
scripts/bench.sh                         # every benchmark of the working tree
scripts/bench.sh graph_perf              # one benchmark file
scripts/bench.sh -b master graph_perf    # also run master, fail if the tree is >30% slower
scripts/bench.sh -b master -t 1.15       # same, stricter threshold
cargo bench -p jgrapht --bench graph_perf   # plain criterion, no comparison
```

With `-b <ref>` the base revision is built in a temporary git worktree and both runs share one
target directory, so the comparison (`scripts/bench_gate.py`) uses the same machine and build
cache. Sub-microsecond benchmarks are ignored by the gate. On GitHub use the manual **Benchmarks**
workflow (Actions tab, Run workflow, or `gh workflow run bench.yml -f bench=graph_perf -f base=master`);
it accepts the same benchmark name, base ref and threshold.

| File | Ports and covers |
|---|---|
| `graph_perf` | `GraphPerformanceTest` (generate a 1000-vertex, 100000-edge graph, Dijkstra, Edmonds-Karp, strong connectivity) plus edge-lookup micro-benchmarks, `BaseGraph` against the sparse graph |
| `shortestpath_perf` | `DijkstraShortestPathPerformanceTest` and `BaseGraph` against sparse-graph Dijkstra and BFS |
| `algorithms_perf` | flow, spanning tree and maximal clique tests |
| `structures_perf` | LCA, tree dynamic connectivity, path-growing and Hopcroft-Karp matching, DAG Hamiltonian path |
| `pathsearch_perf` | Yen, bounded-pruned Yen, Eppstein, delta stepping, `AllDirectedPaths`, DAG and Hamiltonian path, Blossom V |

Not ported: the Java JMH-only variants (many-to-many, sandwich-prune, `GraphBuilders`) and the
memory-footprint parts of the graph tests. Memory is checked by `tests/sparse_memory.rs`.

## Reference numbers

`graph_perf` on a shared cloud container, 10 samples, 1000 vertices and 100000 directed weighted
edges (mean of one run; expect wide intervals on shared machines, so read the ratios, not the
digits):

| Benchmark | Time |
|---|---|
| `graph/generate` | 784 ms |
| `graph/dijkstra_path` | 13.4 ms |
| `graph/edmonds_karp` | 2.19 s |
| `graph/is_strongly_connected` | 34 ms |
| `graph/base_edge_endpoints_and_weight` (all 100000 edges) | 10.7 ms |
| `graph/sparse_edge_endpoints_and_weight` | 0.163 ms |
| `graph/base_out_edges_scan` | 13.6 ms |
| `graph/sparse_out_edges_scan` | 0.113 ms |
| `graph/base_edge_between_lookup` (10000 lookups) | 136 ms |

The micro-benchmarks show what the hash-based storage of `BaseGraph` costs: reading the endpoints
and weight of an edge takes about 100 ns on `BaseGraph` (hash lookups) against about 1.6 ns on the
sparse graph (array reads), roughly 65 times slower. `edges_between` is a large cost in this run
too, 14 us per lookup on the dense graph without fast lookups. Whether a dense-id storage strategy
for `BaseGraph` is worth adding is a decision for after the port is finished.

## Where the results are

- Locally: criterion prints a summary to the terminal and writes HTML reports to
  `target/criterion/report/index.html` (one page per benchmark, with the saved baselines `base` and
  `head` when `-b` was used).
- In the manual workflow: the job log of the **Benchmarks** run under the Actions tab.
- Reference numbers: the table above, updated by hand when a run is worth keeping.
