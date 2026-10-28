"""The M1 region (ADR-0040): res-5 cells with centers in 24-42N, 26-58E."""

import h3
import numpy as np

from oikos_data import region

M1 = region.load("m1-swasia")


def cell_of(lat, lng):
    return int(h3.str_to_int(h3.latlng_to_cell(lat, lng, 5)))


def test_m1_region_cells():
    cells = M1.cells()
    assert M1.resolution == 5 and len(cells) == 22198
    assert cells.dtype == np.uint64 and np.all(np.diff(cells.astype(np.int64)) > 0)
    for c in cells[::997]:
        lat, lng = h3.cell_to_latlng(h3.int_to_str(int(c)))
        assert 24 <= lat <= 42 and 26 <= lng <= 58
        assert h3.get_resolution(h3.int_to_str(int(c))) == 5


def test_m1_region_covers_the_key_areas():
    cells = set(int(c) for c in M1.cells())
    inside = {
        "Jericho": (31.87, 35.44),
        "Catalhoyuk": (37.67, 32.83),
        "Shillourokambos, Cyprus": (34.68, 33.02),
        "Gobekli Tepe": (37.22, 38.92),
        "Ganj Dareh": (34.43, 47.48),
        "Eridu": (30.82, 45.99),
        "Persian Gulf basin": (27.0, 51.0),
        "Nile delta": (30.9, 31.0),
        "Fayum": (29.3, 30.8),
    }
    for name, (lat, lng) in inside.items():
        assert cell_of(lat, lng) in cells, name
    for name, (lat, lng) in {"Crimea": (45.0, 34.0), "Western Desert": (26.0, 25.0)}.items():
        assert cell_of(lat, lng) not in cells, name


def test_region_file_records_its_rule_and_hash():
    assert M1.sha256 and len(M1.sha256) == 64
    assert M1.bbox == (24.0, 42.0, 26.0, 58.0)
