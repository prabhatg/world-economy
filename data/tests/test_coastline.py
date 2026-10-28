"""Coastline pack: land fraction per cell and century from a paleo-DEM's land mask."""

import h3
import numpy as np
import pytest

from oikos_data import coastline, pack, raster, region

BOX = region.Region(name="test-box", resolution=5, bbox=(35.2, 35.8, 40.2, 40.8), sha256="0" * 64)
MANIFEST = {"dataset": "test-dem", "version": "1", "citation": "Test",
            "license": {"id": "T", "attribution": "contains test dem"}, "access": {"time_ids": [-101, -99]}}
SHA = "1" * 64
N = 60


def tile(fn):
    lat = 36.2 - (np.arange(int(1.4 * N)) + 0.5) / N
    lng = 39.8 + (np.arange(int(1.4 * N)) + 0.5) / N
    la, lo = np.meshgrid(lat, lng, indexing="ij")
    return raster.Tile(np.asarray(fn(la, lo), dtype=np.float32), north=36.2, west=39.8, dlat=1 / N, dlng=1 / N)


def shore(west_of):
    """Valid fractions of a DEM with sea west of a meridian."""
    return tile(lambda la, lo: (lo >= west_of).astype(float))


def build(tiles):
    return pack.read(coastline.build(BOX, MANIFEST, SHA, tiles))


def centers():
    return np.array([h3.cell_to_latlng(h3.int_to_str(int(c))) for c in BOX.cells()])


def test_the_sea_advances_over_time():
    p = build({-101: shore(40.0), -100: shore(40.5), -99: shore(41.0)})
    assert p.header["kind"] == "coastline"
    assert p.header["time"] == {"calendar_year": [-10150, -10050, -9950], "rule": "linear"}
    land = p.values("land_fraction")
    lng = centers()[:, 1]
    assert np.all(land[0] == 1) and np.all(land[2] == 0)
    assert np.all(land[1][lng < 40.4] < 0.05) and np.all(land[1][lng > 40.6] > 0.95)
    assert p.header["sources"][0]["attribution"] == "contains test dem"


def test_partial_cells_average_their_samples():
    p = build({t: tile(lambda la, lo: 0.25 + 0 * la) for t in (-101, -100, -99)})
    np.testing.assert_allclose(p.values("land_fraction"), 0.25)
    assert p.raw("land_fraction").dtype == np.uint16


def test_missing_slices_fail():
    with pytest.raises(KeyError):
        build({-101: shore(40.0)})
