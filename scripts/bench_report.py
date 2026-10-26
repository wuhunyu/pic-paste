#!/usr/bin/env python3
"""Compute reproducible latency summaries from a CSV of recorded milliseconds."""
import argparse
import csv
import json
import math
from pathlib import Path

def percentile(sorted_values, q):
    if not sorted_values:
        raise ValueError("empty samples")
    rank = (len(sorted_values) - 1) * q
    lower = math.floor(rank)
    upper = math.ceil(rank)
    return sorted_values[lower] + (sorted_values[upper] - sorted_values[lower]) * (rank - lower)

def summarize(path):
    with Path(path).open(newline="", encoding="utf-8") as file:
        rows = list(csv.DictReader(file))
    if not rows or not {"scenario", "latency_ms"}.issubset(rows[0]):
        raise ValueError("CSV requires scenario,latency_ms and non-empty samples")
    groups = {}
    for row in rows:
        value = float(row["latency_ms"])
        if not math.isfinite(value) or value < 0:
            raise ValueError("latency must be finite and non-negative")
        groups.setdefault(row["scenario"], []).append(value)
    return {
        name: {
            "samples": len(values),
            "p50_ms": round(percentile(sorted(values), 0.50), 2),
            "p95_ms": round(percentile(sorted(values), 0.95), 2),
            "max_ms": round(max(values), 2),
        }
        for name, values in sorted(groups.items())
    }

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("csv", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    result = json.dumps(summarize(args.csv), indent=2, ensure_ascii=False)
    if args.output:
        args.output.write_text(result + "\n", encoding="utf-8")
    else:
        print(result)
