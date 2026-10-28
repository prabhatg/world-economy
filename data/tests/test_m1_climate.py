"""The M1 climate pack against known geography and climate history (runs when the
CHELSA-TraCE21k windows are cached: `python -m oikos_data build climate m1-swasia`
reads them once, ~1 h)."""

import json
import subprocess
from pathlib import Path

import h3
import numpy as np
import pytest

from oikos_data import climate, geometry, pack, raster, region

OIKOS = Path(__file__).resolve().parents[2] / "engine" / "target" / "release" / "oikos"


@pytest.fixture(scope="module")
def data():
    try:
        return climate.build_pack(region.load("m1-swasia"), offline=True)
    except raster.MissingInputs:
        pytest.skip("CHELSA-TraCE21k windows not cached; run: python -m oikos_data build climate m1-swasia")


@pytest.fixture(scope="module")
def m1(data):
    cells = region.load("m1-swasia").cells()
    p = pack.read(data)
    years = np.array(p.header["time"]["calendar_year"])

    def at(lat, lng, field, year):
        c = np.searchsorted(cells, h3.str_to_int(h3.latlng_to_cell(lat, lng, 5)))
        return p.values(field)[np.searchsorted(years, year), c]

    def mean(field, a, b):
        """Region mean over the slices with midpoints in [a, b]."""
        sel = (years >= a) & (years <= b)
        return float(np.nanmean(p.values(field)[sel]))

    return p, at, mean


def test_shape_and_time(m1):
    p, _, _ = m1
    t = p.header["time"]
    assert t["rule"] == "linear" and t["calendar_year"][0] == -12050 and t["calendar_year"][-1] == -2950
    assert len(t["calendar_year"]) == 92 and np.all(np.diff(t["calendar_year"]) == 100)
    assert p.header["sources"][0]["license"] == "CC-BY-SA-4.0"


def test_known_places(m1):
    _, at, _ = m1
    for y in (-12050, -9050, -6050, -3050):
        assert at(30.96, 46.1, "precip", y) < 250, "southern Mesopotamia is too dry for rain-fed farming"
        assert at(34.31, 47.07, "precip", y) > 2 * at(30.96, 46.1, "precip", y), "Zagros foothills (Kermanshah) wetter"
        assert at(24.09, 32.9, "precip", y) < 5 and at(30.04, 31.24, "precip", y) < 60, "Egypt is desert"
        assert at(35.95, 52.11, "temp_mean", y) < at(27.0, 51.0, "temp_mean", y) - 20, "Damavand colder than the Gulf"
    assert 9 < at(37.67, 32.83, "temp_mean", -7050) < 15, "Konya plain at Catalhoyuk"
    assert np.isnan(at(24.09, 32.9, "precip_cv", -12050)), "rainless Aswan has no precipitation CV"


def test_younger_dryas(m1):
    """Bolling-Allerod warmth, Younger Dryas cooling (12.9-11.7 ka BP, ~10,950-9,750 BCE),
    abrupt Holocene warming: TraCE-21ka's meltwater-forced YD reaches the region as ~1-1.5 °C."""
    _, _, mean = m1
    allerod, yd, early_holocene = mean("temp_mean", -11450, -11050), mean("temp_mean", -10450, -10050), mean("temp_mean", -9450, -9050)
    assert allerod - yd > 0.8 and early_holocene - yd > 0.8
    assert mean("temp_mean", -12050, -11950) < allerod, "warming out of the Oldest Dryas"


def test_holocene_trends(m1):
    _, _, mean = m1
    assert mean("temp_mean", -3450, -2950) - mean("temp_mean", -9450, -8950) > 0.5, "Holocene warming in TraCE-21ka"
    assert mean("temp_seasonality", -9450, -8950) > mean("temp_seasonality", -3450, -2950), \
        "temperature seasonality declines as boreal summer insolation falls from its early-Holocene peak"


def test_8_2_ka_event_is_not_resolved(m1):
    """The 8.2 ka cold event (~6,200 BCE, ~160 years) does not show in this data (the
    region cools by ~0.1 °C); 100-year means would smooth it even if the model had it. A
    mechanism that needs it must add it from proxies; this test fails if the data changes."""
    _, _, mean = m1
    before, during = mean("temp_mean", -6650, -6450), mean("temp_mean", -6250, -6150)
    assert abs(before - during) < 0.5


@pytest.mark.skipif(not OIKOS.exists(), reason="build the CLI first: cargo build --release -p oikos-cli")
def test_engine_runs_an_m1_world_with_the_climate_pack(data, tmp_path):
    g = geometry.build(region.load("m1-swasia"))
    inputs = []
    for kind, b in (("geometry", g), ("climate", data)):
        (tmp_path / f"{kind}.oikospack").write_bytes(b)
        inputs.append({"id": pack.read(b).id, "path": f"{kind}.oikospack"})
    wu = {"format": 1, "id": "m1-climate-smoke", "world": {"geometry": pack.read(g).id}, "seed": 1,
          "start": {"kind": "initial"}, "years": 20, "partitions": 2, "summary_every_years": 10, "inputs": inputs}
    (tmp_path / "wu.json").write_text(json.dumps(wu))
    out = json.loads(subprocess.run([str(OIKOS), "run", "wu.json"], cwd=tmp_path, check=True, capture_output=True, text=True).stdout)
    assert out["summaries"][-1]["population"] > 0
