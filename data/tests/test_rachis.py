"""Rachis observations: archaeobotanical cereal assemblages read from a supplement's
tables into a Parquet table located on the region's cells."""

import io
import struct
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


R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
A = "http://schemas.openxmlformats.org/drawingml/2006/main"


def emf(texts):
    """A minimal EMF: (x, y, text) drawn as EXTTEXTOUTW records in a font 120 units high,
    60 units per character."""
    out = struct.pack("<II", 1, 8)  # a header record, skipped
    out += struct.pack("<IIIi", 82, 16, 1, -120)  # the font (truncated after its height)
    for x, y, s in texts:
        chars = s.encode("utf-16le") + (b"\0\0" if len(s) % 2 else b"")
        dx = struct.pack(f"<{len(s)}i", *[60] * len(s))
        body = struct.pack("<16xIff", 1, 0.0, 0.0) + struct.pack("<iiIII16sI", x, y, len(s), 76, 0, b"", 76 + len(chars))
        out += struct.pack("<II", 84, 8 + len(body) + len(chars) + len(dx)) + body + chars + dx
    return out + struct.pack("<II", 14, 8)  # end of file


def docx(*blocks):
    """A minimal .docx: strings become paragraphs, lists of rows become tables and bytes
    become an embedded EMF picture."""
    def p(s):
        return f"<w:p><w:r><w:t>{s}</w:t></w:r></w:p>"
    body, media = "", {}
    for b in blocks:
        if isinstance(b, str):
            body += p(b)
        elif isinstance(b, bytes):
            rid = f"rId{len(media) + 1}"
            media[rid] = b
            body += f'<w:p><w:r><w:drawing><a:blip r:embed="{rid}"/></w:drawing></w:r></w:p>'
        else:
            body += "<w:tbl>" + "".join("<w:tr>" + "".join(f"<w:tc>{p(c)}</w:tc>" for c in r) + "</w:tr>" for r in b) + "</w:tbl>"
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        z.writestr("word/document.xml",
                   f'<w:document xmlns:w="{W}" xmlns:r="{R}" xmlns:a="{A}"><w:body>{body}</w:body></w:document>')
        rels = "".join(f'<Relationship Id="{rid}" Target="media/image{rid[3:]}.emf"/>' for rid in media)
        z.writestr("word/_rels/document.xml.rels", f"<Relationships>{rels}</Relationships>")
        for rid, b in media.items():
            z.writestr(f"word/media/image{rid[3:]}.emf", b)
    return buf.getvalue()


def table(rows, tables=None):
    return pq.read_table(io.BytesIO(rachis.build(BOX, SOURCES, rows, tables or {"barley": "Table S4."})))


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


# Tables S7 and S10 as the supplement draws them: centred cells, a header over three lines,
# a row whose site name sits a little below its coordinates, and one whose site name
# reaches under the latitude's header.
EMMER = emf([
    (3200, 300, "median age"), (4930, 300, "non-shattering"), (5860, 300, "total n (det."), (6700, 300, "excluded"),
    (40, 370, "Site"), (1460, 370, "Latitude"), (2330, 370, "Longitude"), (4060, 370, "shattering %"),
    (7600, 370, "primary source"),
    (3200, 450, "(cal. BC)"), (4930, 450, "%"), (5860, 450, "Rachis)"), (6700, 450, "indet. Rachis."),
    (40, 800, "Aswad (I)"), (1610, 800, "33.40415"), (2480, 800, "36.55006"), (3460, 800, "8300"),
    (4260, 800, "74.40%"), (5130, 800, "25.60%"), (6090, 800, "297"), (6930, 800, "1857"),
    (7820, 800, "Tanno and Willcox 2012"),
    (1610, 990, "37.58"), (2480, 990, "38.65"),
    (40, 1000, "Nevalı Çori"), (3460, 1000, "8275"), (4260, 1000, "68.50%"), (5130, 1000, "31.50%"),
    (6090, 1000, "355"), (6930, 1000, "-"), (7820, 1000, "Tanno and Willcox, 2012"),
    (40, 1200, "Tell Qarassa North"), (1560, 1200, "32.7"), (2480, 1200, "36.3"), (3460, 1200, "8495"),
    (4260, 1200, "78.95%"), (5130, 1200, "21.05%"), (6090, 1200, "19"), (6930, 1200, "0"),
    (7820, 1200, "Arranz-Otaegui et al 2016"),
])


def test_a_table_drawn_as_a_picture_is_read_from_its_text():
    t = table(docx("Table S7. Raw data on emmer.", EMMER, "Figure S5."), {"emmer": "Table S7."}).to_pydict()
    assert t["site"] == ["Aswad (I)", "Nevalı Çori", "Tell Qarassa North"]
    assert t["crop"] == ["emmer"] * 3
    assert t["median_year"] == [-8300, -8275, -8495]
    assert t["lat"] == [33.40415, 37.58, 32.7] and t["lng"] == [36.55006, 38.65, 36.3], "cells on two baselines"
    assert t["n_rachis"] == [297, 355, 19] and t["k_nonshattering"] == [76, 112, 4]
    assert t["excluded_indeterminate"] == [1857, 0, 0]
    assert t["source"][:2] == ["Tanno and Willcox 2012", "Tanno and Willcox, 2012"]


def test_a_picture_with_other_columns_is_an_error():
    other = emf([(40, 370, "Site"), (1460, 370, "Latitude"), (40, 800, "Aswad"), (1610, 800, "33.4")])
    with pytest.raises(ValueError, match="columns"):
        table(docx("Table S7. Emmer.", other), {"emmer": "Table S7."})


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
