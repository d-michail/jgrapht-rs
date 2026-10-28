# Panic audit (roadmap R9)

Every production `unwrap`, `expect`, `panic!` and slice or array index in the two library crates is
listed in [`panic_sites.tsv`](panic_sites.tsv): 5419 clippy hits (`unwrap_used`, `expect_used`,
`panic`, `indexing_slicing` on `--lib`), of which 750 are `unwrap`/`expect` and none is `panic!`.
Indexing hits are grouped per function; each `unwrap`/`expect` has its own row. Regenerate with
`cargo clippy -p <crate> --lib -- -W clippy::unwrap_used -W clippy::expect_used -W clippy::panic -W clippy::indexing_slicing`.

| Class | Rows | Meaning |
|---|---|---|
| indexing contract | 4668 (grouped) | Dense array indexed by an `Indexed` position, vertex number or loop bound produced by the same routine. |
| internal invariant | 555 | The value was inserted, linked or computed earlier in the same routine, or a constructor was called with constant valid arguments. |
| third-party trait invariant | 196 | A `Graph` implementation yields edges that have vertex endpoints and a weight (documented on the trait), or a mutex was poisoned because another thread already panicked. |
| defect | 0 open | Two were found and fixed, see below. |

How the classes were assigned: the `unwrap`/`expect` sites were first sorted by rules on the
surrounding code, then the 258 that no rule covered were read one by one; the lazy-compute `Option`
fields, the list and tree node links in `tree_connectivity`, the Gusfield/Padberg-Rao tree and the
Eppstein path graph were checked against the code that fills them. Indexing sites were classified by
rule only. Reading all 4668 was not done; instead the claim that valid input cannot reach them is
tested (below).

## Defects found by the audit

- `HierholzerEulerianCycle::is_eulerian` unwrapped `try_is_eulerian`, which could no longer fail.
  `is_eulerian` is now infallible and `try_is_eulerian` is gone.
- `JsonImporter`: a `\u` escape followed by a multibyte character cut the four-byte window in the
  middle of the character and `hex4` panicked (`expect("ASCII hex bytes")`). It now returns
  `IoError::Import`.

## Evidence for "valid public input cannot trigger an undocumented panic"

- `crates/jgrapht-io/tests/malformed_input.rs` truncates, deletes and overwrites bytes of a seed
  document for each importer (CSV three formats, DIMACS, DOT, GEXF, GML, graph6, GraphML, JSON,
  TSPLIB) and fails on any panic. It reproduces the JSON defect above when the fix is removed.
- `tests/common/fixtures.rs` and the capability tests run algorithms on graphs that break the
  `Graph` integrity contract only through the exporters (which must report an error), and on empty,
  single-vertex, self-loop, parallel-edge and disconnected graphs.
- Public functions that can reach an `unwrap`, `expect` or `assert!` carry a `# Panics` section,
  and public functions returning `Result` carry `# Errors`. Both were generated from the function
  body, so they name the panic class and the error variants the body mentions, not the exact input.
  Trait methods declared without `pub` were not covered.
- The three explicit assertions (`next_int_bound`, `Queue::new`, `Queue::enqueue`) are cheap
  precondition checks on the caller's argument and stay as `assert!` with a `# Panics` section;
  there is no expensive consistency check left to turn into `debug_assert!`.

## Not covered

No algorithm-level mutation test exists comparable to the importer one; the algorithms rely on the
fixture tests above.
