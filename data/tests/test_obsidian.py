"""Obsidian: where each traced source group's volcanoes are (a forcing pack), and which
sources the sourced artefacts of dated site contexts came from (a Parquet table)."""

import io

import h3
import pyarrow.parquet as pq
import pytest

from oikos_data import obsidian, pack, region

BOX = region.Region(name="test-box", resolution=5, bbox=(36.0, 40.0, 33.0, 43.0), sha256="0" * 64)
SOURCES = [{"dataset": "test-obsidian", "version": "1", "license": "CC-BY-SA-4.0", "manifest_sha256": "1" * 64,
            "citation": "Test", "attribution": "Test data"}]
GROUPS = [{"name": "cappadocia", "region": "Cappadocia", "columns": ["Gollu Dag East", "Nenezi Dag"]},
          {"name": "eastern_anatolia", "region": "Van", "columns": ["Nemrut Dag / Bingol A"]}]
VOLCANOES = [{"Name": "Gollu Dag", "Longitude": "34.55", "Latitude": "38.259722", "Region": "Cappadocia"},
             {"Name": "Nemrut Dag", "Longitude": "42.23", "Latitude": "38.65", "Region": "Van"},
             {"Name": "Bingol A", "Longitude": "40.75", "Latitude": "38.933333", "Region": "Van"},
             {"Name": "Gegham", "Longitude": "45", "Latitude": "40.133333", "Region": "Transcaucasia"}]
CHRONOLOGY = {"0": (None, 12500), "1": (12500, 9500), "2": (10500, 8300), "3": (8300, 7600)}


def test_each_group_reaches_the_cells_around_its_volcanoes():
    p = pack.read(obsidian.build_pack(BOX, SOURCES, VOLCANOES, GROUPS))
    assert p.header["kind"] == "obsidian"
    assert [f["name"] for f in p.header["fields"]] == ["cappadocia", "eastern_anatolia"], "the groups, in order"
    cells = list(BOX.cells())
    for name, sites in (("cappadocia", [(38.259722, 34.55)]), ("eastern_anatolia", [(38.65, 42.23), (38.933333, 40.75)])):
        v = p.raw(name)
        reach = {h3.str_to_int(c) for lat, lng in sites for c in h3.grid_disk(h3.latlng_to_cell(lat, lng, 5), 1)}
        assert {int(cells[i]) for i in range(len(cells)) if v[i]} == reach & {int(c) for c in cells}, name
    assert p.header["sources"][0]["attribution"] == "Test data"


def test_a_group_without_volcanoes_in_the_table_is_an_error():
    with pytest.raises(ValueError, match="no volcanoes"):
        obsidian.build_pack(BOX, SOURCES, VOLCANOES, GROUPS + [{"name": "aegean", "region": "Aegean", "columns": []}])


def row(id, site, aspro, lat, lng, **counts):
    r = {"Site ID": id, "Site": site, "ASPRO": aspro, "Latitude": str(lat), "Longitude": str(lng),
         "Chemical Characterization": "Someone 2001", "Gollu Dag East": "0", "Nenezi Dag": "0",
         "Nemrut Dag / Bingol A": "0", "Gegham": "0", "Unknown": "0"}
    r.update({k: str(v) for k, v in counts.items()})
    return r


def table(rows):
    return pq.read_table(io.BytesIO(obsidian.build_table(BOX, SOURCES, rows, CHRONOLOGY, GROUPS))).to_pydict()


def test_contexts_count_artefacts_by_group_and_take_their_period_dates():
    t = table([row("1_1", "Kortik Tepe", "1", 37.81, 40.98, **{"Nemrut Dag / Bingol A": 20, "Gegham": 1}),
               row("2_1", "Asikli", "2 or 3", 38.35, 34.23, **{"Gollu Dag East": 30, "Nenezi Dag": 12}),
               row("3_1", "Jericho", "3", 31.87, 35.44, **{"Gollu Dag East": 5}),
               row("3_2", "Cafer", "x", 38.4, 38.8, **{"Gollu Dag East": 1})])
    assert t["context"] == ["1_1", "2_1"], "outside the region and undated contexts are dropped"
    assert t["cappadocia"] == [0, 42] and t["eastern_anatolia"] == [20, 0] and t["other"] == [1, 0]
    assert t["start_year"] == [-12500, -10500] and t["end_year"] == [-9500, -7600], "'a or b' spans both periods"
    assert t["period"] == ["1", "2 or 3"]
    assert t["cell"][1] == list(BOX.cells()).index(t["h3"][1])
    assert t["source"][0] == "Someone 2001"


def test_contexts_without_a_dated_period_are_dropped_and_unknown_periods_are_errors():
    t = table([row("0_1", "Zarzi", "0", 37.0, 40.0, **{"Nemrut Dag / Bingol A": 1})])
    assert t["context"] == [], "period 0 has no start"
    with pytest.raises(ValueError, match="period"):
        table([row("9_1", "Somewhere", "17", 37.0, 40.0)])
