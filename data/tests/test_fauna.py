"""Caprine size and age by assemblage (Arbuckle et al. 2014; #37): sheep and goat body
size (LSI) and juvenile share of dated Anatolian faunal assemblages, from the paper's
supplementary tables (Word documents), located and dated."""

import io
import zipfile

import h3
import pyarrow.parquet as pq
import pytest

from oikos_data import fauna, region

BOX = region.Region(name="test-box", resolution=5, bbox=(36.0, 39.0, 30.0, 34.5), sha256="0" * 64)
SOURCES = [{"dataset": "test-fauna", "version": "1", "license": "CC-BY-4.0", "manifest_sha256": "1" * 64,
            "citation": "Test", "attribution": "Test data"}]


def docx(rows):
    """A Word document holding one table of `rows` (lists of cell strings)."""
    cell = lambda s: f"<w:tc><w:p><w:r><w:t>{s}</w:t></w:r></w:p></w:tc>"
    table = "".join("<w:tr>" + "".join(cell(c) for c in r) + "</w:tr>" for r in rows)
    xml = f'<w:document xmlns:w="w"><w:body><w:tbl>{table}</w:tbl></w:body></w:document>'
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        z.writestr("word/document.xml", xml)
    return buf.getvalue()


PHASES = docx([["Site", "Phases", "Region", "ky cal BC", "# specimens", "Author/Reference", "Link to dataset"],
               ["Çatalhöyük", "Early (KOPAL, South G)", "Central", "7.4-7", "14,173", "[3]", ""],
               ["Çatalhöyük", "Middle (South H-N)", "Central", "7-6.5", "20,118", "[3]", ""],
               ["Suberde", "Aceramic", "Lakes", "7.5-7", "359", "[17]", ""],
               ["Höyücek", "Lower Levels", "Lakes", "6.4-6.2", "NA", "[16]", ""],
               ["Höyücek", "Upper Levels", "Lakes", "6.2-6", "NA", "[16]", ""],
               ["Pinarbaşı", "A", "Central", "9", "294", "[6]", ""]])
HEADER = ["Site", "LSI mean", "sd", "N (LSI)", "%Juvenile", "N (%Juv)", "Author"]
OVIS = docx([HEADER,
             ["Çatalhöyük Early", "-0.0382", "0.036", "28", "+0.46", "-", "Russell et al."],
             ["Çatalhöyük Middle", "-0.0299", "0.042", "150", "0.53", "103", "Russell et al."],
             ["Suberde", "0.0197", "0.05", "42", "0.40", "29", "Arbuckle"],
             ["Höyücek", "-0.0542", "0.052", "19", "+*0.41", "40", "[7]"],
             ["+ based on teeth; * combined Ovis and Capra", "", "", "", "", "", ""]])
CAPRA = docx([HEADER, ["Suberde", "0.0278", "0.055", "14", "-", "-", "Arbuckle"]])
TEST_ASSEMBLAGES = {"Çatalhöyük Early": ("Çatalhöyük", ["Early (KOPAL, South G)"]),
                    "Çatalhöyük Middle": ("Çatalhöyük", ["Middle (South H-N)"]),
                    "Suberde": ("Suberde", ["Aceramic"]),
                    "Höyücek": ("Höyücek", ["Lower Levels", "Upper Levels"])}
TEST_SITES = {"Çatalhöyük": (37.666, 32.826), "Suberde": (37.35, 31.93), "Höyücek": (37.456, 30.59)}


def table(**kw):
    args = dict(assemblages=TEST_ASSEMBLAGES, sites=TEST_SITES)
    args.update(kw)
    return pq.read_table(io.BytesIO(fauna.build(BOX, SOURCES, PHASES, OVIS, CAPRA, **args))).to_pydict()


def test_word_tables_are_read_as_rows():
    assert fauna.docx_rows(CAPRA) == [HEADER, ["Suberde", "0.0278", "0.055", "14", "-", "-", "Arbuckle"]]


def test_assemblages_take_their_phases_dates_and_their_sites_place():
    t = table()
    i = t["assemblage"].index("Çatalhöyük Early")
    assert (t["start_year"][i], t["end_year"][i]) == (-7400, -7000), "ky cal BC"
    j = t["assemblage"].index("Höyücek")
    assert (t["start_year"][j], t["end_year"][j]) == (-6400, -6000), "several phases: their span"
    cell = h3.str_to_int(h3.latlng_to_cell(37.666, 32.826, 5))
    assert t["h3"][i] == cell and t["lat"][i] == 37.666


def test_sheep_and_goat_are_kept_apart_with_their_sample_sizes():
    t = table()
    s = t["assemblage"].index("Suberde")
    assert t["ovis_lsi"][s] == pytest.approx(0.0197) and t["ovis_lsi_n"][s] == 42
    assert t["capra_lsi"][s] == pytest.approx(0.0278) and t["capra_lsi_n"][s] == 14
    assert t["ovis_juvenile"][s] == pytest.approx(0.40) and t["ovis_juvenile_n"][s] == 29
    e = t["assemblage"].index("Çatalhöyük Early")
    assert t["capra_lsi"][e] is None, "no goat measured there"
    assert t["ovis_juvenile_n"][e] is None, "a share without its count"
    h = t["assemblage"].index("Höyücek")
    assert t["ovis_juvenile"][h] is None, "juveniles of sheep and goat combined are not sheep's"
    assert t["ovis_juvenile_basis"][e] == "teeth"


def test_a_single_date_spans_five_centuries():
    assert fauna.years("9") == (-9250, -8750)
    assert fauna.years("7.4-7") == (-7400, -7000)


def test_an_unmapped_assemblage_or_site_is_an_error():
    with pytest.raises(ValueError, match="Suberde"):
        table(assemblages={k: v for k, v in TEST_ASSEMBLAGES.items() if k != "Suberde"})
    with pytest.raises(ValueError, match="Höyücek"):
        table(sites={k: v for k, v in TEST_SITES.items() if k != "Höyücek"})


def test_assemblages_outside_the_region_are_dropped():
    far = dict(TEST_SITES, **{"Höyücek": (41.5, 26.5)})
    assert "Höyücek" not in table(sites=far)["assemblage"]
