"""Survey sites: dated settlement phases from archaeological surveys, read from point
layers into a Parquet table located on the region's cells."""

import io

import h3
import numpy as np
import pyarrow.parquet as pq
import pytest

from oikos_data import region, sites

BOX = region.Region(name="test-box", resolution=5, bbox=(30.0, 38.0, 34.0, 40.0), sha256="0" * 64)
SOURCES = [{"dataset": "test-sites", "version": "1", "license": "CC-BY-4.0", "manifest_sha256": "1" * 64}]


def layer(*rows):
    keys = ["Id", "StartDate", "EndDate", "Longitude", "Latitude", "SizeHa", "Source"]
    return {k: np.array([r[i] for r in rows], dtype=object if k in ("Id", "Source") else float) for i, k in enumerate(keys)}


def table(layers):
    return pq.read_table(io.BytesIO(sites.build(BOX, SOURCES, layers))).to_pydict()


def test_site_phases_are_read_and_located():
    t = table({"north": layer(("A7", -6500, -6000, 35.5, 32.0, 1.5, "Survey A"),
                              ("A7", -6000, -5200, 35.5, 32.0, 0.0, "Survey A"),
                              ("B9", -3000, -2000, 36.0, 33.0, np.nan, "Survey B"),
                              ("C11", -6000, -5000, 10.0, 50.0, 2.0, "Far away"))})
    assert t["window"] == ["north"] * 3 and t["site"] == ["A7", "A7", "B9"], "the site outside the region is dropped"
    assert t["start_year"] == [-6500, -6000, -3000] and t["end_year"] == [-6000, -5200, -2000]
    assert t["h3"][0] == h3.str_to_int(h3.latlng_to_cell(32.0, 35.5, 5))
    assert t["cell"][0] == list(BOX.cells()).index(t["h3"][0])
    assert t["size_ha"][0] == 1.5 and np.isnan(t["size_ha"][1]) and np.isnan(t["size_ha"][2]), "0 means unknown"
    assert t["source"][2] == "Survey B"


def test_windows_keep_their_own_sites():
    t = table({"south": layer(("1", -7000, -6000, 35.0, 31.0, 1.0, "S")),
               "north": layer((1, -7000, -6000, 36.0, 33.0, 1.0, "N"))})
    assert t["window"] == ["north", "south"], "windows in name order"
    assert t["site"] == ["1", "1"], "site ids are per window, kept as text (numbers in some layers)"


def test_undated_or_reversed_phases_are_errors():
    with pytest.raises(ValueError, match="dates"):
        table({"north": layer(("1", np.nan, -6000, 35.5, 32.0, 1.0, "S"))})
    with pytest.raises(ValueError, match="dates"):
        table({"north": layer(("1", -6000, -6500, 35.5, 32.0, 1.0, "S"))})
