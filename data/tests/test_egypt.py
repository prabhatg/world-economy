"""Domestic animals in Egypt's Neolithic assemblages (Linseele et al. 2014; #51)."""

import h3

from oikos_data import egypt, region


def test_the_record_is_the_papers():
    assemblages, source = egypt.load()
    assert source["doi"] == "10.1371/journal.pone.0108517"
    by = {a["name"]: a for a in assemblages}
    assert len(by) == 15
    assert (by["Sodmein"]["mammals"], by["Sodmein"]["domestic"]) == (167, 111)
    assert (by["Merimde I"]["start_year"], by["Merimde I"]["end_year"]) == (-4900, -4700)
    # Every assemblage of the 6th and 5th millennia BC holds domestic animals.
    assert all(0 < a["domestic"] <= a["mammals"] for a in assemblages)
    assert sum(a["end_year"] <= -5000 for a in assemblages) == 5
    assert min(a["start_year"] for a in assemblages) == -6200


def test_the_assemblages_lie_on_the_regions_african_cells():
    assemblages, _ = egypt.load()
    r = region.load("m1-swasia")
    cells = set(int(c) for c in r.cells())
    for a in assemblages:
        assert 25 <= a["lat"] <= 31.5 and 27 <= a["lng"] <= 34.1, a["name"]
        assert h3.str_to_int(h3.latlng_to_cell(a["lat"], a["lng"], r.resolution)) in cells, a["name"]


def test_crops_are_the_articles_statements():
    # Linseele et al. (2014): no cultivated crops in Egypt in the 6th millennium BC, none
    # grown in the deserts (Kharga's grains came from the Nile valley), and crops from
    # Southwest Asia in the Nile valley, the delta and the Fayum in the 5th.
    assemblages, _ = egypt.load()
    crops = {a["name"]: a.get("crops") for a in assemblages}
    assert all(crops[a["name"]] is False for a in assemblages if a["end_year"] <= -5000)
    assert crops["KS 43, Kharga"] is False
    assert all(crops[n] for n in ("Merimde I", "Merimde II-V", "Kom K", "Kom W", "El-Omari",
                                  "Saïs I (Middle to Late Neolithic)", "Saïs II (Late Neolithic)"))
    assert crops["Qasr el-Sagha VII A/81"] is None and crops["Maghar Dendera 2"] is None, "the article says nothing"
