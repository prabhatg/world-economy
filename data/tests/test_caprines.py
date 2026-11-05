"""Where wild sheep and goats could live (#37): uplands of Asia, from terrain. The
engine adds a moisture ramp each year from the climate pack."""

import h3
import numpy as np
import pytest

from oikos_data import caprines, pack, region

BOX = region.Region(name="test-box", resolution=4, bbox=(28.0, 39.0, 30.0, 40.0), sha256="0" * 64)


def test_uplands_are_high_or_rugged_land():
    assert caprines.upland(1100.0, 2.4) == 1.0, "the Anatolian plateau"
    assert caprines.upland(250.0, 4.7) == 1.0, "the Levant's hills: rugged though low"
    assert caprines.upland(22.0, 0.1) == 0.0, "the Mesopotamian alluvium"
    assert caprines.upland(600.0, 0.0) == pytest.approx(0.5), "a plateau halfway up"
    assert caprines.upland(0.0, 2.5) == pytest.approx(0.5), "rolling hills"


def terrain_pack(elev, slope):
    cells = BOX.cells()
    b = pack.Builder("terrain", pack.region(BOX.name, BOX.resolution, cells))
    b.source(dataset="test-dem", version="1", license="test", citation="Test DEM", manifest_sha256="1" * 64)
    b.field("elev_mean", "m", "mean", np.full(len(cells), elev, dtype=np.int16))
    b.field("slope_mean", "degrees", "mean", np.full(len(cells), int(slope * 100), dtype=np.uint16), scale=0.01)
    return b.build()


def test_the_pack_holds_asias_uplands_and_no_african_ones():
    p = pack.read(caprines.build_pack(BOX, terrain_pack(1200, 5.0)))
    assert p.header["kind"] == "caprines"
    v = p.values("upland")
    cells = list(BOX.cells())
    at = lambda lat, lng: v[cells.index(h3.str_to_int(h3.latlng_to_cell(lat, lng, BOX.resolution)))]
    assert at(37.7, 32.8) == 1.0, "Anatolia"
    assert at(29.0, 31.0) == 0.0, "Egypt: no wild sheep or goat to tame"
    assert p.header["sources"][0]["dataset"] == "test-dem", "the terrain's sources carry over"
