"""Cultivation and domestication on the real M1 packs (ADR-0049): where wild cereals grow,
the Younger Dryas as climate stress, and when the default (placeholder) parameters farm
and domesticate (runs when the packs' inputs are cached and the bindings are built).
These are stylized facts, not calibration: the parameters are fitted in #19."""

import h3
import numpy as np
import pytest

from oikos_data import climate, coastline, geometry, hydrology, raster, region


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
    cells = r.cells()

    def cell(lat, lng):
        return np.searchsorted(cells, h3.str_to_int(h3.latlng_to_cell(lat, lng, 5)))

    return oikos.World.from_packs([str(p) for p in paths], seed=1), cell


def run_to(w, bce):
    w.run_years(12_000 - bce - w.tick // 365, parts=4)


def test_wild_cereals_grow_in_the_hilly_flanks(m1):
    w, cell = m1
    s = w.suitability()
    for name, at in {"Damascus basin": (33.5, 36.3), "Zagros": (34.4, 46.5), "Karacadag": (37.7, 39.8)}.items():
        assert s[cell(*at)] > 0.5, name
    for name, at in {"Nefud": (28.0, 41.0), "Mesopotamian alluvium": (31.0, 46.0), "Nile valley": (25.7, 32.65),
                     "Pontic coast": (41.0, 40.5)}.items():
        assert s[cell(*at)] == 0, f"{name}: too dry, or too wet for open stands"


def test_the_younger_dryas_registers_as_climate_stress(m1):
    w, _ = m1
    def mean_stress():
        return float(w.climate_stress()[w.habitable()].mean())
    run_to(w, 11_000)     # late Allerod
    allerod = mean_stress()
    run_to(w, 10_800)     # Younger Dryas onset
    onset = mean_stress()
    run_to(w, 10_000)     # early Holocene warming
    assert onset > 2 * allerod and onset > 2 * mean_stress(), (allerod, onset)


def test_farming_stays_where_wild_cereals_grow_and_domestication_is_protracted(m1):
    w, cell = m1
    run_to(w, 9_500)
    # Weighted by cultivators: a few people at the region's edge, where no wild stands
    # dilute the crop, may drift to domestic forms early (two cells, 243 people, in the
    # round-6 world; ADR-0064); rapid selection (0.02) puts 98% of cultivators there.
    sown = w.population() * w.cultivation()
    assert sown[w.domestication() > 0.5].sum() < 0.01 * sown.sum(), "domestic crops do not dominate before the PPNB"
    run_to(w, 7_000)
    e, dom, pop = w.cultivation(), w.domestication(), w.population()
    farmed = pop * e
    assert farmed[w.suitability() > 0].sum() > 0.99 * farmed.sum()
    assert e[cell(28.0, 41.0)] < 0.01, "nobody farms the Nefud (bands arriving from fields lose the habit)"
    assert (e > 0.2).sum() > 1000, "cultivation is widespread by 7000 BCE"
    farming = (e > 0.3) & (dom > 0.5)
    assert farming.sum() > 100, "domestic crops dominate in places by 7000 BCE"
    cap, forage = w.capacity()[farming].sum(), w.forage_capacity()[farming].sum()
    assert cap > 3 * forage, f"farmland feeds more than foraging: {cap} vs {forage}"
