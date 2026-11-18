"""Cereal shares of charred plant remains by site phase (Maeda et al. 2016; #55): the
paper's Table S5 counts, dated by Table S1's calibrated ranges and located."""

import io

import pyarrow.parquet as pq
import pytest

from oikos_data import cereals, region
from test_fauna import docx

BOX = region.Region(name="test-box", resolution=5, bbox=(31.0, 39.0, 33.0, 46.0), sha256="0" * 64)
SOURCES = [{"dataset": "test-cereals", "version": "1", "license": "CC-BY-4.0", "manifest_sha256": "1" * 64,
            "citation": "Test", "attribution": "Test data"}]
S1_HEADER = ["site", "lithic data", "cereal (%)", "cereal rachis", "14C date",
             "1-sigma calibrated summed probability of dates or estimated range from cultural material",
             "median age cal. BC", "number of dates", "reference"]
CHRONOLOGY = docx([S1_HEADER,
                   ["Netiv Hagdud", "", "yes", "", "yes", "9400-8750 BC", "9075", "8", "Kislev 1997"],
                   ["Körtik Tepe", "", "yes", "", "yes", "9680-9320 BC (main modal age only)", "9500", "5", "Riehl"],
                   ["Jarmo (PPN)", "", "yes", "", "", "estimated 7300 BC", "7300", "--", "authors"],
                   ["Jericho (PPNB)", "", "yes", "", "yes", "8200-7500 BC", "7850", "9", "Hopf"]])
S5_HEADER = ["site", "sickle bladegroup", "median agecal. BC", "no. samples", "Wheat", "Barley", "other cereals",
             "other crops", "other", "total NISP", "cereals(%)", "reference"]
SHARES = docx([S5_HEADER,
               ["Netiv Hagdud", "Group 1b", "9075", "8", "152", "4504", "0", "433", "11103", "16192", "28.8%", "Kislev"],
               ["Körtik Tepe", "Group 2 north", "9500", "12", "9", "7", "0", "18", "727", "761", "2.1%", "Riehl"],
               ["Jarmo  (PPN)", "Group 5 north", "7300", "62", "179", "7", "33", "166", "165", "550", "39.8%", "Fuller"],
               ["Jericho (PPNB)", "Group 3 south", "7850", "4", "5214", "1264", "0", "1016", "0", "7494", "86.4%", "Hopf"]])
TEST_SITES = {"Netiv Hagdud": (31.98, 35.38), "Körtik Tepe": (37.816, 40.988), "Jarmo": (35.56, 44.92)}


def table(chronology=CHRONOLOGY, **kw):
    args = dict(sites=TEST_SITES, overlap={"Jericho"}, left_out=set())
    args.update(kw)
    return pq.read_table(io.BytesIO(cereals.build(BOX, SOURCES, chronology, SHARES, **args))).to_pydict()


def test_phases_take_their_counts_and_table_s1s_calibrated_range():
    t = table()
    i = t["phase"].index("Netiv Hagdud")
    assert (t["start_year"][i], t["end_year"][i]) == (-9400, -8750)
    assert t["cereal"][i] == 152 + 4504 + 0 and t["items"][i] == 16192
    assert t["site"][i] == "Netiv Hagdud" and t["h3"][i] != 0
    assert t["reference"][i] == "Kislev", "the primary source, cited with the phase"


def test_a_single_estimated_date_spans_five_centuries_and_names_are_compared_loosely():
    t = table()
    i = t["phase"].index("Jarmo (PPN)")
    assert (t["start_year"][i], t["end_year"][i]) == (-7550, -7050)


def test_table_s1s_ranges_are_read_through_its_typing_slips():
    assert cereals.span("estimated 6450 B") == (-6700, -6200)
    assert cereals.span("8700-7500 BC (based on earlier modal age)") == (-8700, -7500)


def test_sites_the_origins_of_agriculture_database_records_are_left_out():
    # Their assemblages are judged already through that database (ADR-0064).
    assert "Jericho (PPNB)" not in table()["phase"]


def test_phases_are_ordered_by_start_and_out_of_region_ones_dropped():
    t = table()
    assert t["start_year"] == sorted(t["start_year"])
    assert "Jarmo (PPN)" not in table(sites=dict(TEST_SITES, Jarmo=(35.56, 47.5)))["phase"]


def test_an_undated_or_unlocated_phase_is_an_error():
    with pytest.raises(ValueError, match="Netiv Hagdud: not dated"):
        table(chronology=docx([S1_HEADER]))
    with pytest.raises(ValueError, match="Jarmo"):
        table(sites={k: v for k, v in TEST_SITES.items() if k != "Jarmo"})


def test_left_out_phases_need_neither_date_nor_place():
    t = table(sites={k: v for k, v in TEST_SITES.items() if k != "Jarmo"}, left_out={"Jarmo (PPN)"})
    assert "Jarmo (PPN)" not in t["phase"]


def test_every_overlapping_or_left_out_name_is_a_real_phase_or_site():
    # A misspelt entry would silently keep a phase the other record already judges.
    with pytest.raises(ValueError, match="Jerico"):
        table(overlap={"Jerico"})
