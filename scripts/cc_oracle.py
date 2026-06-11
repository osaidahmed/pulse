#!/usr/bin/env python3
"""Differential cc oracle: compare pulse per-function cc against lizard CCN over the fixture corpus.

  ./scripts/remote.sh 'cargo build --release && python3 scripts/cc_oracle.py \
      --pulse-bin target/release/pulse --lizard ~/.venvs/lizard/bin/lizard'

Known convention deltas behind the per-language default tolerances (calibrated June 2026,
675 functions, mean |d| <= 0.30 in every language; pulse verified correct on each class):
  rust=7 / php=5    lizard does not count match arms; pulse counts one per non-default arm
  csharp=3          lizard does not count foreach as a loop decision
  python=2 / lua=2  pulse deliberately excludes comprehension clauses and value-expression
                    short-circuits from cc (idiom-neutrality); lizard counts them everywhere;
                    pulse also attributes nested-closure branching to the enclosing function
  ruby=2            modifier-form conditionals and case/when counting differ at the margin
"""

import argparse
import csv
import io
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

LIZARD_LANGS = {
    ".c": "c",
    ".cpp": "cpp",
    ".java": "java",
    ".cs": "csharp",
    ".js": "javascript",
    ".ts": "typescript",
    ".m": "objectivec",
    ".swift": "swift",
    ".py": "python",
    ".rb": "ruby",
    ".php": "php",
    ".go": "go",
    ".lua": "lua",
    ".rs": "rust",
    ".kt": "kotlin",
}

DEFAULT_LANG_TOLERANCE = {
    "csharp": 3,
    "lua": 2,
    "php": 5,
    "python": 2,
    "ruby": 2,
    "rust": 7,
}

PULSE_FN = re.compile(r"^\s+(.+?) \(L(\d+)-\d+\): loc=\d+ cc=(\d+)")


def normalize(name):
    bare = re.split(r"::|\.|:", name)[-1] or name
    return re.sub(r"[^a-z0-9]", "", bare.lower())


def pulse_functions(pulse_bin, path):
    out = subprocess.run(
        [pulse_bin, "debug", str(path)], capture_output=True, text=True, timeout=30
    )
    functions = []
    for line in out.stdout.splitlines() + out.stderr.splitlines():
        m = PULSE_FN.match(line)
        if m:
            functions.append((normalize(m.group(1)), int(m.group(2)), int(m.group(3))))
    return functions


def lizard_functions(lizard_bin, path):
    out = subprocess.run(
        [lizard_bin, "--csv", str(path)], capture_output=True, text=True, timeout=60
    )
    functions = []
    for row in csv.reader(io.StringIO(out.stdout)):
        if len(row) < 11:
            continue
        try:
            ccn, start = int(row[1]), int(row[9])
        except ValueError:
            continue
        functions.append((normalize(row[7]), start, ccn))
    return functions


def pair_up(pulse_fns, lizard_fns):
    pairs, claimed = [], set()
    for name, line, cc in pulse_fns:
        candidates = [
            (abs(line - lstart), i, lccn)
            for i, (lname, lstart, lccn) in enumerate(lizard_fns)
            if i not in claimed and lname == name
        ]
        if not candidates:
            continue
        _, idx, lccn = min(candidates)
        claimed.add(idx)
        pairs.append((name, line, cc, lccn))
    unmatched = len(pulse_fns) - len(pairs)
    return pairs, unmatched


def check_pairs(label, pairs, tol, verbose):
    deltas, failures = [], []
    for name, line, cc, lccn in pairs:
        delta = abs(cc - lccn)
        deltas.append(delta)
        if delta > tol:
            failures.append(
                f"{label}:{line} {name}: pulse cc={cc} lizard ccn={lccn} (|d|={delta} > {tol})"
            )
        elif verbose and delta:
            print(f"  within-band {label}:{line} {name}: pulse {cc} vs lizard {lccn}")
    return deltas, failures


def compare_language(lang, files, tol, args):
    all_deltas, unmatched_total, failures = [], 0, []
    for path in files:
        pairs, unmatched = pair_up(
            pulse_functions(args.pulse_bin, path), lizard_functions(args.lizard, path)
        )
        unmatched_total += unmatched
        deltas, fails = check_pairs(f"{lang} {path}", pairs, tol, args.verbose)
        all_deltas.extend(deltas)
        failures.extend(fails)
    mean = sum(all_deltas) / len(all_deltas) if all_deltas else 0.0
    print(
        f"{lang}: {len(files)} files, {len(all_deltas)} functions compared, "
        f"mean |d|={mean:.2f}, max |d|={max(all_deltas, default=0)}, tolerance {tol}, unmatched {unmatched_total}"
    )
    return len(all_deltas), failures


def collect_files(root):
    by_lang = defaultdict(list)
    for path in sorted(root.rglob("*")):
        lang = LIZARD_LANGS.get(path.suffix)
        if lang and path.is_file():
            by_lang[lang].append(path)
    return by_lang


def print_summary(root, by_lang, compared_total, failures):
    skipped = sorted(
        {
            p.suffix
            for p in root.rglob("*")
            if p.is_file() and p.suffix and p.suffix not in LIZARD_LANGS
        }
    )
    print(
        f"\ncompared {compared_total} functions across {len(by_lang)} languages; no oracle for: {' '.join(skipped) or 'none'}"
    )
    if failures:
        print(f"\n{len(failures)} out-of-band divergences:")
        print("\n".join(failures))


def parse_args():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--root", default="tests/fixtures")
    ap.add_argument("--pulse-bin", default="pulse")
    ap.add_argument("--lizard", default="lizard")
    ap.add_argument("--tolerance", type=int, default=1)
    ap.add_argument("--lang-tolerance", action="append", default=[], metavar="LANG=N")
    ap.add_argument("-v", "--verbose", action="store_true")
    args = ap.parse_args()
    args.lang_tolerance = dict(
        (kv.split("=")[0], int(kv.split("=")[1])) for kv in args.lang_tolerance
    )
    return args


def main():
    args = parse_args()
    root = Path(args.root)
    by_lang = collect_files(root)
    failures, compared_total = [], 0
    for lang, files in sorted(by_lang.items()):
        tol = args.lang_tolerance.get(
            lang, DEFAULT_LANG_TOLERANCE.get(lang, args.tolerance)
        )
        count, fails = compare_language(lang, files, tol, args)
        compared_total += count
        failures.extend(fails)
    print_summary(root, by_lang, compared_total, failures)
    return 1 if failures else 0


sys.exit(main())
