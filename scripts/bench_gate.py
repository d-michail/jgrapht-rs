#!/usr/bin/env python3
"""Benchmark regression gate: compare two criterion baselines recorded on the same machine.

Run both baselines into one target directory, then compare them:

    cargo bench -p jgrapht --benches -- --save-baseline base       # on the base revision
    cargo bench -p jgrapht --benches -- --save-baseline head       # on the revision under test
    python3 scripts/bench_gate.py base head --threshold 1.30

The script reads `target/criterion/**/<name>/estimates.json` (mean point estimate, in ns) and
fails when `head` is slower than `base` by more than the threshold factor. Benchmarks that exist in
only one baseline are listed but never fail the gate. Sub-microsecond benchmarks are ignored
(`--min-ns`): their noise exceeds any threshold worth enforcing.
"""
import argparse
import json
import pathlib
import sys


def load(root: pathlib.Path, name: str) -> dict:
    result = {}
    for est in root.glob(f"**/{name}/estimates.json"):
        bench = est.parent.parent.relative_to(root)
        with est.open() as f:
            result[str(bench)] = json.load(f)["mean"]["point_estimate"]
    return result


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    p.add_argument("base")
    p.add_argument("head")
    p.add_argument("--criterion-dir", default="target/criterion")
    p.add_argument("--threshold", type=float, default=1.30, help="fail above this slowdown factor")
    p.add_argument("--min-ns", type=float, default=1000.0, help="ignore benchmarks faster than this")
    args = p.parse_args()

    root = pathlib.Path(args.criterion_dir)
    base, head = load(root, args.base), load(root, args.head)
    if not base or not head:
        print(f"no estimates for baseline {args.base!r} or {args.head!r} under {root}", file=sys.stderr)
        return 2

    failures = []
    print(f"{'benchmark':<60} {'base ns':>14} {'head ns':>14} {'ratio':>7}")
    for name in sorted(base.keys() | head.keys()):
        if name not in base or name not in head:
            print(f"{name:<60} {'(only in ' + (args.head if name in head else args.base) + ')':>37}")
            continue
        b, h = base[name], head[name]
        ratio = h / b if b else float("inf")
        flag = ""
        if max(b, h) >= args.min_ns and ratio > args.threshold:
            flag = "  REGRESSION"
            failures.append((name, ratio))
        print(f"{name:<60} {b:>14.0f} {h:>14.0f} {ratio:>7.2f}{flag}")

    if failures:
        print(f"\n{len(failures)} benchmark(s) slower than {args.threshold:.2f}x:", file=sys.stderr)
        for name, ratio in failures:
            print(f"  {name}: {ratio:.2f}x", file=sys.stderr)
        return 1
    print("\nno regression above the threshold")
    return 0


if __name__ == "__main__":
    sys.exit(main())
