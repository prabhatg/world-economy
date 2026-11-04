"""The African Humid Period (ADR-0084): the rain it added over Africa at its height, from
the Eastern Saharan reconstructions of Bartlein et al. (2011), falling off northward."""

import math

import h3
import numpy as np
import pytest

from oikos_data import humid, pack, region

BOX = region.Region(name="test-box", resolution=4, bbox=(22.0, 32.0, 26.0, 36.0), sha256="0" * 64)
SOURCES = [{"dataset": "test-humid", "version": "1", "license": "CC0-1.0", "manifest_sha256": "1" * 64,
            "citation": "Test", "attribution": "Test data"}]
EASTERN_SAHARA = (18.0, 23.0, 24.0, 34.0)
ROWS = [{"lat": 19.0, "lon": 27.0, "map_anm_mean": 604.7667},
        {"lat": 21.0, "lon": 29.0, "map_anm_mean": 177.1667},
        {"lat": 21.0, "lon": 17.0, "map_anm_mean": 459.12},  # central Sahara: outside the box
        {"lat": 31.0, "lon": -7.0, "map_anm_mean": -188.5}]


def cell_at(r, lat, lng):
    return list(r.cells()).index(h3.str_to_int(h3.latlng_to_cell(lat, lng, r.resolution)))


def test_the_anomaly_falls_off_northward_through_the_eastern_saharan_cells():
    a, b = humid.anchor(ROWS, EASTERN_SAHARA)
    assert math.isclose(math.exp(a + b * 19.0), 604.7667, rel_tol=1e-9)
    assert math.isclose(math.exp(a + b * 21.0), 177.1667, rel_tol=1e-9)
    assert b < 0, "less rain northward"


def test_the_anomaly_needs_two_cells_that_fall_off_northward():
    with pytest.raises(ValueError, match="two"):
        humid.anchor(ROWS[:1], EASTERN_SAHARA)
    with pytest.raises(ValueError, match="northward"):
        humid.anchor([{**ROWS[0], "lat": 21.0}, {**ROWS[1], "lat": 19.0}], EASTERN_SAHARA)


def test_africa_ends_at_the_suez_isthmus_and_the_red_sea():
    assert humid.african(25.5, 29.0), "Dakhla"
    assert humid.african(30.0, 31.2), "Cairo"
    assert humid.african(24.1, 32.9), "Aswan"
    assert not humid.african(29.5, 33.8), "Sinai"
    assert not humid.african(30.5, 34.8), "the Negev"
    assert not humid.african(26.0, 37.0), "Arabia's Red Sea coast"
    assert not humid.african(-10.0, -50.0), "Brazil: outside the reconstruction's domain"
    assert not humid.african(12.0, 25.0), "the Sahel, south of the southernmost anchor"
    assert not humid.african(40.0, 20.0), "Europe"
    assert not humid.african(37.0, 27.5), "western Anatolia, north of the Mediterranean"
    assert not humid.african(35.0, 25.0), "Crete"
    assert humid.african(36.8, 10.2), "Tunis"


def test_the_pack_holds_the_anomaly_over_africa_and_none_beyond():
    p = pack.read(humid.build_pack(BOX, SOURCES, ROWS, EASTERN_SAHARA))
    assert p.header["kind"] == "humid_period"
    assert [f["name"] for f in p.header["fields"]] == ["precip_anomaly"]
    v = p.values("precip_anomaly")
    a, b = humid.anchor(ROWS, EASTERN_SAHARA)
    dakhla = cell_at(BOX, 25.5, 29.0)
    lat = h3.cell_to_latlng(h3.int_to_str(int(BOX.cells()[dakhla])))[0]
    assert v[dakhla] == pytest.approx(math.exp(a + b * lat), abs=0.05), "at the cell's center, to the stored 0.1 mm"
    assert v[cell_at(BOX, 23.0, 29.0)] > v[dakhla] > v[cell_at(BOX, 29.0, 29.0)] > 0
    assert v[cell_at(BOX, 29.5, 33.8)] == 0 and v[cell_at(BOX, 30.5, 34.8)] == 0, "Sinai and the Negev are Asia"
    assert np.all(v >= 0)
    assert p.header["sources"][0]["attribution"] == "Test data"
