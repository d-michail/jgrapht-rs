#!/usr/bin/env bash
# Run the criterion benchmarks on demand; nothing runs them automatically.
#
#   scripts/bench.sh                          # all benchmarks, 10 independent runs, mean/sd per benchmark
#   scripts/bench.sh graph_perf               # one benchmark file (crates/jgrapht/benches/<name>.rs)
#   scripts/bench.sh -n 20 graph_perf         # more runs
#   scripts/bench.sh -b master graph_perf     # also run <ref>; interleaved runs, Welch t-test, fail on regression
#   scripts/bench.sh -b master -t 1.15        # stricter slowdown threshold (default 1.30)
#
# Everything runs strictly one process at a time on this machine: no parallel jobs, no other machine.
# One run is one separate `cargo bench` process (criterion's own 10 samples inside it). The value of
# a run is criterion's mean, and the statistics are over the runs, so at least 10 runs are required.
# With -b the base revision is built in a temporary git worktree with its own target directory and the
# runs alternate base, head, base, head, ... so that machine drift hits both revisions alike.
# Run on an idle machine; the numbers are only comparable on the same machine.
set -euo pipefail

base=""
threshold="1.30"
runs=10
while getopts "b:t:n:h" opt; do
  case "$opt" in
    b) base="$OPTARG" ;;
    t) threshold="$OPTARG" ;;
    n) runs="$OPTARG" ;;
    *) sed -n '2,14p' "$0"; exit 2 ;;
  esac
done
shift $((OPTIND - 1))

if [ "$runs" -lt 10 ]; then
  echo "need at least 10 runs for meaningful statistics (got $runs)" >&2
  exit 2
fi

root="$(git rev-parse --show-toplevel)"
cd "$root"

args=(-p jgrapht)
if [ "$#" -eq 0 ]; then
  args+=(--benches)
else
  for name in "$@"; do args+=(--bench "$name"); done
fi

# Fresh baselines, so old runs do not leak into the statistics.
find target/criterion -type d -name 'run-*' -prune -exec rm -rf {} + 2>/dev/null || true

if [ -z "$base" ]; then
  for i in $(seq 1 "$runs"); do
    echo "== run $i of $runs"
    cargo bench "${args[@]}" -- --save-baseline "run-$i"
  done
  python3 scripts/bench_gate.py summary --dir target/criterion
  exit 0
fi

work="$(mktemp -d)"
trap 'git worktree remove --force "$work/base" 2>/dev/null || true; rm -rf "$work"' EXIT
git worktree add --detach "$work/base" "$base"
export_base() { (cd "$work/base" && CARGO_TARGET_DIR="$work/target" cargo bench "${args[@]}" -- --save-baseline "run-$1"); }
for i in $(seq 1 "$runs"); do
  echo "== run $i of $runs: base"
  export_base "$i"
  echo "== run $i of $runs: head"
  cargo bench "${args[@]}" -- --save-baseline "run-$i"
done
python3 scripts/bench_gate.py compare --base "$work/target/criterion" --head target/criterion --threshold "$threshold"
