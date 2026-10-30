"""Tests for the Oikos Python bindings (crates/oikos-py).

The bindings are thin wrappers over oikos-sim, so these tests check that Python sees
exactly what the CLI and work units produce: the golden hash, partition-independent
results, faithful checkpoints and forks, and registry-checked parameters.

Build and run (from engine/): `maturin develop --release -m crates/oikos-py/Cargo.toml`
then `pytest python/tests`, or just `scripts/check.sh`.
"""

import json
import tomllib
from pathlib import Path

import numpy as np
import pytest

import oikos

ENGINE = Path(__file__).resolve().parents[2]
GOLDEN_WU = ENGINE / "golden" / "m0-toy.workunit.json"
GOLDEN = json.loads((ENGINE / "golden" / "m0-toy.json").read_text())
GOLDEN_HASH = "362621da7610ffcca4bb4c944abc3508d57afe9a38e920647bb6b05db50d4f66"


def run(world, years=300, parts=1):
    world.run_years(years, parts=parts)
    return world.state_hash()


def test_work_unit_reproduces_the_golden_result():
    result = oikos.run_work_unit(GOLDEN_WU.read_text(), read_dir=str(ENGINE))
    assert result["state_hash"] == GOLDEN_HASH
    drop_engine = lambda r: {k: v for k, v in r.items() if k != "engine"}
    assert drop_engine(result) == drop_engine(GOLDEN)


def test_world_run_years_matches_the_cli():
    wu = json.loads(GOLDEN_WU.read_text())
    w = oikos.World(wu["world"]["base_cell"], wu["world"]["resolution"], seed=wu["seed"])
    summaries = w.run_years(wu["years"], parts=wu["partitions"], summary_every=wu["summary_every_years"])
    assert w.state_hash() == GOLDEN_HASH
    assert w.tick == GOLDEN["final_tick"]
    assert summaries == GOLDEN["summaries"]


def test_same_seed_gives_identical_worlds_for_any_partition_count():
    a, b, c = (oikos.World(seed=11) for _ in range(3))
    assert run(a, parts=1) == run(b, parts=1) == run(c, parts=5)
    assert a.checkpoint() == c.checkpoint()


def test_different_seeds_diverge():
    assert run(oikos.World(seed=1)) != run(oikos.World(seed=2))


def test_step_advances_one_period_and_reports_exact_stats():
    w = oikos.World(seed=3)
    before = w.summary()["population"]
    stats = w.step()
    assert set(stats) == {"births", "deaths", "fissions", "migrants"}
    assert w.tick == 365 and w.year == -11_999, "calendar years count from 12,000 BCE"
    assert w.summary()["population"] == before + stats["births"] - stats["deaths"]


def test_checkpoint_round_trip_continues_identically():
    w = oikos.World(seed=5)
    w.run_years(200)
    data = w.checkpoint()
    assert isinstance(data, bytes)
    restored = oikos.World.from_checkpoint(data)
    assert restored.state_hash() == w.state_hash()
    assert restored.tick == w.tick and restored.seed == w.seed and restored.params == w.params
    assert run(restored) == run(w)


def test_fork_with_the_same_seed_continues_the_same_history():
    w = oikos.World(seed=5)
    w.run_years(200)
    twin = w.fork(seed=5)
    assert run(twin) == run(w)


def same_agents(a, b):
    return all(np.array_equal(x, y) for x, y in zip(a.agents().values(), b.agents().values()))


def test_fork_with_a_new_seed_branches_history():
    w = oikos.World(seed=5)
    w.run_years(200)
    a, b = w.fork(seed=6), w.fork(seed=7)
    assert a.seed == 6 and a.tick == w.tick and same_agents(a, w) and same_agents(b, w)
    a.run_years(300)
    b.run_years(300)
    assert not same_agents(a, b)


