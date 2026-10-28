#!/usr/bin/env bash
# Run the criterion benchmarks on demand; nothing runs them automatically.
#
#   scripts/bench.sh                         # all benchmarks of the working tree
#   scripts/bench.sh graph_perf              # one benchmark file (crates/jgrapht/benches/<name>.rs)
#   scripts/bench.sh -b master graph_perf    # also run <ref> and fail if the tree is >30% slower
#   scripts/bench.sh -b master -t 1.15       # same with a stricter threshold
#
# With -b the base revision is checked out in a temporary git worktree and both runs share one
# target directory, so the comparison (scripts/bench_gate.py) uses the same machine and build cache.
set -euo pipefail

base=""
threshold="1.30"
while getopts "b:t:h" opt; do
  case "$opt" in
    b) base="$OPTARG" ;;
    t) threshold="$OPTARG" ;;
    *) sed -n '2,10p' "$0"; exit 2 ;;
  esac
done
shift $((OPTIND - 1))

root="$(git rev-parse --show-toplevel)"
cd "$root"

args=(-p jgrapht)
if [ "$#" -eq 0 ]; then
  args+=(--benches)
else
  for name in "$@"; do args+=(--bench "$name"); done
fi

if [ -z "$base" ]; then
  cargo bench "${args[@]}"
  exit 0
fi

work="$(mktemp -d)"
trap 'git worktree remove --force "$work/base" 2>/dev/null || true; rm -rf "$work"' EXIT
git worktree add --detach "$work/base" "$base"
(cd "$work/base" && CARGO_TARGET_DIR="$root/target" cargo bench "${args[@]}" -- --save-baseline base)
cargo bench "${args[@]}" -- --save-baseline head
python3 scripts/bench_gate.py base head --threshold "$threshold"
