"""The M1 rachis table against the published supplement and the known record (runs when
the Allaby et al. 2017 supplement is cached: python -m oikos_data build rachis m1-swasia)."""

import io

import pyarrow.parquet as pq
import pytest

from oikos_data import rachis, raster, region


@pytest.fixture(scope="module")
def m1():
    try:
        inputs = rachis.inputs(offline=True)
    except raster.MissingInputs:
        pytest.skip("rachis supplement not cached; run: python -m oikos_data build rachis m1-swasia")
    return pq.read_table(io.BytesIO(rachis.build(region.load("m1-swasia"), *inputs))).to_pydict()


def test_every_barley_assemblage_is_in_the_region(m1):
    assert len(m1["site"]) == 26 and set(m1["crop"]) == {"barley"}
    assert sum(m1["n_rachis"]) == 8095, "the supplement's own total (Figure S2)"


def test_domestic_barley_appears_in_the_ppnb_and_dominates_by_the_late_neolithic(m1):
    share = {s: k / n for s, k, n in zip(m1["site"], m1["k_nonshattering"], m1["n_rachis"])}
    assert share["Iraq ed-Dubb (Natufian)"] == 0 and share["Jerf el-Ahmar"] < 0.01, "wild in the Natufian and PPNA"
    assert share["Çatalhöyük East"] == 1 and share["Abu Hureyra (2C)"] == 1
    early = [k / n for y, k, n in zip(m1["median_year"], m1["k_nonshattering"], m1["n_rachis"]) if y < -8_000]
    assert max(early) < 0.35, "no assemblage is mostly domestic before 8,000 BCE"
