"""Shared JUnit-XML parsing helpers for the nextest-* scripts."""

import sys
import xml.etree.ElementTree as ET
from pathlib import Path


def derive_status(case: ET.Element) -> str:
    if case.find("failure") is not None or case.find("error") is not None:
        return "failed"
    if case.find("flakyFailure") is not None:
        return "flaky"
    if case.find("skipped") is not None:
        return "skipped"
    return "passed"


def parse_junit_dir(junit_dir: Path) -> tuple[list[str], list[str], int]:
    # nextest's partitioning guarantees each test id appears once across all
    # shards, so set-keyed dedup is correct.
    failed: set[str] = set()
    flaky: set[str] = set()
    total = 0
    xml_files = sorted(junit_dir.rglob("*.xml")) if junit_dir.exists() else []
    for path in xml_files:
        try:
            tree = ET.parse(path)
        except ET.ParseError as e:
            print(f"warning: failed to parse {path}: {e}", file=sys.stderr)
            continue
        for case in tree.iter("testcase"):
            total += 1
            classname = case.attrib.get("classname") or ""
            name = case.attrib.get("name") or ""
            test_id = f"{classname}::{name}"
            status = derive_status(case)
            if status == "failed":
                failed.add(test_id)
            elif status == "flaky":
                flaky.add(test_id)
    return sorted(failed), sorted(flaky), total
