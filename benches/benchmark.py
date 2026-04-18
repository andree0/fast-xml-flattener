#!/usr/bin/env python3
"""Benchmark fast-xml-flattener vs xmltodict and lxml.

Measures wall-clock time for common XML-flattening tasks across
different document sizes (1 MB, 10 MB, 50 MB).

Run with:
    python benches/benchmark.py
"""

from __future__ import annotations

import gc
import json
import statistics
import time
from collections.abc import Callable

import lxml.etree as ET
import xmltodict

import fast_xml_flattener as fxf

# ─── XML generation ────────────────────────────────────────────────────────────

RECORD_TEMPLATE = """\
  <record>
    <id>{i}</id>
    <user>
      <name>User {i}</name>
      <email>user{i}@example.com</email>
      <age>{age}</age>
    </user>
    <address>
      <street>ul. Testowa {i}</street>
      <city>Warszawa</city>
      <zip>00-{i:03d}</zip>
      <country>PL</country>
    </address>
    <order>
      <id>ORD-{i:06d}</id>
      <status>confirmed</status>
      <amount>{amount:.2f}</amount>
      <currency>PLN</currency>
    </order>
  </record>"""


def make_xml(n_records: int) -> str:
    records = "\n".join(
        RECORD_TEMPLATE.format(i=i, age=20 + i % 60, amount=99.99 + i * 0.01)
        for i in range(1, n_records + 1)
    )
    return f"<root>\n{records}\n</root>"


# ─── xmltodict helpers ─────────────────────────────────────────────────────────


def _flatten_dict(d: dict, parent: str = "", sep: str = ".") -> dict:
    out = {}
    for k, v in d.items():
        key = f"{parent}{sep}{k}" if parent else k
        if isinstance(v, dict):
            out.update(_flatten_dict(v, key, sep))
        elif isinstance(v, list):
            for i, item in enumerate(v):
                indexed = f"{key}[{i}]"
                if isinstance(item, dict):
                    out.update(_flatten_dict(item, indexed, sep))
                else:
                    out[indexed] = item
        else:
            out[key] = v
    return out


def xmltodict_to_flat_dict(xml: str) -> dict:
    return _flatten_dict(xmltodict.parse(xml))


def lxml_to_flat_dict(xml: str) -> dict:
    root = ET.fromstring(xml.encode())
    out = {}

    def walk(el: ET.Element, prefix: str) -> None:
        children = list(el)
        if not children:
            out[prefix] = el.text or ""
            return
        counts: dict[str, int] = {}
        for ch in children:
            counts[ch.tag] = counts.get(ch.tag, 0) + 1
        seen: dict[str, int] = {}
        for ch in children:
            tag = ch.tag
            if counts[tag] > 1:
                idx = seen.get(tag, 0)
                key = f"{prefix}.{tag}[{idx}]"
                seen[tag] = idx + 1
            else:
                key = f"{prefix}.{tag}"
            walk(ch, key)

    walk(root, root.tag)
    return out


# ─── Benchmarking harness ──────────────────────────────────────────────────────

WARMUP = 2
REPEATS = 7


def bench(fn: Callable, *args, label: str) -> tuple[float, float]:
    for _ in range(WARMUP):
        fn(*args)
    gc.disable()
    times = []
    for _ in range(REPEATS):
        t0 = time.perf_counter()
        fn(*args)
        times.append(time.perf_counter() - t0)
    gc.enable()
    return statistics.median(times) * 1000, statistics.stdev(times) * 1000


# ─── Main ──────────────────────────────────────────────────────────────────────

SIZES = [
    ("1 MB", 1_200),
    ("10 MB", 12_000),
    ("50 MB", 60_000),
]

TASKS = [
    ("XML → flat dict", fxf.to_flatten_dict, xmltodict_to_flat_dict, lxml_to_flat_dict),
    ("XML → nested dict", fxf.to_dict, xmltodict.parse, None),
    (
        "XML → flat JSON",
        fxf.to_flatten_json,
        lambda x: json.dumps(_flatten_dict(xmltodict.parse(x))),
        None,
    ),
]

COL = 28


def fmt(ms: float, sd: float) -> str:
    return f"{ms:6.1f} ms ± {sd:.1f}"


def header(title: str) -> None:
    print(f"\n{'─' * 72}")
    print(f"  {title}")
    print(f"{'─' * 72}")
    print(f"  {'Library':<{COL}} {'median':>14}  {'± stdev':>10}")
    print(f"  {'-' * (COL + 28)}")


def row(label: str, ms: float, sd: float, baseline: float | None = None) -> None:
    speedup = f"  {baseline / ms:.1f}x faster" if baseline and ms < baseline else ""
    print(f"  {label:<{COL}} {fmt(ms, sd)}{speedup}")


def main() -> None:
    print("\nGenerating XML documents...", end=" ", flush=True)
    xmls = {label: make_xml(n) for label, n in SIZES}
    sizes_mb = {label: len(xml.encode()) / 1e6 for label, xml in xmls.items()}
    print("done")
    for label, mb in sizes_mb.items():
        print(f"  {label}: {mb:.1f} MB actual")

    results: dict[str, dict[str, dict[str, tuple[float, float]]]] = {}

    for task_name, fxf_fn, xmltodict_fn, lxml_fn in TASKS:
        results[task_name] = {}
        for size_label, xml in xmls.items():
            r = {}
            r["fast-xml-flattener"] = bench(fxf_fn, xml, label="fxf")
            r["xmltodict"] = bench(xmltodict_fn, xml, label="xmltodict")
            if lxml_fn:
                r["lxml"] = bench(lxml_fn, xml, label="lxml")
            results[task_name][size_label] = r

    # ── Print results ──────────────────────────────────────────────────────────
    for task_name, by_size in results.items():
        for size_label, r in by_size.items():
            header(f"{task_name}  [{size_label}]")
            baseline = r["xmltodict"][0]
            for lib, (ms, sd) in r.items():
                bl = baseline if lib != "xmltodict" and lib != "lxml" else None
                row(lib, ms, sd, bl)

    # ── Summary table for README ───────────────────────────────────────────────
    print(f"\n\n{'═' * 72}")
    print("  README TABLE  (XML → flat dict, median)")
    print(f"{'═' * 72}")
    print(f"  | {'Library':<36} | {'1 MB':>8} | {'10 MB':>8} | {'50 MB':>8} |")
    print(f"  |{'-' * 38}|{'-' * 10}|{'-' * 10}|{'-' * 10}|")
    task = "XML → flat dict"
    for lib in ["fast-xml-flattener", "xmltodict", "lxml"]:
        row_parts = []
        for size_label in ["1 MB", "10 MB", "50 MB"]:
            r = results[task].get(size_label, {})
            if lib in r:
                row_parts.append(f"{r[lib][0]:5.0f} ms")
            else:
                row_parts.append("   n/a   ")
        label_col = f"**{lib}**" if lib == "fast-xml-flattener" else f"`{lib}`"
        print(f"  | {label_col:<36} | {row_parts[0]:>8} | {row_parts[1]:>8} | {row_parts[2]:>8} |")
    print()


if __name__ == "__main__":
    main()
