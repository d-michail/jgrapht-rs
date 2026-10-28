# Panic audit (roadmap R9)

Measured with `cargo clippy -p jgrapht --lib -- -W clippy::unwrap_used -W clippy::expect_used`:
747 production sites (398 `expect` + 279 `unwrap` on `Option`, 51 + 19 on `Result`), no `panic!`.
`jgrapht-io` reports none of its own after roadmap step 20.

The classification below is by family. It has not been checked site by site.

| Class | Where | Why it cannot be reached with valid input |
|---|---|---|
| Graph integrity contract | `edge_endpoints(..).unwrap()`, `opposite_vertex(..).unwrap()`, `weight_of` in `alg/*`, `traverse/cross.rs` | `Graph` documents that every yielded edge has vertex endpoints and a weight. A graph that breaks this is a defect in that `Graph` implementation, not a caller error. |
| Internal invariant | `alg/matching/blossom/*` (about 250, a port of Blossom V's pointer structures), heap/handle maps in `clustering.rs`, `arc_ring.rs`, `decomposition.rs`, `eppstein.rs`, queue and seen-map lookups in `traverse/cross.rs` | The value was inserted by the same routine a few lines earlier or by the algorithm's loop invariant. |
| Validated at construction | `scoring.rs` ("operands are non-negative"), `traverse/cross.rs` ("no start vertex to validate") | The constructor returns `Err` for the bad input; the later `expect` restates it. |
| Infallible by type | `.expect("breadth-first traversal cannot fail")` | The strategy's `Output` is `Infallible` for that traversal. |

Not done, so I-6 and I-8 stay open:

- Per-site records and `debug_assert!` conversion for the expensive Blossom checks.
- `# Errors` / `# Panics` sections on every public fallible API.
- `#![warn(missing_docs)]` for `jgrapht`: 686 public items are undocumented (304 methods, 169 associated
  functions, 81 struct fields, 66 functions, 25 variants; the largest groups are `generate/named.rs`,
  `traverse/cross.rs`, `generate/netgen/*`, `alg/interfaces.rs`). Enabling the lint before the backlog
  is cleared would break `clippy -D warnings` in CI.
- `clippy::indexing_slicing` has not been triaged.
