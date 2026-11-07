"""The M1 rachis table (barley, emmer, einkorn) against the published supplement and the known record (runs when
the Allaby et al. 2017 supplement is cached: python -m oikos_data build rachis m1-swasia)."""

import io
import re
import zipfile

import pyarrow.parquet as pq
import pytest

from oikos_data import rachis, raster, region


@pytest.fixture(scope="module")
def supplement():
    try:
        return rachis.inputs(offline=True)
    except raster.MissingInputs:
        pytest.skip("rachis supplement not cached; run: python -m oikos_data build rachis m1-swasia")


@pytest.fixture(scope="module")
def m1():
    try:
        inputs = rachis.inputs(offline=True)
    except raster.MissingInputs:
        pytest.skip("rachis supplement not cached; run: python -m oikos_data build rachis m1-swasia")
    return pq.read_table(io.BytesIO(rachis.build(region.load("m1-swasia"), *inputs))).to_pydict()


def test_every_assemblage_is_in_the_region(m1):
    crops = m1["crop"]
    assert {c: crops.count(c) for c in set(crops)} == {"barley": 26, "emmer": 9, "einkorn": 8}
    total = {c: sum(n for k, n in zip(crops, m1["n_rachis"]) if k == c) for c in set(crops)}
    assert total == {"barley": 8095, "emmer": 672, "einkorn": 504}, "the supplement's totals (Figures S2, S5, S8)"


def test_the_wheats_read_as_the_supplement_plots_them(m1):
    # Values from the supplement's own charts of Tables S7 and S10 (cached in the .docx).
    row = {(c, s): (y, k, n) for c, s, y, k, n in zip(m1["crop"], m1["site"], m1["median_year"],
                                                       m1["k_nonshattering"], m1["n_rachis"])}
    assert row["emmer", "Aswad (I)"] == (-8300, 76, 297)
    assert row["emmer", "Chogha Golan"] == (-8200, 9, 16)
    assert row["emmer", "Salat Cami Yanı"] == (-6345, 105, 106)
    assert row["einkorn", "Qaramel"] == (-10275, 4, 18)
    assert row["einkorn", "Nevalı Çori"] == (-8275, 112, 355)
    assert row["einkorn", "El Kowm 2"] == (-6725, 19, 19)


def test_domestic_barley_appears_in_the_ppnb_and_dominates_by_the_late_neolithic(m1):
    barley = [i for i, c in enumerate(m1["crop"]) if c == "barley"]
    m1 = {k: [v[i] for i in barley] for k, v in m1.items()}
    share = {s: k / n for s, k, n in zip(m1["site"], m1["k_nonshattering"], m1["n_rachis"])}
    assert share["Iraq ed-Dubb (Natufian)"] == 0 and share["Jerf el-Ahmar"] < 0.01, "wild in the Natufian and PPNA"
    assert share["Çatalhöyük East"] == 1 and share["Abu Hureyra (2C)"] == 1
    early = [k / n for y, k, n in zip(m1["median_year"], m1["k_nonshattering"], m1["n_rachis"]) if y < -8_000]
    assert max(early) < 0.35, "no assemblage is mostly domestic before 8,000 BCE"


def test_the_wheats_match_every_point_the_supplement_plots(m1, supplement):
    # The supplement's charts of Tables S7 and S10 (Figures S5 and S8) cache each
    # assemblage's age and non-shattering share: an independent copy of the pictures' text.
    z = zipfile.ZipFile(io.BytesIO(supplement[1]))
    for chart, crop in (("chart1", "emmer"), ("chart2", "einkorn")):
        xml = z.read(f"word/charts/{chart}.xml").decode()
        points = set()
        for x, y in re.findall(r"<c:xVal>(.*?)</c:xVal>\s*<c:yVal>(.*?)</c:yVal>", xml, re.S):
            ages, shares = (re.findall(r"<c:v>([^<]*)</c:v>", v) for v in (x, y))
            points |= {(-int(a), round(float(p), 3)) for a, p in zip(ages, shares)}
        rows = {(y, round(p, 3)) for c, y, p in zip(m1["crop"], m1["median_year"], m1["nonshattering"]) if c == crop}
        assert rows == points, crop
