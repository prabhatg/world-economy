"""The M1 calibration problem on the real records (runs when p3k14c, IntCal20, the rachis
supplement, the hand-downloaded Origins of agriculture tables and the M1 packs are
cached and the bindings are built)."""

import numpy as np
import pytest

from oikos_cal import history
from oikos_data import raster


@pytest.fixture(scope="module")
def problem():
    pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    from oikos_cal import m1

    try:
        m1.packs()
        return m1.Problem()
    except (raster.MissingInputs, FileNotFoundError) as e:
        pytest.skip(str(e))


def test_the_record_summaries(problem):
    ts = {t.name: t for t in problem.targets}
    rc = [t for t in problem.targets if t.name.startswith("radiocarbon") and t.name.endswith("BCE")]
    assert len(rc) == 10 and rc[0].observed < rc[4].observed, "dates rise into the Neolithic"
    g = problem.groups[0]
    assert g.indices == list(range(10)) and g.dof == 9
    assert np.allclose(np.diag(g.covariance), [t.variance for t in rc])
    assert all(0 < t.variance < 0.3 for t in rc)
    assert ts["domestic barley before 9000 BCE"].observed < 0.05 < 0.7 < ts["domestic barley after 7000 BCE"].observed
    assert problem.names[: len(problem.targets)] == list(ts)
    st = [t.name for t in problem.targets if t.name.startswith("sites")]
    assert len(st) == 14 and st[0] == "sites anatolia 6500-6000 BCE"
    assert [g.indices for g in problem.groups[1:3]] == [list(range(14, 21)), list(range(21, 28))]
    assert all(g.dof == 6 for g in problem.groups[1:3])
    assert (ts["sites upper_mesopotamia 3500-3000 BCE"].observed > ts["sites upper_mesopotamia 6500-6000 BCE"].observed
            + 0.3), "villages multiply into the Late Chalcolithic"
    ob = [t for t in problem.targets if t.name.startswith("obsidian")]
    assert len(ob) == 8 and problem.groups[3].indices == list(range(28, 36)) and problem.groups[3].dof == 8
    assert ob[0].observed < 0.1 and ob[3].observed > 0.9, "before 6900 BCE each source supplies its own side"
    assert 0.2 < ob[1].observed < 0.8 or 0.2 < ob[2].observed < 0.8, "and the two mix near the middle"
    assert problem.obsidian.isolated == 5, "the Cypriot contexts: obsidian crossed the sea by boat"
    on = [t for t in problem.targets if t.name.startswith("cultivation effort implied")]
    assert len(on) == 4 and problem.groups[4].indices == list(range(36, 40)) and problem.groups[4].dof == 4
    assert 0.3 < problem.onset.farming_share < 0.7, "Pottery Neolithic phases anchor a farming world"
    assert on[0].observed < 0.2 < 0.5 < on[3].observed, "little planting before 10,000 BCE, much by the late PPNB"
    vs = [t for t in problem.targets if t.name.startswith("village size")]
    assert len(vs) == 14 and vs[0].name == "village size anatolia 6500-6000 BCE"
    assert problem.groups[5].indices == list(range(40, 54)) and problem.groups[5].dof == 14
    assert all(np.log(50) < t.observed < np.log(1000) for t in vs), "villages of a few hundred people"
    assert all(0.2 <= t.variance < 0.4 for t in vs), "one density converts them all; sites add a little"
    up = [t.observed for t in vs if "upper_mesopotamia" in t.name]
    assert up[-1] > up[0] + 0.2, "Upper Mesopotamian sites grow into the Late Chalcolithic"
    sr = [t for t in problem.targets if t.name.endswith("% by")]
    assert len(sr) == 12 and sr[0].name == "radiocarbon southern Levant 10% by"
    assert problem.groups[6].indices == list(range(54, 66)) and problem.groups[6].dof == 12
    by = {t.name: t.observed for t in sr + problem.unjudged.targets}
    for late in ("Anatolia", "northeast Africa"):
        for q in (10, 25, 50):
            assert by[f"radiocarbon {late} {q}% by"] > by[f"radiocarbon southern Levant {q}% by"] + 300, \
                "dated activity rose in the Levant before Anatolia and Africa"
    assert all(50**2 < t.variance < 1000**2 for t in sr)
    assert all(len(m) >= 3 for m in problem.subregions.members), "each subregion spans several blocks"
    oc = [t for t in problem.targets if "rich minus poor" in t.name]
    assert [t.name for t in oc] == ["cultivation effort rich minus poor stands before 7600 BCE",
                                    "cultivation effort rich minus poor stands after 7600 BCE"]
    assert problem.groups[7].indices == [66, 67] and problem.groups[7].dof == 2
    assert oc[1].observed > 0.3, "by the late PPNB the richest stands were farmed far more"
    ss = [t for t in problem.targets if t.name.startswith("sheep size")]
    assert [t.name for t in ss][0] == "sheep size (LSI) 12000-8500 BCE" and len(ss) == 6
    assert ss[0].observed > 0.04 and all(t.observed < -0.03 for t in ss[3:]), "wild, then domestic sizes"
    first = problem.names.index(ss[0].name)
    assert problem.groups[-1].indices == list(range(first, first + 6)) and problem.groups[-1].dof == 6
    assert problem.names[-7:] == ["log-likelihood radiocarbon", "log-likelihood domestication",
                                  "log-likelihood obsidian", "people 3000 BCE",
                                  *(f"radiocarbon northeast Africa {q}% by, not judged" for q in (10, 25, 50))]


