import json

import latency
import pytest
from fakes import load_script

ml = load_script("mainnet-locations")


def entry(addr: str | None, amount: str = "0x10") -> dict:
    info = None if addr is None else {"x25519_key": "k", "p2p_addr": addr}
    return {
        "stake_table_entry": {"stake_key": "bls", "stake_amount": amount},
        "state_ver_key": "s",
        "connect_info": info,
    }


def doc(*entries: dict) -> dict:
    return {"epoch": 7, "stake_table": list(entries)}


def geo(ip: str, city: str, cc: str, lat: float, lon: float) -> dict:
    return {
        "status": "success",
        "query": ip,
        "city": city,
        "country": "X",
        "countryCode": cc,
        "lat": lat,
        "lon": lon,
    }


def test_mainnet_parse_ok():
    parsed = ml.parse_stake_table(
        doc(entry("1.2.3.4:9977", "0x10"), entry("node.example.org:9977", "0xff"))
    )
    assert parsed == [("1.2.3.4", 16), ("node.example.org", 255)]


def test_mainnet_null_connect_fails():
    with pytest.raises(ValueError, match="2 of 3"):
        ml.parse_stake_table(doc(entry("1.2.3.4:1"), entry(None), entry(None)))


def test_mainnet_missing_key_fails():
    with pytest.raises(KeyError):
        ml.parse_stake_table({"epoch": 1})


@pytest.mark.parametrize("addr", ["host", "2001:db8::1:9977"])
def test_mainnet_addr_fails(addr):
    with pytest.raises(ValueError):
        ml.host_of(addr)


def test_mainnet_geo_batches_ok():
    ips = [f"10.0.0.{i}" for i in range(150)]
    bodies: list[list[str]] = []

    def post(url: str, body: bytes) -> bytes:
        assert url == ml.GEO_URL
        sent = json.loads(body)
        bodies.append(sent)
        return json.dumps([geo(ip, "C", "FI", 1, 2) for ip in sent]).encode()

    result = ml.geolocate(ips, post)
    assert [len(b) for b in bodies] == [100, 50]
    assert [r["query"] for r in result] == ips


def test_mainnet_geo_status_fails():
    def post(url: str, body: bytes) -> bytes:
        return json.dumps(
            [{"status": "fail", "query": "10.0.0.1", "message": "reserved range"}]
        ).encode()

    with pytest.raises(ValueError, match="10.0.0.1.*reserved range"):
        ml.geolocate(["10.0.0.1"], post)


def test_mainnet_aggregate_ok():
    rows = ml.aggregate(
        [
            (geo("a", "Helsinki", "FI", 60.1699, 24.9384), 10),
            (geo("b", "Tokyo", "JP", 35.6895, 139.6917), 20),
            (geo("c", "Helsinki", "FI", 60.5, 25.0), 10),
        ]
    )
    assert [r["label"] for r in rows] == ["helsinki-fi", "tokyo-jp"]
    assert rows[0] == {
        "label": "helsinki-fi",
        "city": "Helsinki",
        "country": "FI",
        "lat": 60.17,
        "lon": 24.94,
        "nodes": 2,
        "stake_share": 0.5,
    }
    assert sum(r["stake_share"] for r in rows) == pytest.approx(1, abs=1e-6)


def test_mainnet_slug_folds_accents():
    assert ml.slug("Zürich") == "zurich"
    assert ml.slug("São  Paulo!") == "sao-paulo"


def test_mainnet_main_writes_json(tmp_path, capsys):
    table = doc(
        entry("1.2.3.4:9977"), entry("node.example.org:9977"), entry("5.6.7.8:1")
    )
    ips = {"1.2.3.4": "1.2.3.4", "node.example.org": "9.9.9.9", "5.6.7.8": "5.6.7.8"}
    cities = {
        "1.2.3.4": ("Helsinki", "FI"),
        "9.9.9.9": ("Helsinki", "FI"),
        "5.6.7.8": ("Tokyo", "JP"),
    }

    def post(url: str, body: bytes) -> bytes:
        return json.dumps(
            [geo(ip, *cities[ip], 1.0, 2.0) for ip in json.loads(body)]
        ).encode()

    out = tmp_path / "out.json"
    code = ml.main(
        ["--out", str(out)],
        lambda url: json.dumps(table).encode(),
        post,
        ips.__getitem__,
        "2026-10-05",
    )
    text = out.read_text()
    assert code == 0 and text.endswith("\n")
    for secret in [*ips, "9.9.9.9"]:
        assert secret not in text
    data = json.loads(text)
    assert data["measured_at"] == "2026-10-05"
    assert data["source"] == ml.SOURCE
    assert (data["epoch"], data["validators"]) == (7, 3)
    assert [loc["nodes"] for loc in data["locations"]] == [2, 1]
    assert "3 validators, 2 locations" in capsys.readouterr().err


def test_mainnet_main_writes_nothing_on_failure(tmp_path):
    out = tmp_path / "out.json"
    with pytest.raises(ValueError):
        ml.main(
            ["--out", str(out)],
            lambda url: json.dumps(doc(entry(None))).encode(),
            lambda url, body: b"[]",
            str,
            "2026-10-05",
        )
    assert not out.exists()


def test_shipped_mainnet_shares_sum_to_one():
    data = json.loads(latency.MAINNET_JSON.read_text())
    total = sum(place["stake_share"] for place in data["locations"])
    assert total == pytest.approx(1, abs=1e-6)
    assert latency.load_profile("mainnet").name == "mainnet"
