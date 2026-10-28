"""The M1 hydrology pack against known rivers and lakes (runs when the RiverATLAS and
LakeATLAS features are cached: `python -m oikos_data build hydrology m1-swasia`
downloads them once, ~4 GB)."""

from pathlib import Path

import h3
import numpy as np
import pytest

from oikos_data import coastline, geometry, hydrology, pack, raster, region


@pytest.fixture(scope="module")
def data():
    try:
        return hydrology.build_pack(region.load("m1-swasia"), offline=True)
    except raster.MissingInputs:
        pytest.skip("RiverATLAS/LakeATLAS features not cached; run: python -m oikos_data build hydrology m1-swasia")


@pytest.fixture(scope="module")
def at(data):
    cells = region.load("m1-swasia").cells()
    p = pack.read(data)

    def at(lat, lng, field):
        return p.values(field)[np.searchsorted(cells, h3.str_to_int(h3.latlng_to_cell(lat, lng, 5)))]

    return at


def test_great_rivers_have_their_natural_discharge(at):
    # Long-term natural mean discharge (WaterGAP), m3/s.
    for name, (lat, lng), lo, hi in [("Nile at Luxor", (25.7, 32.65), 2000, 3200), ("Euphrates at Mari", (34.55, 40.89), 500, 1100),
                                     ("Tigris at Baghdad", (33.31, 44.36), 800, 1600), ("Karun at Ahvaz", (31.32, 48.67), 400, 1000),
                                     ("Orontes at Hama", (35.13, 36.75), 30, 150), ("Jordan", (32.1, 35.55), 15, 100)]:
        assert lo < at(lat, lng, "river_discharge") < hi, name
        assert at(lat, lng, "river_length") > 10, name  # a res-5 cell is ~17 km across
    assert at(31.0, 47.0, "floodplain_fraction") > 0.3, "Mesopotamian marshes"
    assert at(28.0, 41.0, "river_length") == 0, "Nefud desert: only wadis"


def test_lakes(at):
    assert at(39.5, 51.5, "lake_fraction") == 1, "Caspian Sea"
    assert at(38.6, 42.9, "lake_fraction") > 0.8 and at(37.7, 45.4, "lake_fraction") > 0.8, "Lakes Van and Urmia"
    assert at(38.6, 42.9, "floodplain_fraction") < 0.2, "lake surface is not floodplain"
    assert at(36.0, 38.3, "lake_fraction") == 0, "Lake Assad is a modern reservoir"


def test_engine_keeps_people_out_of_the_caspian(data, tmp_path):
    oikos = pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    r = region.load("m1-swasia")
    try:
        coast = coastline.build_pack(r, offline=True)
    except raster.MissingInputs:
        pytest.skip("coastline inputs not cached")
    lake = pack.read(data).values("lake_fraction")
    paths = []
    for kind, b in (("geometry", geometry.build(r)), ("coastline", coast), ("hydrology", data)):
        paths.append(str(tmp_path / f"{kind}.oikospack"))
        Path(paths[-1]).write_bytes(b)
    for seed in range(4):
        w = oikos.World.from_packs(paths, seed=seed)
        w.run_years(100, parts=2)
        assert w.population()[lake >= 1].sum() == 0 < w.population().sum()
