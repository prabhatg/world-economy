"""Foraging carrying capacity on the real M1 packs (ADR-0047): the engine's capacity map
against known geography and the Younger Dryas (runs when the packs' inputs are cached
and the bindings are built)."""

from pathlib import Path

import h3
import numpy as np
import pytest

from oikos_data import climate, coastline, geometry, hydrology, pack, raster, region


@pytest.fixture(scope="module")
def m1(tmp_path_factory):
    oikos = pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    r = region.load("m1-swasia")
    d = tmp_path_factory.mktemp("m1")
    paths = []
    try:
        for kind, build in (("geometry", lambda: geometry.build(r)), ("climate", lambda: climate.build_pack(r, offline=True)),
                            ("coastline", lambda: coastline.build_pack(r, offline=True)),
                            ("hydrology", lambda: hydrology.build_pack(r, offline=True))):
            paths.append(d / f"{kind}.oikospack")
            paths[-1].write_bytes(build())
    except raster.MissingInputs:
        pytest.skip("M1 pack inputs not cached; build the climate, coastline and hydrology packs first")
    area = pack.read(paths[0].read_bytes()).values("area_km2")
    cells = r.cells()

    def cell(lat, lng):
        return np.searchsorted(cells, h3.str_to_int(h3.latlng_to_cell(lat, lng, 5)))

    return oikos.World.from_packs([str(p) for p in paths], seed=1), area, cell


def test_capacity_map(m1):
    w, area, cell = m1
    density = w.capacity() / area
    zagros, galilee, pontic = cell(34.4, 46.5), cell(33.0, 35.4), cell(41.0, 40.5)
    nile, eastern_desert, nefud = cell(25.7, 32.65), cell(25.7, 33.6), cell(28.0, 41.0)
    for wet in (zagros, galilee, pontic):
        assert density[wet] > 0.1 > 3 * density[nefud], "forested and foothill zones feed far more than deserts"
    assert w.capacity()[nile] > 10 * w.capacity()[eastern_desert], "the Nile feeds people the desert beside it cannot"
    assert not w.habitable()[cell(39.5, 51.5)], "the Caspian"
    total = w.capacity()[w.habitable()].sum()
    assert 1e5 < total < 3e6, f"regional forager capacity at 12,000 BCE: {total}"


def test_younger_dryas_lowers_capacity(m1):
    w, _, _ = m1
    def total():
        return int(w.forage_capacity()[w.habitable()].sum())
    w.run_years(850, parts=4)    # 11,150 BCE: Allerod
    allerod = total()
    w.run_years(850, parts=4)    # 10,300 BCE: Younger Dryas
    younger_dryas = total()
    w.run_years(1300, parts=4)   # 9,000 BCE: early Holocene
    assert younger_dryas < 0.97 * allerod and younger_dryas < 0.97 * total()


def test_foragers_spread_over_the_region(m1):
    w, _, _ = m1
    w.run_years(3000 - w.tick // 365, parts=4)  # to 9,000 BCE whatever earlier tests ran
    pop, hab, cap = w.population(), w.habitable(), w.capacity()
    occupied = pop > 0
    assert occupied.sum() > 500, "bands disperse from a handful of start cells across the landscape (#14)"
    assert pop[~hab].sum() == 0
    assert pop.sum() <= 1.05 * cap[occupied].sum(), "the population stays within the capacity of the land it holds"
