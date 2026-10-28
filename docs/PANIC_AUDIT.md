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

Documentation state after the final pass:

- `#![deny(missing_docs)]` is on in both crates.
- Every public function returning `Result` has an `# Errors` section, and every public function whose
  body can `unwrap`, `expect`, `panic!` or `assert!` has a `# Panics` section. Both were added by a
  script from the function body, so the text names the error variants the body mentions and the
  panic class above rather than the exact condition. Trait methods without `pub` were not covered.
- `clippy::indexing_slicing` reports 4597 sites in `jgrapht`, mostly in `blossom/*`, `planar.rs`,
  `scoring.rs`, `color.rs` and `connectivity.rs`. They index dense arrays by an `Indexed` position
  or a vertex number that the same routine produced, so they belong to the internal-invariant class.
  They were not individually reviewed.
- Converting expensive internal checks to `debug_assert!` was not done.
