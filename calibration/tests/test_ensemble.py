"""Ensembles of plausible worlds for the viewer (ADR-0063): which worlds, the work units
that run them, and the series a viewer charts."""

import numpy as np
import pytest

from oikos_cal import ensemble


def _best(n):
    cands = [{"wave": 4, "run": 10 + i, "params": {"growth_rate": 0.01 + i / 1000}, "consistent": i != 1,
              "rerun_log_likelihood": -100.0 - i} for i in range(n)]
    return {"command": "best", "waves": ["runs/m1/round5/wave5.json"], "candidates": cands}


def test_the_defaults_come_first_then_consistent_candidates_best_first():
    worlds = ensemble.choose(_best(5), k=3)
    assert [w["label"] for w in worlds] == ["Calibrated defaults", "Wave 5 run 10", "Wave 5 run 12",
                                            "Wave 5 run 13"], "run 11 is inconsistent"
    assert worlds[0]["params"] == {} and worlds[1]["params"] == {"growth_rate": 0.01}
    assert [w["id"] for w in worlds] == ["w0", "w1", "w2", "w3"]
    assert len(ensemble.choose(_best(2), k=5)) == 2, "as many as there are"


def test_a_world_runs_as_an_ordinary_work_unit_with_outputs():
    packs = [{"kind": "geometry", "id": "g" * 64, "path": "/d/m1-swasia.geometry.oikospack"},
             {"kind": "climate", "id": "c" * 64, "path": "/d/m1-swasia.climate.oikospack"}]
    wu = ensemble.work_unit(packs, {"growth_rate": 0.02}, seed=3, out="viewer/w1", years=9000, every=100)
    assert wu["world"] == {"geometry": "g" * 64}
    assert wu["inputs"] == [{"id": p["id"], "path": p["path"]} for p in packs]
    assert wu["params"] == {"growth_rate": 0.02} and wu["seed"] == 3 and wu["years"] == 9000
    assert wu["start"] == {"kind": "initial"} and wu["outputs"]["dir"] == "viewer/w1"
    assert wu["outputs"]["every_years"] == 100
    with pytest.raises(ValueError, match="geometry"):
        ensemble.work_unit(packs[1:], {}, seed=1, out="x", years=100, every=100)


def test_series_are_people_and_the_shares_farming_and_domestic():
    population = np.array([[10, 0, 30], [20, 20, 0]], dtype=np.uint64)
    cultivation = np.array([[0, 0, 5_000], [10_000, 0, 0]], dtype=np.uint64)  # parts per 10,000
    domestication = np.array([[0, 0, 2_000], [5_000, 0, 0]], dtype=np.uint64)
    s = ensemble.series({"population": population, "cultivation": cultivation, "domestication": domestication})
    assert s["people"] == [40, 40]
    assert s["farming"] == pytest.approx([15 / 40, 20 / 40]), "people-weighted cultivation effort"
    assert s["domestic"] == pytest.approx([0.2, 0.5]), "effort-weighted domestic share"
    empty = ensemble.series({k: np.zeros((1, 3), dtype=np.uint64) for k in ("population", "cultivation",
                                                                            "domestication")})
    assert empty == {"people": [0], "farming": [0.0], "domestic": [0.0]}
