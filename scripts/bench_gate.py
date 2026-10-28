#!/usr/bin/env python3
"""Summarize repeated criterion runs and compare two revisions statistically.

Every run is one independent `cargo bench -- --save-baseline run-<i>` process. The value of a run
is criterion's mean point estimate (ns) for a benchmark, so the statistics below are over
independent runs, not over the samples inside one run.

    bench_gate.py summary --dir target/criterion
    bench_gate.py compare --base target-base/criterion --head target/criterion --threshold 1.30

`compare` prints mean, standard deviation and coefficient of variation per revision, the ratio
head/base and a Welch t-test p-value. A benchmark is a REGRESSION when the head is slower than the
base by more than the threshold factor *and* the difference is significant (p < alpha); it is an
improvement in the opposite case. Benchmarks that exist in only one revision are listed but never
fail. Sub-microsecond benchmarks are ignored (--min-ns).
"""
import argparse
import json
import math
import pathlib
import statistics
import sys


def load_runs(root: pathlib.Path, prefix: str) -> dict:
    """benchmark name -> list of per-run mean estimates in ns."""
    result = {}
    for est in sorted(root.glob(f"**/{prefix}*/estimates.json")):
        run_dir = est.parent
        bench = str(run_dir.parent.relative_to(root))
        with est.open() as f:
            result.setdefault(bench, []).append(json.load(f)["mean"]["point_estimate"])
    return result


def betainc(a: float, b: float, x: float) -> float:
    """Regularized incomplete beta function I_x(a, b) (continued fraction, Numerical Recipes)."""
    if x <= 0.0:
        return 0.0
    if x >= 1.0:
        return 1.0
    ln_beta = math.lgamma(a + b) - math.lgamma(a) - math.lgamma(b)
    front = math.exp(ln_beta + a * math.log(x) + b * math.log1p(-x))

    def cf(a, b, x):
        tiny = 1e-300
        c, d = 1.0, 1.0 - (a + b) * x / (a + 1.0)
        d = 1.0 / (d if abs(d) > tiny else tiny)
        h = d
        for m in range(1, 300):
            m2 = 2 * m
            aa = m * (b - m) * x / ((a + m2 - 1.0) * (a + m2))
            d = 1.0 + aa * d
            d = 1.0 / (d if abs(d) > tiny else tiny)
            c = 1.0 + aa / (c if abs(c) > tiny else tiny)
            h *= d * c
            aa = -(a + m) * (a + b + m) * x / ((a + m2) * (a + m2 + 1.0))
            d = 1.0 + aa * d
            d = 1.0 / (d if abs(d) > tiny else tiny)
            c = 1.0 + aa / (c if abs(c) > tiny else tiny)
            delta = d * c
            h *= delta
            if abs(delta - 1.0) < 1e-14:
                break
        return h

    if x < (a + 1.0) / (a + b + 2.0):
        return front * cf(a, b, x) / a
    return 1.0 - front * cf(b, a, 1.0 - x) / b


def welch_p(xs, ys) -> float:
    """Two-sided p-value of Welch's t-test; 1.0 when the samples are too small or identical."""
    n1, n2 = len(xs), len(ys)
    if n1 < 2 or n2 < 2:
        return 1.0
    v1, v2 = statistics.variance(xs) / n1, statistics.variance(ys) / n2
    se2 = v1 + v2
    if se2 == 0.0:
        return 0.0 if statistics.mean(xs) != statistics.mean(ys) else 1.0
    t = (statistics.mean(xs) - statistics.mean(ys)) / math.sqrt(se2)
    df = se2**2 / ((v1**2 / (n1 - 1)) + (v2**2 / (n2 - 1)))
    return betainc(df / 2.0, 0.5, df / (df + t * t))


def describe(xs):
    mean = statistics.mean(xs)
    sd = statistics.stdev(xs) if len(xs) > 1 else 0.0
    return mean, sd, (100.0 * sd / mean if mean else 0.0)


def fmt_ns(ns: float) -> str:
    for unit, scale in (("s", 1e9), ("ms", 1e6), ("us", 1e3)):
        if ns >= scale:
            return f"{ns / scale:.3f} {unit}"
    return f"{ns:.1f} ns"


def summary(args) -> int:
    runs = load_runs(pathlib.Path(args.dir), args.prefix)
    if not runs:
        print(f"no runs named {args.prefix}* under {args.dir}", file=sys.stderr)
        return 2
    print(f"{'benchmark':<48} {'runs':>4} {'mean':>12} {'sd':>12} {'cv %':>6} {'min':>12} {'max':>12}")
    for name in sorted(runs):
        xs = runs[name]
        mean, sd, cv = describe(xs)
        print(f"{name:<48} {len(xs):>4} {fmt_ns(mean):>12} {fmt_ns(sd):>12} {cv:>6.1f} "
              f"{fmt_ns(min(xs)):>12} {fmt_ns(max(xs)):>12}")
    return 0


def compare(args) -> int:
    base = load_runs(pathlib.Path(args.base), args.prefix)
    head = load_runs(pathlib.Path(args.head), args.prefix)
    if not base or not head:
        print(f"no runs named {args.prefix}* under {args.base} or {args.head}", file=sys.stderr)
        return 2
    failures = []
    print(f"{'benchmark':<44} {'n':>3} {'base mean ± sd':>26} {'head mean ± sd':>26} {'ratio':>6} {'p':>7}")
    for name in sorted(base.keys() | head.keys()):
        if name not in base or name not in head:
            print(f"{name:<44} (only in {'head' if name in head else 'base'})")
            continue
        b, h = base[name], head[name]
        (bm, bsd, _), (hm, hsd, _) = describe(b), describe(h)
        ratio = hm / bm if bm else float("inf")
        p = welch_p(h, b)
        flag = ""
        if max(bm, hm) >= args.min_ns and p < args.alpha:
            if ratio > args.threshold:
                flag = "  REGRESSION"
                failures.append((name, ratio, p))
            elif ratio < 1.0 / args.threshold:
                flag = "  improvement"
        small = " (few runs)" if min(len(b), len(h)) < 10 else ""
        print(f"{name:<44} {min(len(b), len(h)):>3} {fmt_ns(bm) + ' ± ' + fmt_ns(bsd):>26} "
              f"{fmt_ns(hm) + ' ± ' + fmt_ns(hsd):>26} {ratio:>6.2f} {p:>7.3f}{flag}{small}")
    if failures:
        print(f"\n{len(failures)} benchmark(s) significantly slower than {args.threshold:.2f}x:", file=sys.stderr)
        for name, ratio, p in failures:
            print(f"  {name}: {ratio:.2f}x (p = {p:.3f})", file=sys.stderr)
        return 1
    print(f"\nno significant regression above {args.threshold:.2f}x (alpha = {args.alpha})")
    return 0


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--prefix", default="run-", help="baseline name prefix of the runs")
    s = sub.add_parser("summary", parents=[common])
    s.add_argument("--dir", default="target/criterion")
    s.set_defaults(fn=summary)
    c = sub.add_parser("compare", parents=[common])
    c.add_argument("--base", required=True, help="criterion directory of the base revision")
    c.add_argument("--head", required=True, help="criterion directory of the revision under test")
    c.add_argument("--threshold", type=float, default=1.30, help="slowdown factor that counts")
    c.add_argument("--alpha", type=float, default=0.05, help="significance level")
    c.add_argument("--min-ns", type=float, default=1000.0, help="ignore benchmarks faster than this")
    c.set_defaults(fn=compare)
    args = p.parse_args()
    return args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
