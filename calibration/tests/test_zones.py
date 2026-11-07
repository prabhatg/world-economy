"""The reduced calibration record (#47): zone sums score as the full record does."""

import json

import numpy as np
import pytest

from oikos_cal import record, zones
from oikos_cal.radiocarbon import Window

WINDOW = Window(-1000, -500)  # five centuries
CELLS = np.arange(600_000_000_000_000_000, 600_000_000_000_000_000 + 8, dtype=np.uint64)


def _activity(rng):
    arrays = {name: rng.uniform(0, 10, (len(CELLS), WINDOW.centuries)) for name in record.FIELDS}
    obsidian = {"a": rng.uniform(0, 1, (len(CELLS), WINDOW.centuries))}
    return record.Activity(WINDOW, obsidian=obsidian, **arrays)


def _zones(weights=None):
    sets = [np.array([0, 1, 2]), np.array([5, 3]), np.arange(len(CELLS))]
    return zones.Zones(CELLS, WINDOW, sets, weighted=[np.array([6, 7]), np.array([0, 1, 2])], weights=weights)


def test_zone_sums_answer_the_reads_operators_make():
    rng = np.random.default_rng(1)
    full = _activity(rng)
    reduced = _zones().reduce(full)
    assert reduced.window == WINDOW
    for name in record.FIELDS:
        f, z = getattr(full, name), getattr(reduced, name)
        np.testing.assert_allclose(z[np.array([0, 1, 2])].sum(axis=0), f[[0, 1, 2]].sum(axis=0), rtol=1e-14)
        np.testing.assert_allclose(z[np.array([3, 5])].sum(axis=0), f[[3, 5]].sum(axis=0), rtol=1e-14)
        np.testing.assert_allclose(z[np.array([3, 5]), 1:3].sum(), f[[3, 5], 1:3].sum(), rtol=1e-14)
        mask = np.array([True, False, True, True, False])
        np.testing.assert_allclose(z[np.ix_(np.array([5, 3]), mask)].sum(), f[np.ix_([3, 5], mask)].sum(), rtol=1e-14)
        np.testing.assert_allclose(z[:, -1].sum(), f[:, -1].sum(), rtol=1e-14)
    np.testing.assert_allclose(reduced.obsidian["a"][np.array([0, 1, 2])].sum(axis=0),
                               full.obsidian["a"][:3].sum(axis=0), rtol=1e-14)
    visible = reduced.visible(0.5)
    np.testing.assert_allclose(visible[np.array([5, 3])].sum(axis=0), full.visible(0.5)[[3, 5]].sum(axis=0), rtol=1e-14)
    with pytest.raises(KeyError, match="not a zone"):
        reduced.people[np.array([0, 1])]


def test_weighted_zones_count_each_cell_by_its_weight_in_each_century():
    rng = np.random.default_rng(2)
    full = _activity(rng)
    weights = np.ones((len(CELLS), WINDOW.centuries))
    weights[1] = [0.0, 0.25, 0.5, 0.75, 1.0]
    weights[7] = 0.5
    reduced = _zones(weights).reduce(full)
    got = reduced.people.weighted([np.array([0, 1, 2]), np.array([6, 7])])
    want = np.stack([(full.people * weights)[[0, 1, 2]].sum(axis=0), (full.people * weights)[[6, 7]].sum(axis=0)])
    np.testing.assert_allclose(got, want, rtol=1e-14)
    np.testing.assert_allclose(reduced.people[np.array([0, 1, 2])].sum(axis=0), full.people[:3].sum(axis=0),
                               rtol=1e-14)  # the same cells unweighted are their own zone


def test_zones_round_trip_through_their_file_exactly():
    rng = np.random.default_rng(3)
    weights = np.ones((len(CELLS), WINDOW.centuries))
    weights[2] = rng.uniform(0, 1, WINDOW.centuries)
    weights[4] = weights[2]  # equal rows are stored once
    z = _zones(weights)
    doc = json.loads(z.encode())
    assert doc["format"] == zones.FORMAT and doc["centuries"] == WINDOW.centuries
    assert doc["weights"]["cells"] == [2, 4] and doc["weights"]["row"] == [0, 0] and len(doc["weights"]["rows"]) == 1
    back = zones.Zones.decode(z.encode(), CELLS)
    assert back.encode() == z.encode() and back.id == z.id
    np.testing.assert_array_equal(back.weights, weights)
    with pytest.raises(ValueError, match="cells"):
        zones.Zones.decode(z.encode(), CELLS[:-1])


def test_zones_refuse_cells_outside_the_world():
    with pytest.raises(ValueError, match="outside"):
        zones.Zones(CELLS, WINDOW, [np.array([0, 8])])
    with pytest.raises(ValueError, match="weights"):
        zones.Zones(CELLS, WINDOW, [], weighted=[np.array([0])], weights=np.ones((len(CELLS), 4)))


def test_reads_zone_sums_cannot_answer_raise_instead_of_answering_wrongly():
    full = _activity(np.random.default_rng(4))
    z = _zones().reduce(full).people
    with pytest.raises(KeyError):
        z[np.array([0, 1, 2]), np.array([0, 1, 2])]  # paired indexing: one value per cell
    with pytest.raises(KeyError, match="mask"):
        z[np.array([True, True, False])]  # a mask the size of no world
    mask = np.zeros(len(CELLS), dtype=bool)
    mask[[0, 1, 2]] = True
    np.testing.assert_allclose(z[mask].sum(axis=0), full.people[mask].sum(axis=0), rtol=1e-14)
    with pytest.raises(KeyError):
        z[np.array([0.0, 1.0, 2.0])]
    np.testing.assert_allclose(z[:, [0, 2]].sum(), full.people[:, [0, 2]].sum(), rtol=1e-14)
    with pytest.raises(TypeError):
        z + 1.0  # a constant per cell, not per zone
    with pytest.raises(TypeError):
        z * z


def test_radiocarbon_refuses_zones_weighted_by_another_visibility():
    from oikos_cal import radiocarbon

    weights = np.ones((len(CELLS), WINDOW.centuries))
    weights[0] = 0.5
    reduced = _zones(weights).reduce(_activity(np.random.default_rng(5)))
    with pytest.raises(ValueError, match="visibility"):
        radiocarbon.check_weights(reduced.people, np.ones_like(weights))
    radiocarbon.check_weights(reduced.people, weights)


def test_a_decoded_zone_file_keeps_its_bytes_and_refuses_malformed_ones():
    weights = np.ones((len(CELLS), WINDOW.centuries))
    weights[3] = 0.25
    z = _zones(weights)
    pretty = json.dumps(json.loads(z.encode()), indent=1).encode()
    assert zones.Zones.decode(pretty, CELLS).encode() == pretty, "the id is the hash of the file as given"

    def bad(change, match):
        doc = json.loads(z.encode())
        change(doc)
        with pytest.raises(ValueError, match=match):
            zones.Zones.decode(json.dumps(doc).encode(), CELLS)

    bad(lambda d: d["weights"]["rows"][0].pop(), "centuries")
    bad(lambda d: d.update(cells=7), "cells")
    bad(lambda d: d.update(weighted=d["weighted"] + [99]), "weighted")
    bad(lambda d: d["weights"].update(row=[5]), "row")
    bad(lambda d: d["zones"][0].reverse(), "ascending")
