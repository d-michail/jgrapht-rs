#!/usr/bin/env python3
"""Rust API hardening audit (docs/RUST_API_ROADMAP.md, R0).

Subcommands:
  unwraps    report `unwrap`/`expect` applied directly to a fallible graph query in production
             code. Reporting-only until R4 completes; `--strict` makes findings fail the run.
  inventory  write the public API inventory (docs/API_INVENTORY.md) classifying every public
             function and trait method, and listing sentinels and call-order dependencies.
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = ["jgrapht", "jgrapht-io"]

# Graph queries that return `Result` today because of a storage capability.
FALLIBLE_QUERIES = [
    "edges_of",
    "in_edges_of",
    "out_edges_of",
    "degree_of",
    "in_degree_of",
    "out_degree_of",
    "try_neighbors_of",
    "try_successors_of",
    "try_predecessors_of",
    "try_incoming_of",
    "try_outgoing_of",
]
QUERY_CALL = re.compile(r"\.(" + "|".join(FALLIBLE_QUERIES) + r")\s*\(")
UNWRAP_CALL = re.compile(r"\s*\.\s*(unwrap|expect)\s*\(")


def rust_files(crate: str) -> list[Path]:
    return sorted((ROOT / "crates" / crate / "src").rglob("*.rs"))


def blank(text: str, start: int, end: int) -> str:
    """Replace [start, end) by spaces, keeping newlines so line numbers stay valid."""
    return text[:start] + re.sub(r"[^\n]", " ", text[start:end]) + text[end:]


def match_close(text: str, open_idx: int, open_ch: str, close_ch: str) -> int:
    depth = 0
    for i in range(open_idx, len(text)):
        if text[i] == open_ch:
            depth += 1
        elif text[i] == close_ch:
            depth -= 1
            if depth == 0:
                return i
    return len(text) - 1


def production_text(text: str) -> str:
    """Blank comments, string literals and `#[cfg(test)]` items."""
    out = text
    for m in re.finditer(r"//[^\n]*|\"(?:\\.|[^\"\\])*\"", text):
        out = blank(out, m.start(), m.end())
    while True:
        m = re.search(r"#\[cfg\(test\)\]", out)
        if not m:
            return out
        brace = out.find("{", m.end())
        semi = out.find(";", m.end())
        if brace == -1 or (semi != -1 and semi < brace):
            out = blank(out, m.start(), semi + 1)
        else:
            out = blank(out, m.start(), match_close(out, brace, "{", "}") + 1)


def line_of(text: str, idx: int) -> int:
    return text.count("\n", 0, idx) + 1


@dataclass
class Finding:
    path: Path
    line: int
    query: str
    kind: str


def find_unwraps() -> list[Finding]:
    findings = []
    for crate in CRATES:
        for path in rust_files(crate):
            text = production_text(path.read_text())
            for m in QUERY_CALL.finditer(text):
                close = match_close(text, m.end() - 1, "(", ")")
                u = UNWRAP_CALL.match(text, close + 1)
                if u:
                    findings.append(Finding(path, line_of(text, m.start()), m.group(1), u.group(1)))
    return findings


def cmd_unwraps(args: argparse.Namespace) -> int:
    findings = find_unwraps()
    for f in findings:
        print(f"{f.path.relative_to(ROOT)}:{f.line}: `{f.query}(..)` followed by `{f.kind}`")
    by_file = Counter(str(f.path.relative_to(ROOT)) for f in findings)
    print(f"\n{len(findings)} direct unwrap/expect call(s) on fallible graph queries "
          f"in {len(by_file)} file(s).")
    if args.strict and findings:
        return 1
    return 0


# ---------------------------------------------------------------- inventory

FN_SIG = re.compile(r"\b(pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+(\w+)")
SENTINELS = [
    ("INFINITY", re.compile(r"\b(?:f64::)?INFINITY\b")),
    ("-1", re.compile(r"(?<![\w.])-1(?:\.0)?(?![\w.])")),
    ("usize::MAX", re.compile(r"\busize::MAX\b")),
    ("NaN", re.compile(r"\bf64::NAN\b")),
]


@dataclass
class Item:
    path: str
    owner: str
    name: str
    sig: str
    ret: str
    klass: str
    sentinels: list[str]
    unwraps: int
    panics: int
    call_order: bool


def classify(name: str, ret: str, has_self: bool) -> str:
    r = re.sub(r"\s+", " ", ret)
    if re.match(r"(?:crate::)?(?:\w+::)*Result<\s*Option<", r) or "Result<Option<" in r.replace(" ", ""):
        return "Result<Option>"
    if re.search(r"\bResult<", r) or re.search(r"->\s*Result\b", "-> " + r):
        return "Result"
    if re.match(r"Option<", r):
        return "Option"
    if "impl Iterator" in r or "impl IntoIterator" in r or "Iterator<Item" in r:
        return "iterator"
    if not has_self and (name == "new" or name.startswith("new_") or name.startswith("with_")
                         or name.startswith("from_")) and r in ("Self", ""):
        return "unchecked constructor"
    if has_self and (name.startswith("is_") or name.startswith("has_") or name.startswith("contains")):
        return "infallible"
    if has_self and r:
        return "state query" if not name.startswith(("compute", "run")) else "infallible"
    return "infallible"


def split_return(sig: str) -> str:
    depth = 0
    for i in range(len(sig) - 1):
        c = sig[i]
        if c in "(<[":
            depth += 1
        elif c in ")>]" and not (c == ">" and sig[i - 1] == "-"):
            depth -= 1
        elif depth == 0 and sig[i : i + 2] == "->":
            ret = sig[i + 2 :]
            return re.split(r"\bwhere\b", ret)[0].strip()
    return ""


def block_spans(text: str, header: re.Pattern[str]) -> list[tuple[str, int, int]]:
    spans = []
    for m in header.finditer(text):
        brace = text.find("{", m.end())
        if brace == -1:
            continue
        spans.append((m.group(1), brace, match_close(text, brace, "{", "}")))
    return spans


def scan_file(path: Path, crate: str) -> list[Item]:
    text = production_text(path.read_text())
    rel = str(path.relative_to(ROOT / "crates"))
    traits = block_spans(text, re.compile(r"\bpub\s+trait\s+(\w+)[^{;]*(?=\{)"))
    impls = block_spans(text, re.compile(r"(?m)^\s*impl\b[^{;]*?\b(\w+)(?:<[^{]*?>)?\s*(?:where[^{]*)?(?=\{)"))
    items = []
    for m in FN_SIG.finditer(text):
        is_pub = bool(m.group(1)) and "pub(" not in (m.group(1) or "")
        owner = next((n for n, s, e in traits if s < m.start() < e), None)
        in_trait = owner is not None
        if not (is_pub or in_trait):
            continue
        brace_or_semi = re.compile(r"[{;]").search(text, m.end())
        # skip generic/where braces: signature ends at first `{` or `;` at depth 0
        i, depth = m.end(), 0
        while i < len(text):
            c = text[i]
            if c in "(<[":
                depth += 1
            elif c in ")]" or (c == ">" and text[i - 1] != "-"):
                depth -= 1
            elif depth <= 0 and c in "{;":
                break
            i += 1
        sig = re.sub(r"\s+", " ", text[m.start() : i]).strip()
        ret = split_return(sig)
        body = text[i : match_close(text, i, "{", "}") + 1] if i < len(text) and text[i] == "{" else ""
        params = sig[sig.find("(") :]
        has_self = bool(re.search(r"\(\s*&?\s*(?:mut\s+)?self\b", params))
        if owner is None:
            owner = next((n for n, s, e in impls if s < m.start() < e), "")
        sentinels = [label for label, rx in SENTINELS if rx.search(body) or rx.search(ret)]
        call_order = "Unsupported" in body and ("compute" in body or "first" in body or "before" in body)
        items.append(Item(
            path=rel, owner=owner, name=m.group(2), sig=sig, ret=ret,
            klass=classify(m.group(2), ret, has_self),
            sentinels=sentinels,
            unwraps=len(re.findall(r"\.\s*unwrap\s*\(\s*\)", body)),
            panics=len(re.findall(r"\.\s*expect\s*\(|\bpanic!\s*\(|\bunreachable!\s*\(|\bassert(?:_eq|_ne)?!\s*\(", body)),
            call_order=call_order,
        ))
    return items


def cmd_inventory(args: argparse.Namespace) -> int:
    per_crate = {c: [i for p in rust_files(c) for i in scan_file(p, c)] for c in CRATES}
    lines = [
        "# Public API inventory",
        "",
        "Generated by `scripts/api_audit.py inventory`; do not edit by hand. It is a heuristic,",
        "regex-based audit used by the API hardening roadmap (R0): classification is by return type",
        "and the sentinel/panic columns scan function bodies, so treat entries as leads to verify.",
        "",
        "Classes: `infallible`, `Option`, `Result`, `Result<Option>`, `iterator`, `state query`",
        "(a `&self` getter returning a plain value, possibly depending on earlier calls) and",
        "`unchecked constructor` (a `new`/`with_`/`from_` returning `Self`).",
        "",
        "## Summary",
        "",
        "| Crate | Public fns | " + " | ".join(KLASSES) + " |",
        "|---|---|" + "---|" * len(KLASSES),
    ]
    for c, items in per_crate.items():
        n = Counter(i.klass for i in items)
        lines.append(f"| `{c}` | {len(items)} | " + " | ".join(str(n[k]) for k in KLASSES) + " |")
    lines += ["", "## Numeric sentinels", "",
              "Public functions whose signature or body mentions `INFINITY`, `-1`, `usize::MAX` or `NaN`.",
              "Each needs an owner in R3/R5/R6.", ""]
    sent = [(c, i) for c, items in per_crate.items() for i in items if i.sentinels]
    lines += table(sent, lambda i: ", ".join(f"`{s}`" for s in i.sentinels), "Sentinel")
    lines += ["", "## Call-order dependencies", "",
              "Public functions that return `Unsupported` for a state the caller controls (heuristic).",
              "Each needs an owner in R5.", ""]
    lines += table([(c, i) for c, items in per_crate.items() for i in items if i.call_order],
                   lambda i: "call order", "Kind")
    lines += ["", "## Panic classification", "",
              "Public functions with `unwrap`/`expect`/`panic!`/`assert!` in the body. Classify each as",
              "invariant, documented indexing contract, third-party trait invariant or defect (R9).", ""]
    lines += table([(c, i) for c, items in per_crate.items() for i in items if i.unwraps or i.panics],
                   lambda i: f"{i.unwraps} unwrap, {i.panics} expect/panic/assert", "Count")
    lines += ["", "## All public functions", ""]
    for c, items in per_crate.items():
        lines += [f"### `{c}`", "", "| Location | Item | Class | Returns |", "|---|---|---|---|"]
        for i in sorted(items, key=lambda i: (i.path, i.owner, i.name)):
            ret = i.ret.replace("|", "\\|") or "`()`"
            lines.append(f"| `{i.path}` | `{qual(i)}` | {i.klass} | `{ret}` |" if i.ret
                         else f"| `{i.path}` | `{qual(i)}` | {i.klass} | `()` |")
        lines.append("")
    out = ROOT / "docs" / "API_INVENTORY.md"
    out.write_text("\n".join(lines))
    print(f"wrote {out.relative_to(ROOT)}: " + ", ".join(f"{c}={len(v)}" for c, v in per_crate.items()))
    return 0


KLASSES = ["infallible", "Option", "Result", "Result<Option>", "iterator", "state query",
           "unchecked constructor"]


def qual(i: Item) -> str:
    return f"{i.owner}::{i.name}" if i.owner else i.name


def table(rows, detail, head: str) -> list[str]:
    if not rows:
        return ["None found."]
    out = [f"| Location | Item | {head} |", "|---|---|---|"]
    for _, i in sorted(rows, key=lambda r: (r[1].path, r[1].owner, r[1].name)):
        out.append(f"| `{i.path}` | `{qual(i)}` | {detail(i)} |")
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    u = sub.add_parser("unwraps")
    u.add_argument("--strict", action="store_true", help="exit 1 when anything is found")
    u.set_defaults(fn=cmd_unwraps)
    sub.add_parser("inventory").set_defaults(fn=cmd_inventory)
    args = ap.parse_args()
    return args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
