"""Rachis observations: archaeobotanical cereal assemblages read from a supplement's
tables into a Parquet table located on the region's cells."""

import io
import zipfile

import h3
import pyarrow.parquet as pq
import pytest

from oikos_data import rachis, region

BOX = region.Region(name="test-box", resolution=5, bbox=(30.0, 38.0, 34.0, 40.0), sha256="0" * 64)
SOURCES = [{"dataset": "test-rachis", "version": "1", "license": "CC-BY-4.0", "manifest_sha256": "1" * 64}]
HEADER = ["Site", "Latitude", "Longitude", "median age  (cal.BC)", "shattering %", "non-shattering %",
          "total n (det. Rachis)", "excluded indet. Rachis.", "primary source"]
W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"


def docx(*blocks):
    """A minimal .docx: strings become paragraphs, lists of rows become tables."""
    def p(s):
        return f"<w:p><w:r><w:t>{s}</w:t></w:r></w:p>"
    body = ""
    for b in blocks:
        if isinstance(b, str):
            body += p(b)
        else:
            body += "<w:tbl>" + "".join("<w:tr>" + "".join(f"<w:tc>{p(c)}</w:tc>" for c in r) + "</w:tr>" for r in b) + "</w:tbl>"
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        z.writestr("word/document.xml", f'<w:document xmlns:w="{W}"><w:body>{body}</w:body></w:document>')
    return buf.getvalue()


def table(rows):
    return pq.read_table(io.BytesIO(rachis.build(BOX, SOURCES, rows, {"barley": "Table S4."})))


def test_the_captioned_table_is_read_and_located():
    data = docx("Table S1. Rice.", [["sites", "date"], ["X", "1"]],
                "Table S4. Raw data on barley.",
                [HEADER, ["Netiv Hagdud", "31.98333", "35.45", "9080", "96.00%", "4.00%", "3277", "0", "Kislev, 1997"],
                 ["Abu Hureyra (2B)", "35.87", "38.40", "7400", "0%", "100%", "1", "13", "de Moulins, 1997"],
                 ["Far away", "50.0", "10.0", "7000", "0%", "100%", "10", "-", "Nobody"]],
                "Figure S2. A plot.")
    t = table(data).to_pydict()
    assert t["site"] == ["Netiv Hagdud", "Abu Hureyra (2B)"], "the site outside the region is dropped"
    assert t["crop"] == ["barley", "barley"]
    assert t["median_year"] == [-9080, -7400], "cal BC as calendar years"
    assert t["n_rachis"] == [3277, 1] and t["k_nonshattering"] == [131, 1]
    assert t["nonshattering"] == [pytest.approx(0.04), 1.0]
    assert t["excluded_indeterminate"] == [0, 13]
    assert t["h3"][0] == h3.str_to_int(h3.latlng_to_cell(31.98333, 35.45, 5))
    assert t["cell"][0] == list(BOX.cells()).index(t["h3"][0])
    assert t["source"][1] == "de Moulins, 1997"


def test_a_missing_or_changed_table_is_an_error():
    with pytest.raises(ValueError, match="Table S4"):
        table(docx("Table S1. Rice.", [["sites"], ["X"]]))
    with pytest.raises(ValueError, match="columns"):
        table(docx("Table S4. Barley.", [["Site", "Lat"], ["X", "1"]]))


def test_rebuilds_give_the_same_bytes():
    data = docx("Table S4. Barley.", [HEADER, ["Ramad", "33.42924", "36.09087", "6975", "40.90%", "59.10%", "455", "84", "vZ"]])
    a = rachis.build(BOX, SOURCES, data, {"barley": "Table S4."})
    assert a == rachis.build(BOX, SOURCES, data, {"barley": "Table S4."})
    assert pq.read_table(io.BytesIO(a)).schema.metadata[b"sources"]