def test_worlds_are_run_in_parallel_and_reproducibly(problem):
    from oikos_cal import m1

    a, b = m1.evaluate([({}, 1), ({}, 1)], workers=2)
    assert len(a) == len(problem.names) and np.isfinite(a).all()
    assert (a == b).all(), "a run is a function of its parameters and seed"
    i = history.implausibility(a[None, : len(problem.targets)], 0.0, problem.targets)[0]
    print("M1 default world implausibility: " + ", ".join(f"{t.name} {x:.1f}" for t, x in zip(problem.targets, i)))


def test_the_default_world_is_consistent_with_the_record(problem):
    """The registry defaults are a calibrated M1 world (calibration/runs/m1/round10/best-wave6.json,
    ADR-0081): with any seed, every target and every group pass history matching."""
    from oikos_cal import __main__ as cli
    from oikos_cal import m1

    out = m1.evaluate([({}, s) for s in (1, 2, 3)], workers=3)
    doc = {"targets": cli._targets_json(problem.targets), "groups": cli._groups_json(problem.groups), "outputs": out}
    i = history.implausibility(out[:, : len(problem.targets)], 0.0, problem.targets)
    assert cli.consistent(doc).all(), f"max implausibility per seed {i.max(axis=1)}"


def test_a_round_can_leave_out_the_house_size_record(problem):
    from oikos_cal import m1

    without = m1.Problem(leave_out=("inequality",))
    kept = [t.name for t in problem.targets if not t.name.startswith("house Gini")]
    assert len(kept) < len(problem.targets), "the full problem judges house Ginis"
    assert [t.name for t in without.targets] == kept and without.inequality is None
    assert [g.indices for g in without.groups] == [g.indices for g in problem.groups]
    assert without.names == kept + m1.DIAGNOSTICS
    with pytest.raises(ValueError, match="radiocarbon"):
        m1.Problem(leave_out=("radiocarbon",))


def test_the_record_is_split_by_the_engines_wild_cereal_suitability(problem):
    """The stands the onset contrasts split phases by are the engine's: the same formula,
    climate and parameters (#36)."""
    import oikos

    from oikos_cal import m1, onset
    from oikos_data import DATA, pack

    w = oikos.World.from_packs(m1.packs(), seed=1, params={})
    climate = pack.read((DATA / "build" / "m1-swasia.climate.oikospack").read_bytes())
    assert climate.header["time"]["calendar_year"][:2] == [-12050, -11950] and w.year == -12000
    mid = {k: (climate.values(k)[0] + climate.values(k)[1]) / 2 for k in ("temp_mean", "precip")}
    params = {k: oikos.registry()["params"][k]["default"] for k in onset.CEREAL}
    assert np.allclose(onset.cereal_suitability(mid["temp_mean"], mid["precip"], params), w.suitability())
    assert 0.0 <= problem.stands.min() and problem.stands.max() <= 1.0
