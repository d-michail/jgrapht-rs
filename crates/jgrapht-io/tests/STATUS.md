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
| Lemon | done | all 5 upstream exporter cases ported, including default and weighted output, Java string escaping and parameter toggles |
| Matrix | done | all 3 upstream exporter cases ported, covering directed and undirected adjacency, Laplacian and normalized Laplacian output |
| DOT | done | all 52 upstream importer/exporter/utility cases ported, including scoped defaults, nested endpoint subgraphs, ports, strict graphs, HTML values, factories, consumers and subgraph export |
| GraphML | done | all 49 upstream exporter, graph importer, simple importer, event importer and integer edge-list cases ported, including typed defaults, nested graphs, validation, factories, consumers and registered attributes |
| GEXF | done | all 11 upstream exporter, simple importer and event importer cases ported, including typed declarations, metadata, nested-graph exclusion, consumers, options, labels, edge types and weights |
| TSPLIB | partial | coordinate instances, nine distance types, complete weighted graphs, metadata and embedded tours |

Current Rust integration tests: 285, including injected reader/writer failures and importer reuse.
The remaining work is to port the upstream tests format by
format and close every behavior gap they expose.
