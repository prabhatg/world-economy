"""Sickle blades among the flint tools of dated site phases (Maeda et al. 2016 Table S3;
#55): a record of how far people harvested cereals, apart from what they burnt."""

import io

import pyarrow.parquet as pq
import pytest

from oikos_data import region, sickles
from test_fauna import docx

BOX = region.Region(name="test-box", resolution=5, bbox=(31.0, 39.0, 33.0, 46.0), sha256="0" * 64)
SOURCES = [{"dataset": "test-sickles", "version": "1", "license": "CC-BY-4.0", "manifest_sha256": "1" * 64,
            "citation": "Test", "attribution": "Test data"}]
HEADER = ["sickle blade group", "region", "site", "median age cal. BC", "% sickle blades to all flint tools",
          "reference"]
TABLE = docx([["Late Epi-Palaeolithic", "", "", ""], HEADER,
               ["", "", "Hayonim Cave", "11700", "2.7-3.1%", "Belfer-Cohen and Bar-Yosef, 2012"],
               ["Group 1a", "South Levant", "Iraq ed-Dubb (Natufian)", "10850", "0%", "Kuijt and Goodale, 2006"],
               ["", "", "Abu Hureyra (1)", "10650", "rare", "Olszewski, 1986"],
               ["", "", "", "", "", ""],
               ["Pre-Pottery Neolithic A", "", "", ""], HEADER,
               ["", "", "Netiv Hagdud", "9075", "3.6%", "Nadel, 1997"],
               ["", "", "Dhra’", "9500", "1.4%", "Goodale et al., 2007"],
               ["Pre-Pottery Neolithic B", "", "", ""], HEADER,
               ["", "", "Jericho (PPNB)", "7850", "20.9%", "Crowfoot-Payne, 1983"],
               ["Pottery Neolithic", "", "", ""], HEADER,
               ["", "", "Ramad", "6975", "22.0-28.2%", "de Contenson, 2000"],
               ["", "", "Tepe Sabz (Sabz phase)", "5100", "23.1%", "Hole, 1977"]])
TEST_SITES = {"Hayonim Cave": (32.92, 35.22), "Iraq ed-Dubb": (32.3908, 35.7158), "Netiv Hagdud": (31.98, 35.38),
              "Dhra'": (31.255, 35.566)}


def table(**kw):
    args = dict(sites=TEST_SITES, left_out=set())
    args.update(kw)
    return pq.read_table(io.BytesIO(sickles.build(BOX, SOURCES, TABLE, **args))).to_pydict()


def test_phases_take_their_share_of_sickles_and_five_centuries_around_their_median_age():
    t = table()
    i = t["phase"].index("Netiv Hagdud")
    assert t["share"][i] == pytest.approx(0.036)
    assert (t["start_year"][i], t["end_year"][i]) == (-9325, -8825)
    assert t["site"][i] == "Netiv Hagdud" and t["h3"][i] != 0
    assert t["reference"][i] == "Nadel, 1997", "the primary source, cited with the phase"


def test_a_range_reads_as_its_midpoint_and_a_word_is_left_out():
    t = table()
    assert t["share"][t["phase"].index("Hayonim Cave")] == pytest.approx(0.029)
    assert t["share"][t["phase"].index("Iraq ed-Dubb (Natufian)")] == 0.0
    assert "Abu Hureyra (1)" not in t["phase"], "'rare' gives no share"


def test_either_apostrophe_names_the_same_site():
    assert "Dhra'" in table()["phase"], "Table S3 spells it both ways"


def test_phases_from_8800_to_7000_bc_are_not_carried_and_later_ones_anchor_unlocated():
    # ADR-0143: sickles are judged only before 8800 BC, where domestication confounds
    # them least; phases from 7000 BC anchor a farming world and need no place.
    t = table()
    assert "Jericho (PPNB)" not in t["phase"]
    for p in ("Ramad", "Tepe Sabz (Sabz phase)"):
        i = t["phase"].index(p)
        assert t["h3"][i] == 0 and t["cell"][i] is None
    assert t["share"][t["phase"].index("Ramad")] == pytest.approx(0.251)


def test_phases_are_ordered_by_start_and_out_of_region_ones_dropped():
    t = table()
    assert t["start_year"] == sorted(t["start_year"])
    assert "Netiv Hagdud" not in table(sites=dict(TEST_SITES, **{"Netiv Hagdud": (31.98, 47.5)}))["phase"]


def test_an_unlocated_judged_phase_is_an_error_unless_left_out():
    sites = {k: v for k, v in TEST_SITES.items() if k != "Netiv Hagdud"}
    with pytest.raises(ValueError, match="Netiv Hagdud"):
        table(sites=sites)
    assert "Netiv Hagdud" not in table(sites=sites, left_out={"Netiv Hagdud"})["phase"]
    with pytest.raises(ValueError, match="Nowhere"):
        table(left_out={"Nowhere"})
