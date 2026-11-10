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
