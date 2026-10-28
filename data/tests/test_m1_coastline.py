"""The M1 coastline pack against the known postglacial transgression (runs when the
paleo-DEM windows are cached: `python -m oikos_data build coastline m1-swasia` reads
them once, ~5 min)."""

from pathlib import Path

import h3
import numpy as np
import pytest

from oikos_data import coastline, geometry, pack, raster, region


@pytest.fixture(scope="module")
def data():
    try:
        return coastline.build_pack(region.load("m1-swasia"), offline=True)
    except raster.MissingInputs:
        pytest.skip("paleo-DEM windows not cached; run: python -m oikos_data build coastline m1-swasia")


@pytest.fixture(scope="module")
def m1(data):
    cells = region.load("m1-swasia").cells()
    p = pack.read(data)
    years = np.array(p.header["time"]["calendar_year"])
    land = p.values("land_fraction")

    def at(lat, lng, year):
        return land[np.searchsorted(years, year), np.searchsorted(cells, h3.str_to_int(h3.latlng_to_cell(lat, lng, 5)))]

    return p, land, at


def test_the_sea_only_advances(m1):
    p, land, _ = m1
    assert p.header["kind"] == "coastline" and land.shape == (92, 22198)
    assert np.all(np.diff(land, axis=0) <= 0), "sea level rises through 12,000-3,000 BCE, so no cell gains land"
    lost = (land[0] > 0).sum() - (land[-1] > 0).sum()
    assert 500 < lost < 1500, "the paleo-Gulf basin and coastal shelves flood"


def test_the_gulf_floods_in_the_early_holocene(m1):
    _, _, at = m1
    assert at(27.0, 51.0, -12050) == 1 and at(27.0, 51.0, -10050) == 1, "central Gulf dry before ~10,000 BCE"
    assert 0 < at(27.0, 51.0, -9050) < 0.5, "flooding under way"
    assert at(27.0, 51.0, -6050) == 0 and at(27.0, 51.0, -3050) == 0, "a sea by ~6,000 BCE"
    for lat, lng in [(30.96, 46.1), (35.0, 33.0), (37.67, 32.83)]:  # Ur, Cyprus, Catalhoyuk
        assert at(lat, lng, -12050) == at(lat, lng, -3050) == 1
    for lat, lng in [(33.5, 30.0), (41.8, 30.0)]:  # eastern Mediterranean, Black Sea
        assert at(lat, lng, -12050) == 0


def test_known_gaps_inland_waters_count_as_land(m1):
    """The paleo-DEM masks only the ocean: the Caspian (flat at 0 m) and the Sea of
    Marmara (kept as bathymetry) count as land, as lakes do in the terrain pack. A lake
    mask (hydrology, #9) must remove them; this test fails once it does."""
    _, _, at = m1
    assert at(39.5, 51.5, -3050) == 1 and at(40.75, 28.0, -3050) == 1


def test_engine_keeps_people_on_land(m1, data, tmp_path):
    oikos = pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    _, land, _ = m1
    paths = []
    for kind, b in (("geometry", geometry.build(region.load("m1-swasia"))), ("coastline", data)):
        paths.append(str(tmp_path / f"{kind}.oikospack"))
        Path(paths[-1]).write_bytes(b)
    for seed in range(4):
        w = oikos.World.from_packs(paths, seed=seed)
        assert w.population()[land[0] == 0].sum() == 0 < w.population().sum()
        w.run_years(200, parts=2)
        assert w.population()[land[2] == 0].sum() == 0, "nobody in cells that are sea in 11,850 BCE"
