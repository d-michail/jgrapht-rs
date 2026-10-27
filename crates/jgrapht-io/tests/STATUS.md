# `jgrapht-io` port status

The sibling Java `jgrapht-io` checkout is the behavioral oracle. A format is **done** only after all
applicable upstream tests have been ported; **partial** means the public implementation exists but
the complete Java test matrix has not yet been brought across.

| Area | State | Current coverage |
|---|---|---|
| shared layer | partial | attributes and types, structured errors, importer/exporter traits, event stream, exporter providers, integer IDs |
| DIMACS | done | all 23 applicable `IntegerIdProvider`, event-driven importer, graph importer and exporter tests ported with the upstream Mycielski fixtures |
| CSV / DSV | done | all 32 applicable DSV, importer and exporter tests ported; Visio export, multiline quoting, custom delimiters and every matrix parameter combination covered |
| graph6 / sparse6 / digraph6 | partial | known graph6 encoding and round trips for simple, directed, looped and parallel-edge graphs |
| JSON | partial | strict hand-written parser, mixed IDs, typed/nested attributes, weights, factories, compact deterministic exporter |
| GML | partial | hand-written nested-list parser, vertices, edges and weights; attribute and parameter parity remains |
| Lemon | partial | base output, Java string escaping and edge weights |
| Matrix | partial | sparse adjacency, Laplacian and normalized Laplacian exporters |
| DOT | partial | hand-written lexer/parser for graph kind, IDs, comments, ports, chained edges, attribute skipping and basic subgraphs; scoped attribute parity remains |
| GraphML | partial | `quick-xml` streaming import, typed keys/defaults, configurable edge weights and deterministic basic export |
| GEXF | partial | `quick-xml` node/edge/weight import and deterministic basic export; declared attributes and visualization parity remain |
| TSPLIB | partial | coordinate instances, nine distance types, complete weighted graphs, metadata and embedded tours |

Current Rust integration tests: 79, including injected reader/writer failures and importer reuse.
The remaining work is to port the upstream tests format by
format and close every behavior gap they expose.
