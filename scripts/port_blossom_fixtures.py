#!/usr/bin/env python3
"""Extract Blossom V edge fixtures from the upstream JGraphT Java tests."""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path


TRIPLE = re.compile(r"\{\s*(-?\d+)\s*,\s*(-?\d+)\s*,\s*(-?\d+)\s*\}")


@dataclass(frozen=True)
class WeightedCase:
    case_id: int
    edges: list[tuple[int, int, int]]
    minimum: int
    maximum: int
    pseudograph: bool


@dataclass(frozen=True)
class FailureCase:
    case_id: int
    edges: list[tuple[int, int, int]]
    extra_vertices: list[int]


def method_body(source: str, case_id: int) -> str:
    marker = f"void testGetMatching{case_id}("
    start = source.index(marker)
    opening = source.index("{", start)
    depth = 0
    for offset in range(opening, len(source)):
        if source[offset] == "{":
            depth += 1
        elif source[offset] == "}":
            depth -= 1
            if depth == 0:
                return source[opening + 1 : offset]
    raise ValueError(f"unterminated method for case {case_id}")


def edge_array(body: str) -> list[tuple[int, int, int]]:
    declaration = body.find("int[][] edges")
    if declaration < 0:
        return []
    opening = body.index("{", declaration)
    depth = 0
    for offset in range(opening, len(body)):
        if body[offset] == "{":
            depth += 1
        elif body[offset] == "}":
            depth -= 1
            if depth == 0:
                return [tuple(map(int, match)) for match in TRIPLE.findall(body[opening : offset + 1])]
    raise ValueError("unterminated edge array")


def weight(body: str, name: str) -> int | None:
    match = re.search(rf"double\s+{name}\s*=\s*(-?\d+)\s*;", body)
    return int(match.group(1)) if match else None


def weighted_cases(source: str, case_ids: range) -> list[WeightedCase]:
    cases = []
    for case_id in case_ids:
        body = method_body(source, case_id)
        minimum = weight(body, "minWeight")
        maximum = weight(body, "maxWeight")
        if minimum is None or maximum is None:
            continue
        cases.append(
            WeightedCase(
                case_id,
                edge_array(body),
                minimum,
                maximum,
                "WeightedPseudograph" in body,
            )
        )
    return cases


def failure_cases(source: str, case_ids: range) -> list[FailureCase]:
    cases = []
    for case_id in case_ids:
        body = method_body(source, case_id)
        if "assertThrows" not in body:
            continue
        cases.append(
            FailureCase(
                case_id,
                edge_array(body),
                [int(value) for value in re.findall(r"graph\.addVertex\((-?\d+)\)", body)],
            )
        )
    return cases


def rust_edges(edges: list[tuple[int, int, int]], indent: str = "        ") -> list[str]:
    if not edges:
        return [f"{indent}edges: &[],"]
    if len(edges) <= 4:
        values = ", ".join(f"({source}, {target}, {edge_weight}.)" for source, target, edge_weight in edges)
        return [f"{indent}edges: &[{values}],"]
    lines = [f"{indent}edges: &["]
    for source, target, edge_weight in edges:
        lines.append(f"{indent}    ({source}, {target}, {edge_weight}.),")
    lines.append(f"{indent}],")
    return lines


def emit_weighted(name: str, cases: list[WeightedCase]) -> list[str]:
    lines = [f"pub const {name}: &[WeightedCase] = &["]
    for case in cases:
        lines.extend(
            [
                "    WeightedCase {",
                f"        id: {case.case_id},",
                *rust_edges(case.edges),
                f"        minimum: {case.minimum}.,",
                f"        maximum: {case.maximum}.,",
                f"        pseudograph: {str(case.pseudograph).lower()},",
                "    },",
            ]
        )
    lines.extend(["];", ""])
    return lines


def emit_failures(cases: list[FailureCase]) -> list[str]:
    lines = ["pub const PERFECT_FAILURE_CASES: &[FailureCase] = &["]
    for case in cases:
        vertices = ", ".join(map(str, case.extra_vertices))
        lines.extend(
            [
                "    FailureCase {",
                f"        id: {case.case_id},",
                *rust_edges(case.edges),
                f"        extra_vertices: &[{vertices}],",
                "    },",
            ]
        )
    lines.extend(["];", ""])
    return lines


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("perfect_java", type=Path)
    parser.add_argument("general_java", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    perfect_source = args.perfect_java.read_text()
    general_source = args.general_java.read_text()
    perfect = [case for case in weighted_cases(perfect_source, range(51)) if case.case_id >= 16]
    general = [case for case in weighted_cases(general_source, range(1, 20)) if case.case_id >= 6]
    failures = failure_cases(perfect_source, range(51))

    assert [case.case_id for case in perfect] == [*range(16, 37), 38, 39, 40]
    assert [case.case_id for case in general] == list(range(6, 20))
    assert [case.case_id for case in failures] == list(range(41, 51))

    lines = [
        "//! Generated from JGraphT's public Blossom V tests by `scripts/port_blossom_fixtures.py`.",
        "//! Do not edit this file by hand.",
        "",
        "pub struct WeightedCase {",
        "    pub id: usize,",
        "    pub edges: &'static [(i32, i32, f64)],",
        "    pub minimum: f64,",
        "    pub maximum: f64,",
        "    pub pseudograph: bool,",
        "}",
        "",
        "pub struct FailureCase {",
        "    pub id: usize,",
        "    pub edges: &'static [(i32, i32, f64)],",
        "    pub extra_vertices: &'static [i32],",
        "}",
        "",
        *emit_weighted("PERFECT_WEIGHTED_CASES", perfect),
        *emit_weighted("GENERAL_WEIGHTED_CASES", general),
        *emit_failures(failures),
    ]
    args.output.write_text("\n".join(lines))


if __name__ == "__main__":
    main()
