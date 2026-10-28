# Guide for coding agents and contributors

jgrapht-rs is a Rust port of [JGraphT](https://jgrapht.org). This is a *port*, not a release
project: the goal is a faithful, idiomatic Rust library. Do not publish crates or create release
tags unless asked. See `README.md` for the overview.

## Layout

| Path | Contents |
|---|---|
| `crates/jgrapht/src/graph` | `Graph` traits, capability traits, `BaseGraph`, sparse (`graph::sparse`) and map graphs, builders |
| `crates/jgrapht/src/view` | view wrappers (`AsSubgraph`, `MaskSubgraph`, `GraphUnion`, ...) |
| `crates/jgrapht/src/alg` | algorithms, one module per JGraphT package; results are plain structs |
| `crates/jgrapht/src/{traverse,generate,util}` | traversals, generators, Java-compatible helpers (heaps, priority queue) |
| `crates/jgrapht-io` | importers and exporters, one module per format |
| `crates/*/tests` | ports of the Java tests; `tests/STATUS.md` in each crate maps Java test classes to Rust |
| `crates/jgrapht/benches` | criterion benchmarks (on demand only, see below) |

## Commands

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
cargo test -p jgrapht --test graph                   # one suite; also alg, generate, traverse, util, delta_stepping
cargo test -p jgrapht --features serde --test serde_roundtrip
cargo test -p jgrapht-io
cargo test --workspace                               # everything, several minutes; the alg suite alone takes about 7
cargo bench-graph                                    # benchmark aliases (.cargo/config.toml), only when asked for
scripts/bench.sh -b <base-ref> [bench...]            # compare with another revision
```

The workspace requires Rust 1.88 (edition 2024). Prefer the targeted suites while working and run the
whole workspace once before a pull request. CI also runs clippy with `-D warnings`, rustdoc with
`-D warnings` and `cargo fmt --check`.

## Patterns and best practices

These come from the design work of the port. Java is the oracle for *results, ordering and complexity*; it is not the
oracle for API shape. Do not copy Java exceptions, nullable results, stateful getters or numeric
sentinels when Rust can express the contract with traits, enums, `Option` or `Result`.

### Which type to return

| Situation | Representation |
|---|---|
| a value is normally absent (unreachable vertex, no cycle) | `Option<T>`; never `f64::INFINITY`, `-1` or an empty collection |
| an operation fails because of caller data or external state | `Result<T, GraphError>` |
| it can fail and its value can also be absent | `Result<Option<T>>` |
| a graph representation cannot support an operation | a capability trait it does not implement, so the call does not compile; `Result` only for genuinely dynamic capability |
| a value is valid only after another call ran | return a result object from the computation (`MaximumFlow`, `SpanningTree`, ...); no "compute, then read getters", no sentinel fields |
| the caller breaks a documented indexing contract | a panic is acceptable, as with slice indexing |
| an internal invariant is broken | `debug_assert!`, `expect` or a panic, but never reachable from valid public input |

- Do not add `Result` mechanically. Make invalid operations unrepresentable when practical.
- A function that returns `Result` or `Option` must not `unwrap` a fallible query on the caller's
  data; use `ok_or(GraphError::EdgeNotFound)?`, `?`, or `GraphError::Internal(..)` for state the
  algorithm itself should have computed.
- `try_` is only for a fallible alternative to an otherwise infallible operation. A panic wrapper
  around the real fallible operation does not get the shorter name.
- Public APIs do not use Java vocabulary (`null`, `cause`, `get_*`, exception messages).

### Errors

- Use typed variants (`InvalidParameter { name, requirement }`, `InvalidGraph(GraphProperty)`,
  `InvalidWeight(WeightProblem)`, `MissingParameter`, `MissingVertexData`, `VertexNotFound`, ...).
  Add a variant or a requirement/property when none fits; never `InvalidArgument(String)`.
  Helpers such as `GraphError::must_be_positive("name")` keep call sites short.
- Callers branch on variants, so tests match variants, never message text (unless an external
  format promises the text). Error enums are `#[non_exhaustive]`.
- Errors that carry a payload (Bellman-Ford's negative cycle) get their own type, derived with
  `thiserror`, with sources preserved. `IoError` is the boundary error of `jgrapht-io` and wraps
  graph and I/O errors.
- Validate arguments in constructors when the check is finite and does not change the documented
  complexity: start vertices, radii, weights, distributions. Then `next()` cannot fail.

### Graphs, capabilities and elements

- Ask for the narrowest bound: `Graph`, `OutgoingGraph`, `IncomingGraph`, `IncidenceGraph`,
  `DirectedGraph`, `UndirectedGraph`, `GraphMut`. Do not add run-time "unsupported" errors for what
  a bound can express.
- Missing elements: `edge_source`/`edge_target`/`edge_weight` give `None` for an unknown edge;
  `edges_between` and incidence queries are empty for an unknown vertex; an algorithm that needs a
  source, sink, root or terminal returns `VertexNotFound` when it is absent; mutations distinguish
  failure from "no change" with `Result<bool>` or `Result<Option<T>>`.
- Integrity contract of `Graph`: every edge yielded by `edges()` or an incidence iterator has
  endpoints in `vertices()` and a weight. Algorithms may rely on it internally but must not turn a
  violation into undefined behaviour; at I/O boundaries report an error instead.
- Vertices and edges are values (`Clone + Eq + Hash`). `Graph::vertices()`/`edges()` yield
  `VertexRef`/`EdgeRef`, which implement `Borrow<Vertex>`: call `.borrow()` or use
  `vertices_cloned()`/`edges_cloned()`. Do not apply mass mechanical `.borrow()` edits to code on
  concrete graph types; inference breaks (`Borrow` against `RefCell::borrow`).
- Views borrow their base graph and cannot go stale; caches are snapshots unless registered as
  listeners. Result objects are values and do not borrow the graph.
- Types with an invariant keep fields private and offer validated constructors (`GraphWalk::from_parts`
  returns `Result`); `pub(crate)` unchecked constructors are for library code that built the data.

### Iterators

- `Iterator::next` never swallows an error. Make iteration infallible by construction, or yield
  `Result<T>` so callers can `collect::<Result<Vec<_>>>()`.
- Traversals report an unreached vertex as `None`, not as infinity.

### Panics and documentation

- Both crates deny `missing_docs`. A doc comment says what the item does and under which
  conditions; it never restates the name ("Returns the len.").
- Public fallible functions have an `# Errors` section; intentional public panics have `# Panics`.
- Prefer `debug_assert!` for expensive internal consistency checks; keep `expect` messages precise
  when they would indicate a library bug.
- Remaining `unwrap`/`expect` sites are internal invariants (arena indices, links inside an
  algorithm, values computed a few lines earlier). Do not add new ones that depend on caller data.

### Randomness and ordering

- Randomised algorithms take any `RngCore`. Results need not equal Java's for the same seed; test
  properties and correctness, not Java's random stream.
- Tie-breaking and iteration order follow insertion order. Reproduce Java's `HashSet` order only
  if a ported test needs it, and record the deviation in the crate's `STATUS.md`.

### Serde

- The optional `serde` feature covers graphs, sparse graphs, views, walks and result types. Input
  is validated through the normal mutation rules (direction, loops, parallel edges, acyclicity,
  subset invariants). Function pointers (edge and vertex factories) are not serialized. Maps keyed
  by vertices are written as lists of pairs.

### Porting and tests

- Port faithfully first, refactor after: for large classes (Blossom V, Boyer-Myrvold, contraction
  hierarchies, VF2) port the Java into one module, keep the Java class name as the type name, port
  its tests, then clean up. These are the most order-sensitive tests.
- Keep Rust test modules mirroring the upstream test classes and record the mapping in the crate's
  `tests/STATUS.md`. `N/A` is only for Java-only constructs (serialization, reflection, synchronized
  wrappers, Java collections). Write down any deliberate deviation there.
- When an algorithm's complexity matters, add a scaling guard test (for example a 10,000-vertex
  path).
- New algorithms get a no-panic sweep: run them on random pseudographs (loops, parallel edges,
  zero, negative, NaN and infinite weights) and assert typed outcomes (`alg_no_panic_tests.rs`).
  Importers get malformed-input tests; exporters get a graph that breaks the integrity contract.
- File headers: new Rust files carry the dual license header used by the existing files.

### Definition of done for a change

- Public signatures follow the table above; describe breaking changes in the commit message and
  the pull request.
- Targeted tests pass, then the whole workspace once before a pull request; clippy and rustdoc with
  `-D warnings` and `cargo fmt --check` are clean.
- Status registers change only when fidelity or coverage changed.

### Non-goals

- Removing every internal `unwrap` and index from proven arena or dense-array code.
- Preserving Java class names, getter names or messages when they hurt Rust ergonomics (ported
  large algorithm types keep their names, see above).
- Changing Java-compatible results, tie-breaking or asymptotic complexity.
- Compile-time checking of data-dependent preconditions (weight signs, simplicity, completeness);
  these stay run-time errors.

## Benchmarks

Benchmarks never run automatically (not on push, pull request or schedule). Run them locally with
`scripts/bench.sh`, or through the manual **Benchmarks** workflow; criterion writes HTML reports to
`target/criterion/report/index.html`. They are noisy on shared machines; compare ratios on the same machine, not absolute numbers.

## Working on the repository

- Keep changes focused; update the rustdoc when behaviour users rely on changes, and the status registers when an item
  is closed.
- Do not leave generated or scratch files in the repository.
- Commit messages describe the change and its reason. Do not force-push shared branches; merge
  the base branch instead.
- Ask before anything outward-facing or hard to reverse (publishing, tags, deleting branches).
