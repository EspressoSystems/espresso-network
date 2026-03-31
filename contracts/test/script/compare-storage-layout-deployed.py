#!/usr/bin/env python3
import argparse
import json
import os
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


def extract_deployed_layout(address: str) -> list | None:
    rpc_url = os.environ["RPC_URL"]
    result = subprocess.run(
        ["cast", "storage", address, "--rpc-url", rpc_url, "--json"],
        capture_output=True,
        text=True,
    )
    if "Storage layout is empty" in result.stderr or "Storage layout is empty" in result.stdout:
        return None  # signal: skip check
    if result.returncode != 0:
        print(result.stderr, file=sys.stderr)
        sys.exit(1)
    raw = result.stdout.strip()
    if not raw or raw == "null":
        return None
    data = json.loads(raw)
    storage = data.get("storage") or []
    if not storage:
        return None
    return [
        {
            "label": e["label"],
            "slot": to_int(e["slot"]),
            "offset": to_int(e["offset"]),
            "type": normalize_type(e["type"]),
        }
        for e in storage
    ]


def extract_local_layout(contract: str) -> list:
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


def types_compatible(t_a: str, t_b: str) -> bool:
    if t_a == t_b:
        return True
    # Both contract/interface types store as address — compatible
    return t_a.startswith("t_contract(") and t_b.startswith("t_contract(")


def compare(a: list, b: list) -> bool:
    # New contract may add storage slots at the end — that's safe for upgrades.
    if len(a) > len(b):
        return False
    return all(
        a[i]["label"] == b[i]["label"]
        and a[i]["slot"] == b[i]["slot"]
        and a[i]["offset"] == b[i]["offset"]
        and types_compatible(a[i]["type"], b[i]["type"])
        for i in range(len(a))
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Compare deployed contract storage layout against local source. RPC_URL must be set in environment."
    )
    parser.add_argument("deployed_address", help="Deployed contract address (0x...)")
    parser.add_argument("local_contract", help="Local contract name (forge inspect format)")
    args = parser.parse_args()

    deployed = extract_deployed_layout(args.deployed_address)
    if deployed is None:
        print("true", end="")
        sys.exit(0)
    local = extract_local_layout(args.local_contract)
    print("true" if compare(deployed, local) else "false", end="")
