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
    assert len(sr) == 15 and sr[0].name == "radiocarbon southern Levant 10% by"
    assert sr[-1].name == "radiocarbon northeast Africa 50% by", "judged since the dates out of sight are (#44)"
    assert problem.groups[6].indices == list(range(54, 69)) and problem.groups[6].dof == 15
    by = {t.name: t.observed for t in sr}
    for late in ("Anatolia", "northeast Africa"):
        for q in (10, 25, 50):
            assert by[f"radiocarbon {late} {q}% by"] > by[f"radiocarbon southern Levant {q}% by"] + 300, \
                "dated activity rose in the Levant before Anatolia and Africa"
    assert all(50**2 < t.variance < 1000**2 for t in sr)
    assert all(len(m) >= 3 for m in problem.subregions.members), "each subregion spans several blocks"
    oc = [t for t in problem.targets if "rich minus poor" in t.name]
    assert [t.name for t in oc] == ["cultivation effort rich minus poor stands before 7600 BCE",
                                    "cultivation effort rich minus poor stands after 7600 BCE"]
    assert problem.groups[7].indices == [69, 70] and problem.groups[7].dof == 2
    assert oc[1].observed > 0.3, "by the late PPNB the richest stands were farmed far more"
    ss = [t for t in problem.targets if t.name.startswith("sheep size")]
    assert [t.name for t in ss][0] == "sheep size (LSI) 12000-8500 BCE" and len(ss) == 6
    assert ss[0].observed > 0.04 and all(t.observed < -0.03 for t in ss[3:]), "wild, then domestic sizes"
    first = problem.names.index(ss[0].name)
    assert problem.groups[-1].indices == list(range(first, first + 6)) and problem.groups[-1].dof == 6
    assert problem.names[-4:] == ["log-likelihood radiocarbon", "log-likelihood domestication",
                                  "log-likelihood obsidian", "people 3000 BCE"]


def test_the_wheats_leave_out_the_villages_the_model_cannot_farm(problem):
    from oikos_cal import m1

    # El Kowm 2 alone would be einkorn's era after 7000 BCE; an oasis the model cannot farm (#48)
    assert "El Kowm 2" in m1.OASES
    eras = [t.name for t in problem.targets if t.name.startswith("domestic einkorn")]
    assert eras == ["domestic einkorn before 9000 BCE", "domestic einkorn 9000-8000 BCE",
                    "domestic einkorn after 8000 BCE"]
    assert len([t for t in problem.targets if t.name.startswith("domestic emmer")]) == 4
    assert len(problem.wheats[1].n) == 7 and problem.wheats[1].crop == "einkorn"
    assert len([t for t in problem.targets if t.name.startswith("domestic barley")]) == 4, "barley's eras keep it"


def test_every_crop_counts_the_wild_cereal_its_foragers_gather(problem):
    from oikos_cal import m1

    # ADR-0103: calibrated against the rachis assemblages at the defaults, three seeds
    assert 0 < m1.GATHERED < 1
    assert [op.gathered for op in (problem.domestication, *problem.wheats)] == [m1.GATHERED] * 3


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


def test_a_reduced_record_scores_as_the_full_one(problem):
    from oikos_cal import obsidian, record

    rng = np.random.default_rng(0)
    shape = (len(problem.cells), problem.radiocarbon.settings.true.centuries)
    arrays = {name: rng.uniform(0, 100, shape) for name in record.FIELDS}
    arrays["domestic"] = arrays["cultivators"] * rng.uniform(0, 1, shape)
    arrays["herd_domestic"] = arrays["herders"] * rng.uniform(0, 1, shape)
    for wheat in ("emmer_domestic", "einkorn_domestic"):
        arrays[wheat] = arrays["cultivators"] * rng.uniform(0, 1, shape)
    full = record.Activity(problem.radiocarbon.settings.true, obsidian={g: rng.uniform(0, 100, shape)
                                                                        for g in obsidian.GROUPS}, **arrays)
    z = problem.zones()
    np.testing.assert_allclose(problem.outputs(z.reduce(full)), problem.outputs(full), rtol=1e-9)
    assert len(z.zones) < 400 and len(z.encode()) < 4e6, "a zone file small enough to fetch once per experiment"


def test_the_wheats_are_gathered_wild_only_within_their_ancestors_ranges(problem):
    # Wild emmer and einkorn ranges (#38): none on the central Anatolian plateau for emmer,
    # so Aşıklı Höyük's emmer reads as sown; barley's wild stands are where it grows.
    from oikos_cal import m1

    assert "wild_wheats" in m1.PACKS
    emmer, einkorn = problem.wheats
    assert problem.domestication.stands is None
    asikli = list(emmer.sites).index("Asikli Hoyuk")
    assert emmer.gather[asikli].size == 0 and emmer.near[asikli].size == 7
    nevali = list(einkorn.sites).index("Nevalı Çori")
    assert np.array_equal(einkorn.gather[nevali], einkorn.near[nevali])


def test_m1_worlds_wear_thin_soils_down_first():
    # Erosion reads each cell's depth to bedrock (#24): M1's worlds carry the pack.
    from oikos_cal import m1

    assert "bedrock" in m1.PACKS


def test_m1_worlds_carry_decadal_climate_anomalies():
    # The 8.2 ka event and the Holocene's other abrupt anomalies (#41).
    from oikos_cal import m1

    assert "climate_anomaly" in m1.PACKS


def test_a_problem_can_be_judged_on_the_finer_grid():
    # F8 (#40): the same record on resolution-6 cells, each operator reading `rings` around
    # its sites (3 at resolution 6 cover about the area 1 does at resolution 5).
    pytest.importorskip("oikos", reason="build the bindings: maturin develop -m engine/crates/oikos-py/Cargo.toml")
    import h3

    from oikos_cal import m1

    try:
        paths = m1.packs("m1-swasia-r6")
        fine = m1.Problem(region="m1-swasia-r6", rings=3)
    except (raster.MissingInputs, FileNotFoundError) as e:
        pytest.skip(str(e))
    assert all("m1-swasia-r6." in p for p in paths)
    assert len(fine.cells) == 155_352 and h3.get_resolution(h3.int_to_str(int(fine.cells[0]))) == 6
    assert max(len(n) for n in fine.domestication.near) == 37
    assert fine.names[: len(fine.targets)] == [t.name for t in fine.targets]
    assert m1.river_reach(5) == m1.RIVER_RINGS and m1.river_reach(6) == 5
