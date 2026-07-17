#!/usr/bin/env python3
import argparse
import json
import subprocess
import sys


def normalize_type(t: str) -> str:
    idx = t.find(")")
    return t[: idx + 1] if idx != -1 else t


def to_int(v) -> int:
    if isinstance(v, int):
        return v
    s = str(v)
    return int(s, 16) if s.startswith(("0x", "0X")) else int(s)


def extract_layout(contract: str) -> list:
    try:
        result = subprocess.run(
            ["forge", "inspect", contract, "storageLayout", "--json"],
            capture_output=True,
            text=True,
            check=True,
        )
    except subprocess.CalledProcessError as e:
        print(e.stderr, file=sys.stderr)
        sys.exit(1)
    data = json.loads(result.stdout)
    return [
        {
            "label": e["label"],
            "slot": to_int(e["slot"]),
            "offset": to_int(e["offset"]),
            "type": normalize_type(e["type"]),
        }
        for e in (data.get("storage") or [])
    ]


def compare(a: list, b: list) -> bool:
    # New contract may add storage slots at the end — that's safe for upgrades.
    if len(a) > len(b):
        return False
    return all(
        a[i]["label"] == b[i]["label"]
        and a[i]["slot"] == b[i]["slot"]
        and a[i]["offset"] == b[i]["offset"]
        and a[i]["type"] == b[i]["type"]
        for i in range(len(a))
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Compare storage layouts of two contracts for upgrade safety."
    )
    parser.add_argument("old_contract", help="Old contract name (forge inspect format)")
    parser.add_argument("new_contract", help="New contract name (forge inspect format)")
    args = parser.parse_args()

    a = extract_layout(args.old_contract)
    b = extract_layout(args.new_contract)
    print("true" if compare(a, b) else "false", end="")