def test_fork_with_new_params_applies_them():
    w = oikos.World(seed=5)
    w.run_years(200)
    r = w.params["growth_rate"]
    f = w.fork(seed=5, params={"growth_rate": 2 * r})
    assert f.params["growth_rate"] == 2 * r and w.params["growth_rate"] == r
    assert same_agents(f, w)
    f.run_years(300)
    w.run_years(300)
    assert f.summary()["population"] > w.summary()["population"]


def test_population_and_agents_agree_with_the_summary():
    w = oikos.World(seed=9)
    final = w.run_years(500, parts=3, summary_every=100)[-1]
    assert final == w.summary()
    pop = w.population()
    assert pop.dtype == np.uint64 and pop.shape == (w.cells,)
    assert int(pop.sum()) == final["population"]
    assert int(np.count_nonzero(pop)) == final["occupied_cells"]
    agents = w.agents()
    assert agents["id"].dtype == np.uint64 and agents["cell"].dtype == np.uint32
    assert agents["weight"].dtype == np.uint64
    assert len(agents["id"]) == final["agents"]
    assert int(agents["weight"].sum()) == final["population"]
    np.testing.assert_array_equal(np.bincount(agents["cell"], weights=agents["weight"], minlength=w.cells), pop)
    order = np.lexsort((agents["id"], agents["cell"]))
    np.testing.assert_array_equal(order, np.arange(len(order)))


def test_arrays_are_copies_the_caller_may_modify():
    w = oikos.World(seed=9)
    pop = w.population()
    pop[:] = 0
    assert w.population().sum() > 0


def test_params_default_to_the_registry_and_accept_overrides():
    defaults = {k: d["default"] for k, d in oikos.registry()["params"].items()}
    assert oikos.World(seed=1).params == defaults
    w = oikos.World(seed=1, params={"growth_rate": 0.02, "initial_bands": 8})
    assert w.params == {**defaults, "growth_rate": 0.02, "initial_bands": 8}
    assert w.summary()["agents"] == 8


@pytest.mark.parametrize(
    "params, message",
    [
        ({"growth_rate": 0.5}, "growth_rate"),
        ({"fission_probability": -0.1}, "fission_probability"),
        ({"initial_bands": 2.5}, "initial_bands"),
        ({"initial_bands": -1}, "initial_bands"),
        ({"initial_bands": True}, "initial_bands"),
        ({"ghost": 1}, "unknown parameter ghost"),
        ({"capacity_min": 500, "capacity_max": 100}, "capacity_min"),
    ],
)
def test_invalid_params_raise_value_error_naming_the_key(params, message):
    with pytest.raises(ValueError, match=message):
        oikos.World(seed=1, params=params)
    with pytest.raises(ValueError, match=message):
        oikos.World(seed=1).fork(seed=2, params=params)


def test_engine_errors_become_value_errors_with_the_engine_message():
    with pytest.raises(ValueError, match="not an Oikos checkpoint"):
        oikos.World.from_checkpoint(b"NOT-OIKOS checkpoint")
    with pytest.raises(ValueError, match="checkpoint truncated"):
        oikos.World.from_checkpoint(oikos.World(seed=1).checkpoint()[:-3])
    with pytest.raises(ValueError, match="unsupported work unit format 9"):
        oikos.run_work_unit(GOLDEN_WU.read_text().replace('"format": 1', '"format": 9'))
    with pytest.raises(ValueError, match="work unit"):
        oikos.run_work_unit("{not json")


def test_work_unit_files_go_to_write_dir_and_checkpoints_come_from_read_dir(tmp_path):
    wu = json.loads((ENGINE / "examples" / "outputs.workunit.json").read_text())
    wu.update(years=200, checkpoint_out="state.ckpt", outputs={**wu["outputs"], "dir": "out"})
    first = oikos.run_work_unit(json.dumps(wu), write_dir=str(tmp_path))
    assert first["files"] and all((tmp_path / f).is_file() for f in first["files"])
    restored = oikos.World.from_checkpoint((tmp_path / "state.ckpt").read_bytes())
    assert restored.state_hash() == first["state_hash"]

    resume = {**wu, "start": {"kind": "checkpoint", "path": "state.ckpt"}, "outputs": None}
    resume.pop("checkpoint_out")
    second = oikos.run_work_unit(json.dumps(resume), read_dir=str(tmp_path), write_dir=str(tmp_path))
    assert second["final_tick"] == 2 * first["final_tick"]
    assert second["state_hash"] == run(restored, years=200, parts=wu["partitions"])


