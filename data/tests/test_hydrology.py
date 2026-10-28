"""Hydrology pack: rivers, floodplains and lakes per cell from vector datasets."""

import h3
import numpy as np
import pytest
import shapely

from oikos_data import hydrology, pack, region

BOX = region.Region(name="test-box", resolution=5, bbox=(35.2, 35.8, 40.2, 40.8), sha256="0" * 64)
RIVERS = {"dataset": "test-rivers", "version": "1", "citation": "Rivers", "license": {"id": "CC-BY-4.0", "attribution": "rivers (c) test"}}
LAKES = {"dataset": "test-lakes", "version": "1", "citation": "Lakes", "license": {"id": "CC-BY-4.0", "attribution": "lakes (c) test"}}
KM_PER_DEG = 6371.0088 * np.pi / 180


def rivers(*reaches):
    """(LineString, discharge m3/s, Strahler order, catchment km2, max inundation %) per reach."""
    geoms = [r[0] for r in reaches]
    cols = ["DIS_AV_CMS", "ORD_STRA", "CATCH_SKM", "inu_pc_cmx"]
    return geoms, {c: np.array([r[i + 1] for r in reaches], dtype=float) for i, c in enumerate(cols)}


def lakes(*polys):
    """(Polygon, Lake_type) per lake; type 2 is a reservoir."""
    return [p[0] for p in polys], {"Lake_type": np.array([p[1] for p in polys])}


def build(r=None, l=None):
    r = r or rivers()
    l = l or lakes()
    return pack.read(hydrology.build(BOX, (RIVERS, "1" * 64, r), (LAKES, "2" * 64, l)))


def area_km2():
    return np.array([h3.cell_area(h3.int_to_str(int(c)), "km^2") for c in BOX.cells()])


def test_a_reach_gives_length_discharge_order_and_floodplain():
    line = shapely.LineString([(40.4, 35.5), (40.6, 35.5)])
    p = build(rivers((line, 100.0, 4, 50.0, 20.0)))
    assert p.header["kind"] == "hydrology" and "time" not in p.header
    length = p.values("river_length")
    want = 0.2 * KM_PER_DEG * np.cos(np.radians(35.5))
    assert length.sum() == pytest.approx(want, abs=0.05)
    wet = length > 0
    assert 1 <= wet.sum() <= 4
    assert np.all(p.values("river_discharge")[wet] == 100) and np.all(p.values("river_order")[wet] == 4)
    assert np.all(p.values("river_discharge")[~wet] == 0) and np.all(p.values("river_order")[~wet] == 0)
    flood = (p.values("floodplain_fraction") * area_km2()).sum()
    assert flood == pytest.approx(50 * 0.2, rel=0.02), "the reach's inundated catchment area, spread along it"
    assert np.all(p.values("lake_fraction") == 0)
    assert [s["dataset"] for s in p.header["sources"]] == ["test-rivers", "test-lakes"]


def test_the_largest_reach_in_a_cell_sets_discharge_and_order():
    a = shapely.LineString([(40.45, 35.45), (40.55, 35.55)])
    b = shapely.LineString([(40.45, 35.55), (40.55, 35.45)])
    p = build(rivers((a, 10.0, 2, 5.0, 0.0), (b, 300.0, 6, 5.0, 0.0)))
    c = np.searchsorted(BOX.cells(), h3.str_to_int(h3.latlng_to_cell(35.5, 40.5, 5)))
    assert p.values("river_discharge")[c] == 300 and p.values("river_order")[c] == 6


def test_natural_lakes_count_and_reservoirs_do_not():
    lake = shapely.box(40.45, 35.45, 40.55, 35.55)
    reservoir = shapely.box(40.25, 35.25, 40.35, 35.35)
    p = build(l=lakes((lake, 1), (reservoir, 2)))
    area = (p.values("lake_fraction") * area_km2()).sum()
    want = (0.1 * KM_PER_DEG) * (0.1 * KM_PER_DEG * np.cos(np.radians(35.5)))
    assert area == pytest.approx(want, rel=0.03)
    c = np.searchsorted(BOX.cells(), h3.str_to_int(h3.latlng_to_cell(35.3, 40.3, 5)))
    assert p.values("lake_fraction")[c] == 0


def test_is_reproducible():
    r = rivers((shapely.LineString([(40.4, 35.5), (40.6, 35.6)]), 7.0, 3, 9.0, 11.0))
    l = lakes((shapely.box(40.45, 35.45, 40.55, 35.55), 1))
    a = hydrology.build(BOX, (RIVERS, "1" * 64, r), (LAKES, "2" * 64, l))
    assert a == hydrology.build(BOX, (RIVERS, "1" * 64, r), (LAKES, "2" * 64, l))


def test_river_length_counts_only_reaches_with_flowing_water():
    wadi = shapely.LineString([(40.4, 35.5), (40.6, 35.5)])
    p = build(rivers((wadi, 0.05, 3, 50.0, 0.0)))
    assert p.values("river_length").sum() == 0, "a wadi averaging 0.05 m3/s is not a river"
    assert p.values("river_discharge").max() == pytest.approx(0.05) and p.values("river_order").max() == 3


def test_floodplain_excludes_lake_surface():
    lake = shapely.box(40.3, 35.3, 40.7, 35.7)
    line = shapely.LineString([(40.4, 35.5), (40.6, 35.5)])
    p = build(rivers((line, 100.0, 4, 200.0, 100.0)), lakes((lake, 1)))
    c = np.searchsorted(BOX.cells(), h3.str_to_int(h3.latlng_to_cell(35.5, 40.5, 5)))
    assert p.values("lake_fraction")[c] > 0.9 and p.values("floodplain_fraction")[c] < 0.1
