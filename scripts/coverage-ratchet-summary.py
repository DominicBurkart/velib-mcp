#!/usr/bin/env python3
"""Summarize tarpaulin-report.json for the coverage ratchet.

Subcommands:
  total   <report.json>              -- print integer count of uncovered lines
  summary <report.json> [root] [top] -- print ranked top-N uncovered files
                                        with line ranges and a paradigm heuristic

Heuristic (tuned for this Rust project; trivially extensible):
  - path contains /data/ or client/http/reqwest/fetch -> integration (I/O path)
  - path contains /server/ or /mcp/ or handler/route  -> integration (routing/protocol)
  - path ends in types.rs or contains serde/parse/
    convert/decode/encode/codec                       -> property (data transforms)
  - otherwise                                         -> unit (pure function)
"""
from __future__ import annotations
import json
import os
import re
import sys


def _fp(file_entry: dict, root_abs: str) -> str:
    parts = list(file_entry.get("path") or [])
    if parts and parts[0] == "/":
        joined = "/" + "/".join(parts[1:])
    else:
        joined = "/".join(parts)
    if joined.startswith(root_abs + "/"):
        return joined[len(root_abs) + 1 :]
    return joined.lstrip("/")


def _uncovered_lines(file_entry: dict) -> list[int]:
    out: list[int] = []
    for tr in file_entry.get("traces", []):
        line = tr.get("line")
        cov = tr.get("stats", {}).get("Line", 0)
        if isinstance(line, int) and isinstance(cov, int) and cov == 0:
            out.append(line)
    return sorted(set(out))


def _heuristic(path: str) -> str:
    p = path.lower()
    if re.search(r"(^|/)data(/|$)|client|http|reqwest|fetch", p):
        return "integration (I/O path)"
    if re.search(r"(^|/)server(/|$)|(^|/)mcp(/|$)|handler|route|router", p):
        return "integration (routing/protocol)"
    if re.search(r"types\.rs$|serde|parse|convert|decode|encode|codec", p):
        return "property (data transforms)"
    return "unit (pure function)"


def _ranges(lines: list[int]) -> str:
    if not lines:
        return ""
    out: list[str] = []
    start = prev = lines[0]
    for ln in lines[1:]:
        if ln == prev + 1:
            prev = ln
        else:
            out.append(f"{start}-{prev}" if start != prev else f"{start}")
            start = prev = ln
    out.append(f"{start}-{prev}" if start != prev else f"{start}")
    return ",".join(out)


def cmd_total(report_path: str) -> int:
    with open(report_path) as f:
        d = json.load(f)
    total = 0
    for file in d.get("files", []):
        total += len(_uncovered_lines(file))
    print(total)
    return 0


def cmd_summary(report_path: str, root: str, top_n: int) -> int:
    with open(report_path) as f:
        d = json.load(f)
    root_abs = os.path.abspath(root).rstrip("/")
    per_file: dict[str, list[int]] = {}
    total = 0
    for file in d.get("files", []):
        lines = _uncovered_lines(file)
        if not lines:
            continue
        per_file[_fp(file, root_abs)] = lines
        total += len(lines)
    ranked = sorted(per_file.items(), key=lambda kv: -len(kv[1]))
    print(f"TOTAL_UNCOVERED={total}")
    print("---")
    for fp, lines in ranked[:top_n]:
        print(
            f"{len(lines):4d}  {fp}  lines={_ranges(lines)}  "
            f"suggest={_heuristic(fp)}"
        )
    return 0


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(__doc__, file=sys.stderr)
        return 64
    cmd = argv[1]
    if cmd == "total":
        return cmd_total(argv[2])
    if cmd == "summary":
        root = argv[3] if len(argv) > 3 else os.getcwd()
        top_n = int(argv[4]) if len(argv) > 4 else 5
        return cmd_summary(argv[2], root, top_n)
    print(f"unknown subcommand: {cmd}", file=sys.stderr)
    return 64


if __name__ == "__main__":
    sys.exit(main(sys.argv))
