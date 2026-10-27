# `jgrapht-io` port status

The sibling Java `jgrapht-io` checkout is the behavioral oracle. A format is **done** only after all
applicable upstream tests have been ported; **partial** means the public implementation exists but
the complete Java test matrix has not yet been brought across.

| Area | State | Current coverage |
|---|---|---|
| shared layer | partial | attributes and types, structured errors, importer/exporter traits, event stream, exporter providers, integer IDs |
| DIMACS | done | all 23 applicable `IntegerIdProvider`, event-driven importer, graph importer and exporter tests ported with the upstream Mycielski fixtures |
| CSV / DSV | done | all 32 applicable DSV, importer and exporter tests ported; Visio export, multiline quoting, custom delimiters and every matrix parameter combination covered |
| graph6 / sparse6 / digraph6 | done | all 30 upstream importer/exporter cases ported, including canonical encodings, large named graphs, pseudographs, factories and randomized round trips |
| JSON | done | all 23 upstream importer/exporter cases ported, including mixed IDs, typed and nested attributes, factories, escapes, null edge collections and custom collection names |
| GML | done | all 33 upstream importer/exporter cases ported, including attributes and nested lists, singletons, implicit vertices, graphics sections, parameters and reserved-key validation |
| Lemon | partial | base output, Java string escaping and edge weights |
| Matrix | partial | sparse adjacency, Laplacian and normalized Laplacian exporters |
| DOT | done | all 52 upstream importer/exporter/utility cases ported, including scoped defaults, nested endpoint subgraphs, ports, strict graphs, HTML values, factories, consumers and subgraph export |
| GraphML | partial | `quick-xml` streaming import, typed keys/defaults, configurable edge weights and deterministic basic export |
| GEXF | partial | `quick-xml` node/edge/weight import and deterministic basic export; declared attributes and visualization parity remain |
| TSPLIB | partial | coordinate instances, nine distance types, complete weighted graphs, metadata and embedded tours |

Current Rust integration tests: 217, including injected reader/writer failures and importer reuse.
The remaining work is to port the upstream tests format by
format and close every behavior gap they expose.
