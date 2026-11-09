"""The M1 wild-wheat ranges against where the wheats' ancestors are known to grow, from
the GBIF records' snapshot in data/snapshots."""

import h3
import numpy as np
import pytest

from oikos_data import pack, raster, region, wheats


@pytest.fixture(scope="module")
def ranges():
    r = region.load("m1-swasia")
    try:
        p = pack.read(wheats.build_pack(r, offline=True))
    except raster.MissingInputs:
        pytest.skip("no snapshot of the GBIF records in data/snapshots")
    index = {int(c): i for i, c in enumerate(r.cells())}
    at = lambda f, lat, lng: f[index[h3.str_to_int(h3.latlng_to_cell(lat, lng, 5))]]
    return {c: (lambda lat, lng, f=p.values(c): at(f, lat, lng)) for c in wheats.CROPS}, p


def test_wild_emmer_grows_in_the_fertile_crescents_arc(ranges):
    at, _ = ranges
    # The upper Jordan valley and Galilee (Ammiad), Karacadağ, the hills east of the Jordan.
    for lat, lng in [(32.98, 35.6), (37.67, 39.83), (32.53, 35.78)]:
        assert at["emmer"](lat, lng) == 1.0, (lat, lng)
    # Not on the central Anatolian plateau (Aşıklı Höyük), the Mesopotamian alluvium or the Nile.
    for lat, lng in [(38.35, 34.23), (31.3, 45.6), (30.0, 31.2)]:
        assert at["emmer"](lat, lng) == 0.0, (lat, lng)


def test_wild_einkorn_ranges_wider_north_and_east_but_not_into_the_lowlands(ranges):
    at, p = ranges
    # Karacadağ (Nevalı Çori nearby), the northern Iraqi Zagros, western Anatolia.
    for lat, lng in [(37.58, 38.65), (36.4, 44.2)]:
        assert at["einkorn"](lat, lng) == 1.0, (lat, lng)
    for lat, lng in [(31.3, 45.6), (30.0, 31.2), (35.2, 38.85)]:  # alluvium, Nile, El Kowm
        assert at["einkorn"](lat, lng) == 0.0, (lat, lng)
    emmer, einkorn = p.values("emmer"), p.values("einkorn")
    assert einkorn.sum() > 1.5 * emmer.sum() > 0
    assert 0.02 < emmer.mean() < 0.1 and 0.05 < einkorn.mean() < 0.2


def test_every_dataset_is_credited(ranges):
    _, p = ranges
    sources = p.header["sources"]
    assert sources[0]["dataset"] == "gbif-wild-wheats"
    assert any("crop wild relatives" in s["citation"] for s in sources[1:])
    assert all(s["license"] in ("CC-BY-4.0", "CC0-1.0") for s in sources)
