#!/usr/bin/env python3
"""Evaluate *recorded* OCR predictions against manually transcribed references.

Input CSV columns: id,language,expected,actual. language is zh or mixed.
No OCR is run here. Do not treat fixture data or CI as device evidence.
"""
import argparse
import csv
import json
from pathlib import Path


def character_errors(expected: str, actual: str) -> int:
    """Levenshtein edit distance in Unicode code points (not UTF-8 bytes)."""
    previous = list(range(len(actual) + 1))
    for index, left in enumerate(expected, start=1):
        current = [index]
        for offset, right in enumerate(actual, start=1):
            current.append(min(
                current[-1] + 1,
                previous[offset] + 1,
                previous[offset - 1] + (left != right),
            ))
        previous = current
    return previous[-1]


def evaluate(path: Path) -> dict:
    """Return corpus-wide weighted CER and a conservative acceptance verdict."""
    with Path(path).open("r", encoding="utf-8-sig", newline="") as stream:
        reader = csv.DictReader(stream)
        required = {"id", "language", "expected", "actual"}
        if not reader.fieldnames or not required.issubset(reader.fieldnames):
            raise ValueError("CSV requires id,language,expected,actual")
        groups = {
            "zh": {"samples": 0, "characters": 0, "errors": 0},
            "mixed": {"samples": 0, "characters": 0, "errors": 0},
        }
        sample_count = 0
        hits = 0
        seen = set()
        for row in reader:
            sample_id = row.get("id")
            language = row.get("language")
            expected = row.get("expected")
            actual = row.get("actual")
            if (not sample_id or sample_id in seen or language not in groups
                    or not expected or actual is None or None in row):
                raise ValueError(f"invalid sample at CSV row {reader.line_num}")
            seen.add(sample_id)
            sample_count += 1
            hits += bool(actual.strip())
            groups[language]["samples"] += 1
            groups[language]["characters"] += len(expected)
            groups[language]["errors"] += character_errors(expected, actual)
    if not sample_count:
        raise ValueError("OCR evaluation requires at least one sample")
    for metrics in groups.values():
        metrics["cer"] = (metrics["errors"] / metrics["characters"]
                          if metrics["characters"] else None)
    hit_rate = hits / sample_count
    reasons = []
    if sample_count < 400:
        reasons.append("requires >=400 samples")
    if groups["zh"]["cer"] is None:
        reasons.append("missing Chinese samples")
    elif groups["zh"]["cer"] > 0.02:
        reasons.append("Chinese CER >2%")
    if groups["mixed"]["cer"] is None:
        reasons.append("missing mixed-language samples")
    elif groups["mixed"]["cer"] > 0.05:
        reasons.append("mixed CER >5%")
    if hit_rate < 0.95:
        reasons.append("hit rate <95%")
    return {
        "overall": {"samples": sample_count, "hits": hits, "hit_rate": hit_rate},
        **groups,
        "acceptance": {"passed": not reasons, "reasons": reasons},
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv", type=Path, help="real labeled OCR samples")
    parser.add_argument("--output", type=Path, help="write JSON report here")
    args = parser.parse_args()
    try:
        summary = evaluate(args.csv)
    except (OSError, UnicodeError, csv.Error, ValueError) as error:
        parser.exit(2, f"OCR evaluation: {error}\n")
    rendered = json.dumps(summary, ensure_ascii=False, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    return 0 if summary["acceptance"]["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
