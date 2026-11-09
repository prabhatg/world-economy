"""Forcing packs written from Python must be the format oikos-io reads (docs/03 §3.8a)."""

import json
import subprocess
from pathlib import Path

import blake3
import numpy as np
import pytest

from oikos_data import pack

ENGINE = Path(__file__).resolve().parents[2] / "engine"
OIKOS = ENGINE / "target" / "release" / "oikos"
H3 = np.array([0x8528_0003_FFFF_FFFF, 0x8528_0007_FFFF_FFFF, 0x8528_000B_FFFF_FFFF], dtype=np.uint64)


def sample():
    b = pack.Builder("terrain", pack.region("test", 5, H3))
    b.time([-12000, -11900, -11800], "linear")
    b.source(dataset="test-dem", version="1", license="CC-BY-4.0", citation="Test (2026)", manifest_sha256="00" * 32)
    b.field("elev", "m", "test elevation", np.array([20, 65535, 40], dtype=np.uint16), scale=0.5, offset=-10.0, nodata=65535)
    b.field("temp", "degC", "test temperature", np.array([[1, 2, 3], [11, 12, 13], [-21, -22, -23]], dtype=np.int16))
    b.field("frac", "1", "test fraction", np.array([0.25, 0.5, 1.0], dtype=np.float32))
    return b.build()


def test_id_is_blake3_and_header_is_canonical():
    data = sample()
    p = pack.read(data)
    assert p.id == blake3.blake3(data).hexdigest()
    n = int.from_bytes(data[8:12], "little")
    header = data[12 : 12 + n].decode()
    assert header == json.dumps(json.loads(header), sort_keys=True, separators=(",", ":"))
    assert all(f["offset"] % 8 == 0 for f in p.header["fields"])
    assert sample() == data


def test_values_round_trip_with_scale_offset_and_nodata():
    p = pack.read(sample())
    np.testing.assert_array_equal(p.values("elev"), [0.0, np.nan, 10.0])
    np.testing.assert_array_equal(p.values("temp")[1], [11.0, 12.0, 13.0])
    assert p.values("temp").shape == (3, 3)
    np.testing.assert_array_equal(p.values("frac"), [0.25, 0.5, 1.0])
    np.testing.assert_array_equal(p.raw("elev"), [20, 65535, 40])


def test_sources_carry_attribution_only_when_given():
    b = pack.Builder("terrain", pack.region("test", 5, H3))
    b.source(dataset="d", version="1", license="L", citation="C", manifest_sha256="00" * 32, attribution="produced using X")
    assert pack.read(b.build()).header["sources"][0]["attribution"] == "produced using X"
    assert "attribution" not in pack.read(sample()).header["sources"][0]


def test_builder_rejects_inconsistent_fields():
    b = pack.Builder("terrain", pack.region("test", 5, H3))
    with pytest.raises(ValueError, match="3 cells"):
        b.field("x", "m", "x", np.array([1, 2], dtype=np.uint8))
    with pytest.raises(ValueError, match="time"):
        b.field("y", "m", "y", np.zeros((2, 3), dtype=np.uint8))
    with pytest.raises(ValueError, match="dtype"):
        b.field("z", "m", "z", np.array([1, 2, 3], dtype=np.int8))
    b.field("w", "m", "w", np.array([1, 2, 3], dtype=np.uint8))
    with pytest.raises(ValueError, match="duplicate"):
        b.field("w", "m", "w", np.array([1, 2, 3], dtype=np.uint8))
    with pytest.raises(ValueError, match="ascending"):
        pack.region("bad", 5, H3[::-1])


@pytest.mark.skipif(not OIKOS.exists(), reason="build the CLI first: cargo build --release -p oikos-cli")
def test_the_engine_reads_python_packs(tmp_path):
    f = tmp_path / "sample.oikospack"
    f.write_bytes(sample())
    summary = json.loads(subprocess.run([str(OIKOS), "pack", str(f)], check=True, capture_output=True, text=True).stdout)
    p = pack.read(f.read_bytes())
    assert summary["id"] == p.id
    assert summary["region"]["cells_hash"] == pack.cells_hash(H3)
    by_name = {s["name"]: s for s in summary["fields"]}
    assert (by_name["elev"]["min"], by_name["elev"]["max"], by_name["elev"]["nodata"]) == (0.0, 10.0, 1)
    assert (by_name["temp"]["min"], by_name["temp"]["max"]) == (-23.0, 13.0)


@pytest.mark.skipif(not OIKOS.exists(), reason="build the CLI first: cargo build --release -p oikos-cli")
def test_the_engine_checks_geometry_cells_against_the_region(tmp_path):
    good = pack.Builder("geometry", pack.region("test", 5, H3))
    good.field("h3", "h3 index", "cells", H3)
    f = tmp_path / "geo.oikospack"
    f.write_bytes(good.build())
    subprocess.run([str(OIKOS), "pack", str(f)], check=True, capture_output=True)

    bad = pack.Builder("geometry", pack.region("test", 5, H3))
    bad.field("h3", "h3 index", "cells", H3 + np.uint64(1 << 40))
    f.write_bytes(bad.build())
    r = subprocess.run([str(OIKOS), "pack", str(f)], capture_output=True, text=True)
    assert r.returncode != 0 and "cells_hash" in r.stderr


def series():
    b = pack.Builder("climate_anomaly", pack.region("test", 5, H3))
    b.time([0, 10, 30], "linear")
    b.field("index", "1", "test index", np.array([2, -4, 0], dtype=np.int16), scale=0.5, nodata=0, series=True)
    b.field("weight", "1", "test weight", np.array([1, 2, 3], dtype=np.uint8))
    return b.build()


def test_time_series_hold_one_value_per_slice():
    p = pack.read(series())
    assert p.header["fields"][0]["dims"] == ["time"]
    assert np.array_equal(p.values("index"), [1.0, -2.0, np.nan], equal_nan=True)
    assert p.values("weight").shape == (3,)
    b = pack.Builder("climate_anomaly", pack.region("test", 5, H3))
    with pytest.raises(ValueError, match="time"):
        b.field("i", "1", "i", np.array([1, 2, 3], dtype=np.int16), series=True)
    b.time([0, 10], "linear")
    with pytest.raises(ValueError, match="2 slices"):
        b.field("i", "1", "i", np.array([1, 2, 3], dtype=np.int16), series=True)


@pytest.mark.skipif(not OIKOS.exists(), reason="build the CLI first: cargo build --release -p oikos-cli")
def test_the_engine_reads_python_time_series(tmp_path):
    f = tmp_path / "series.oikospack"
    f.write_bytes(series())
    summary = json.loads(subprocess.run([str(OIKOS), "pack", str(f)], check=True, capture_output=True, text=True).stdout)
    index = summary["fields"][0]
    assert (index["dims"], index["min"], index["max"], index["nodata"]) == (["time"], -2.0, 1.0, 1)