def test_registry_is_the_embedded_parameter_file():
    reg = oikos.registry()
    toml = tomllib.loads((ENGINE / "params" / "m0-toy.toml").read_text())
    assert reg["name"] == toml["name"] and reg["version"] == toml["version"]
    assert reg["params"].keys() == toml["params"].keys()
    for key, d in toml["params"].items():
        assert reg["params"][key]["unit"] == d["unit"]
        assert reg["params"][key]["default"] == d["default"]


def free_params():
    params = oikos.registry()["params"]
    return {k: d for k, d in params.items() if d["prior"]["dist"] in ("uniform", "log_uniform")}


def test_sample_params_respects_prior_bounds_and_kinds():
    free = free_params()
    assert {"growth_rate", "fission_probability", "fission_threshold"} <= free.keys()
    for u in (0.0, 0.1, 0.5, 0.9, 1.0):
        sampled = oikos.sample_params({k: u for k in free})
        assert sampled.keys() == free.keys()
        for key, value in sampled.items():
            prior, kind = free[key]["prior"], free[key]["kind"]["type"]
            assert prior["lo"] <= value <= prior["hi"], (key, u, value)
            assert isinstance(value, int if kind == "integer" else float), (key, value)
        oikos.World(seed=1, params=sampled)
    lo, hi = oikos.sample_params({"growth_rate": 0.0}), oikos.sample_params({"growth_rate": 1.0})
    assert lo["growth_rate"] == free["growth_rate"]["prior"]["lo"]
    assert hi["growth_rate"] == free["growth_rate"]["prior"]["hi"]


def test_sample_params_is_deterministic_and_monotone():
    us = np.linspace(0.0, 1.0, 41)
    values = [oikos.sample_params({"growth_rate": float(u)})["growth_rate"] for u in us]
    assert values == [oikos.sample_params({"growth_rate": float(u)})["growth_rate"] for u in us]
    assert values == sorted(values)


@pytest.mark.parametrize(
    "u, message",
    [
        ({"capacity_min": 0.5}, "capacity_min.*fixed"),
        ({"growth_rate": 1.5}, r"growth_rate.*\[0, 1\]"),
        ({"growth_rate": float("nan")}, "growth_rate"),
        ({"growth_rate": True}, "growth_rate"),
        ({"ghost": 0.5}, "unknown parameter ghost"),
    ],
)
def test_sample_params_rejects_what_it_cannot_map(u, message):
    with pytest.raises(ValueError, match=message):
        oikos.sample_params(u)


def test_capacity_and_habitat():
    w = oikos.World(seed=3)
    cap, hab = w.capacity(), w.habitable()
    assert cap.dtype == np.uint64 and len(cap) == w.cells and hab.dtype == np.bool_ and len(hab) == w.cells
    p = oikos.registry()["params"]
    assert cap.min() >= p["capacity_min"]["default"] and cap.max() <= p["capacity_max"]["default"], "toy capacity"
    assert hab.all(), "without forcing packs every cell is habitable"


def test_cultivation_fields():
    w = oikos.World(seed=3)
    w.run_years(300, parts=2)
    for name in ("cultivation", "domestication", "suitability", "climate_stress", "climate_variability"):
        v = getattr(w, name)()
        assert v.dtype == np.float64 and v.shape == (w.cells,), name
        assert not v.any(), f"{name}: toy worlds never cultivate"
    np.testing.assert_array_equal(w.forage_capacity(), w.capacity())
    assert (w.soil() == 1).all(), "nobody farms, so the soil stays whole"
    agents = w.agents()
    for name in ("cultivation", "domestication"):
        assert agents[name].dtype == np.float64 and len(agents[name]) == len(agents["id"])
        assert not agents[name].any()
