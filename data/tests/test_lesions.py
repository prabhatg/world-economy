"""The record of infectious lesions in the southern Levant (Eshed et al. 2010)."""

from oikos_data import lesions, region


def test_the_record_is_the_papers():
    periods, sites, source = lesions.load()
    assert [(p["name"], p["examined"], p["affected"]) for p in periods] == [("Natufian", 200, 2), ("Neolithic", 205, 9)]
    assert periods[0]["end_year"] == periods[1]["start_year"] == -8300
    assert source["doi"] == "10.1002/ajpa.21301"
    assert len(sites) == 14 and {s["period"] for s in sites} == {"Natufian", "Neolithic"}


def test_the_sites_lie_in_the_southern_levant_of_the_region():
    _, sites, _ = lesions.load()
    r = region.load("m1-swasia")
    s, n, w, e = r.bbox
    for site in sites:
        assert 31.5 <= site["lat"] <= 33.2 and 34.8 <= site["lng"] <= 35.7, site["name"]
        assert s <= site["lat"] <= n and w <= site["lng"] <= e
