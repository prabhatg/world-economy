"""Geometry packs: H3 index, centroid and area per cell, quantized so rebuilds match."""

import json
import subprocess
from pathlib import Path

import numpy as np
import pytest

from oikos_data import geometry, pack, region

ENGINE = Path(__file__).resolve().parents[2] / "engine"
OIKOS = ENGINE / "target" / "release" / "oikos"
SMALL = region.Region(name="test-box", resolution=5, bbox=(31.0, 32.0, 35.0, 36.0), sha256="0" * 64)


def test_geometry_pack_fields():
    p = pack.read(geometry.build(SMALL))
    cells = SMALL.cells()
    assert p.header["kind"] == "geometry" and p.header["region"]["cells"] == len(cells) > 20
    np.testing.assert_array_equal(p.raw("h3"), cells)
    lat, lng, area = p.values("lat"), p.values("lng"), p.values("area_km2")
    assert np.all((lat >= 31) & (lat <= 32) & (lng >= 35) & (lng <= 36))
    assert np.all((area > 140) & (area < 340)), (area.min(), area.max())  # H3 res-5 cell areas
    assert {f["name"] for f in p.header["fields"]} == {"h3", "lat", "lng", "area_km2"}
    assert p.header["sources"][0]["dataset"] == "h3"


def test_geometry_pack_is_reproducible():
    assert geometry.build(SMALL) == geometry.build(SMALL)


@pytest.mark.skipif(not OIKOS.exists(), reason="build the CLI first: cargo build --release -p oikos-cli")
def test_engine_accepts_the_geometry_pack(tmp_path):
    f = tmp_path / "g.oikospack"
    f.write_bytes(geometry.build(SMALL))
    s = json.loads(subprocess.run([str(OIKOS), "pack", str(f)], check=True, capture_output=True, text=True).stdout)
    assert s["kind"] == "geometry" and s["region"]["cells"] == len(SMALL.cells())
