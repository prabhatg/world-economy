"""Climate pack: per-cell bioclimatic fields per century slice from strip-stored rasters."""

import json
import subprocess
from pathlib import Path

import h3
import numpy as np
import pytest

from oikos_data import climate, pack, raster, region

OIKOS = Path(__file__).resolve().parents[2] / "engine" / "target" / "release" / "oikos"
BOX = region.Region(name="test-box", resolution=5, bbox=(35.2, 35.8, 40.2, 40.8), sha256="0" * 64)
MANIFEST = {"dataset": "test-climate", "version": "1", "citation": "Test",
            "license": {"id": "T", "attribution": "contains test climate"}, "access": {"time_ids": [-101, -99]}}
SHA = "1" * 64
N = 60  # samples per degree


def grid():
    """Sample centers of a 1/60-degree window covering the box and its margin."""
    lat = 36.2 - (np.arange(int(1.4 * N)) + 0.5) / N
    lng = 39.8 + (np.arange(int(1.4 * N)) + 0.5) / N
    return lat, lng


def tile(values):
    return raster.Tile(np.asarray(values, dtype=np.float32), north=36.2, west=39.8, dlat=1 / N, dlng=1 / N)


def field(fn):
    lat, lng = grid()
    return tile(fn(lat[:, None] + 0 * lng[None, :], lng[None, :] + 0 * lat[:, None]))


def tiles(**overrides):
    """Constant fields per variable and slice, unless overridden by `<var>=fn(lat, lng, time_id)`."""
    base = {"bio01": 15.0, "bio04": 650.0, "bio12": 400.0, "bio15": 90.0}
    out = {}
    for var, v in base.items():
        for t in (-101, -100, -99):
            fn = overrides.get(var)
            out[(var, t)] = field(lambda la, lo: fn(la, lo, t)) if fn else field(lambda la, lo: v + t + 101 + 0 * la)
    return out


def build(t):
    return pack.read(climate.build(BOX, MANIFEST, SHA, t))


def centers():
    return np.array([h3.cell_to_latlng(h3.int_to_str(int(c))) for c in BOX.cells()])


def test_fields_per_slice_at_midpoint_years():
    p = build(tiles())
    assert p.header["kind"] == "climate"
    assert p.header["time"] == {"calendar_year": [-10150, -10050, -9950], "rule": "linear"}
    assert {f["name"]: f["dims"] for f in p.header["fields"]} == {
        "temp_mean": ["time", "cell"], "temp_seasonality": ["time", "cell"], "precip": ["time", "cell"], "precip_cv": ["time", "cell"]}
    n = len(BOX.cells())
    for name, want in (("temp_mean", 15.0), ("temp_seasonality", 6.5), ("precip", 400.0), ("precip_cv", 90.0)):
        v = p.values(name)
        assert v.shape == (3, n)
        np.testing.assert_allclose(v[0], want)
    np.testing.assert_allclose(p.values("temp_mean")[2], 17.0)
    np.testing.assert_allclose(p.values("temp_seasonality")[1], 6.51)  # source is SD x 100
    assert p.header["sources"][0]["attribution"] == "contains test climate"


def test_cell_means_follow_a_gradient():
    p = build(tiles(bio01=lambda la, lo, t: 10 + 2 * (la - 35)))
    np.testing.assert_allclose(p.values("temp_mean")[0], 10 + 2 * (centers()[:, 0] - 35), atol=0.02)


def test_nan_samples_are_skipped_and_rainless_cells_have_no_cv():
    lat_c, lng_c = centers()[0]
    first = h3.latlng_to_cell(lat_c, lng_c, 5)
    lat, lng = grid()
    inside = np.array([[h3.latlng_to_cell(a, b, 5) == first for b in lng] for a in lat])

    def cv(la, lo, t):
        v = np.full(la.shape, 90.0)
        v[inside] = np.nan
        return v

    def temp(la, lo, t):
        v = np.full(la.shape, 15.0)
        v[inside & (la > lat_c)] = np.nan
        return v

    p = build(tiles(bio15=cv, bio01=temp))
    assert np.isnan(p.values("precip_cv")[0, 0]) and p.raw("precip_cv")[0, 0] == climate.U16_NODATA
    assert np.all(p.values("precip_cv")[:, 1:] == 90.0)
    np.testing.assert_allclose(p.values("temp_mean"), 15.0)


def test_rejects_out_of_range_values_mixed_grids_and_uncovered_cells():
    with pytest.raises(ValueError, match="range"):
        build(tiles(bio12=lambda la, lo, t: 70000 + 0 * la))
    t = tiles()
    t[("bio01", -100)] = raster.Tile(t[("bio01", -100)].data, north=36.3, west=39.8, dlat=1 / N, dlng=1 / N)
    with pytest.raises(ValueError, match="one grid"):
        build(t)
    small = {k: raster.Tile(v.data[:20, :20], v.north, v.west, v.dlat, v.dlng) for k, v in tiles().items()}
    with pytest.raises(ValueError, match="no climate samples"):
        build(small)


def test_is_reproducible_and_matches_the_region():
    a = climate.build(BOX, MANIFEST, SHA, tiles())
    assert a == climate.build(BOX, MANIFEST, SHA, tiles())
    assert pack.read(a).header["region"]["cells_hash"] == pack.cells_hash(BOX.cells())


def test_time_ids_and_midpoints():
    assert climate.time_ids({"access": {"time_ids": [-120, -29]}})[::91] == [-120, -29]
    assert len(climate.time_ids({"access": {"time_ids": [-120, -29]}})) == 92
    assert climate.midpoint_year(20) == 1950 and climate.midpoint_year(-100) == -10050


@pytest.mark.skipif(not OIKOS.exists(), reason="build the CLI first: cargo build --release -p oikos-cli")
def test_engine_reads_the_climate_pack(tmp_path):
    f = tmp_path / "c.oikospack"
    f.write_bytes(climate.build(BOX, MANIFEST, SHA, tiles()))
    s = json.loads(subprocess.run([str(OIKOS), "pack", str(f)], check=True, capture_output=True, text=True).stdout)
    assert s["time"]["rule"] == "linear" and len(s["time"]["calendar_year"]) == 3
    temp = next(x for x in s["fields"] if x["name"] == "temp_mean")
    assert temp["min"] == pytest.approx(15.0) and temp["max"] == pytest.approx(17.0) and temp["nodata"] == 0


def test_range_ends_except_nodata_are_valid():
    p = build(tiles(bio12=lambda la, lo, t: 0 * la, bio01=lambda la, lo, t: -327.67 + 0 * la))
    assert np.all(p.values("precip") == 0) and np.allclose(p.values("temp_mean"), -327.67)
    with pytest.raises(ValueError, match="range"):
        build(tiles(bio12=lambda la, lo, t: 65535 + 0 * la))
    with pytest.raises(ValueError, match="range"):
        build(tiles(bio01=lambda la, lo, t: -327.68 + 0 * la))
